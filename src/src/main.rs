use std::fs::{self, File};
use std::io::{self, BufReader, BufWriter, Read, Write};
use std::path::Path;
use std::process::{Command, Stdio};
use ssh2::Session;
use xxhash_rust::xxh64::Xxh64;
use zstd::stream::encode_all;
use rand::Rng;
use cuda::{Context, Device, Module, Stream};
use tokio;

#[tokio::main]
async fn main() -> io::Result<()> {
    let sender_user = std::env::var("USER").unwrap_or("user".to_string());
    let sender_ip = String::from_utf8_lossy(&Command::new("hostname").arg("-I").output()?.stdout)
        .split_whitespace().next().unwrap_or("127.0.0.1").to_string();
    println!("Sender: {}@{}", sender_user, sender_ip);

    println!("Enter receiver (user@IP, e.g., user@192.168.1.100):");
    let mut receiver = String::new();
    io::stdin().read_line(&mut receiver)?;
    let receiver = receiver.trim();
    let receiver_parts: Vec<&str> = receiver.split('@').collect();
    let receiver_user = receiver_parts[0];
    let receiver_ip = receiver_parts[1];
    let receiver_dir = format!("/home/{}/chaosregenflow", receiver_user);

    let ssh_cmd = format!("ssh -C {}", receiver);
    let scp_cmd = format!("scp -C");
    setup_environment(&ssh_cmd, &scp_cmd, &sender_user, &sender_ip, &receiver_user, &receiver_ip, &receiver_dir)?;

    println!("Enter file path (or Enter for 10MB test):");
    let mut file_path = String::new();
    io::stdin().read_line(&mut file_path)?;
    let file_path = file_path.trim();
    let file_path = if file_path.is_empty() {
        let path = "test_file.bin";
        Command::new("dd")
            .args(["if=/dev/urandom", "of=test_file.bin", "bs=1M", "count=10"])
            .status()?;
        path
    } else {
        file_path
    };
    if !Path::new(file_path).exists() {
        println!("File not found!");
        return Ok(());
    }

    let file_size = fs::metadata(file_path)?.len() as usize;
    let chunk_size = if file_size < 100 * 1024 * 1024 { 1 * 1024 * 1024 } else { 4 * 1024 * 1024 };
    let chunks = (file_size + chunk_size - 1) / chunk_size;
    println!("File: {}, Size: {} bytes, Chunks: {} ({}MB each)", file_path, file_size, chunks, chunk_size / 1024 / 1024);

    let tcp = std::net::TcpStream::connect(format!("{}:22", receiver_ip))?;
    let mut sess = Session::new()?;
    sess.set_tcp_stream(tcp);
    sess.handshake()?;
    sess.userauth_agent(receiver_user)?;

    let device = Device::get_device(0).ok();
    let ctx = device.map(|d| Context::new(d)).transpose()?;
    let module = ctx.as_ref().map(|c| Module::from_ptx_file("chaos_kernel.ptx")).transpose()?;
    let stream = ctx.as_ref().map(|_| Stream::new()).transpose()?;

    println!("CRF Ad-Hoc Mode: Sending {}...", file_path);
    let mut file = BufReader::new(File::open(file_path)?);
    let mut total_sent = 0;
    let start_time = std::time::Instant::now();
    for i in 0..chunks {
        let mut chunk = vec![0u8; chunk_size.min(file_size - i * chunk_size)];
        file.read_exact(&mut chunk)?;

        let mut hasher = Xxh64::new(0);
        hasher.update(&chunk);
        let seed = hasher.digest().to_le_bytes()[0..6].to_vec();
        let tweak = rand::thread_rng().gen::<u16>().to_le_bytes().to_vec();
        let id = [seed.clone(), tweak.clone()].concat();

        let mut gen = vec![0u8; chunk.len()];
        if let (Some(ref module), Some(ref stream)) = (&module, &stream) {
            let seed_val = u64::from_le_bytes([seed[0], seed[1], seed[2], seed[3], seed[4], seed[5], 0, 0]);
            let tweak_val = u16::from_le_bytes(tweak);
            let kernel = module.get_function("chaos_gen")?;
            let block_size = 256;
            let grid_size = (chunk.len() as u32 + block_size - 1) / block_size;
            unsafe {
                kernel.launch(
                    &[&seed_val, &tweak_val, &chunk.len(), gen.as_mut_ptr()],
                    grid_size,
                    block_size,
                    0,
                    stream,
                )?;
            }
            stream.synchronize()?;
        } else {
            let mut seed_val = u64::from_le_bytes([seed[0], seed[1], seed[2], seed[3], seed[4], seed[5], 0, 0]);
            for byte in gen.iter_mut() {
                seed_val = (seed_val.wrapping_mul(314159) + 299792458) % 256;
                *byte = seed_val as u8;
            }
        }

        let residuals: Vec<u8> = chunk.iter().zip(gen.iter()).map(|(&a, &b)| a ^ b).collect();
        let compressed = encode_all(&residuals[..], 3)?;

        let mut channel = sess.channel_session()?;
        channel.exec(&format!("cat > chaosregenflow/id_chunk_{}", i))?;
        channel.write_all(&id)?;
        channel.close()?;
        let mut channel = sess.channel_session()?;
        channel.exec(&format!("cat > chaosregenflow/residual_chunk_{}.zst", i))?;
        channel.write_all(&compressed)?;
        channel.close()?;
        total_sent += id.len() + compressed.len();
    }
    let time = start_time.elapsed().as_secs_f64();
    println!("Ad-Hoc Sent: {} bytes (~{}MB)", total_sent, total_sent / 1024 / 1024);
    println!("Time: {:.3}s", time);
    println!("Ratio: {}:1", file_size / total_sent);

    println!("CRF Pre-Known List Setup...");
    let mut registry = Vec::new();
    let mut file = BufReader::new(File::open(file_path)?);
    for _ in 0..chunks {
        let mut chunk = vec![0u8; chunk_size.min(file_size - registry.len())];
        file.read_exact(&mut chunk)?;
        let mut hasher = Xxh64::new(0);
        hasher.update(&chunk);
        let seed = hasher.digest().to_le_bytes()[0..6].to_vec();
        let tweak = rand::thread_rng().gen::<u16>().to_le_bytes().to_vec();
        let size = file_size.to_le_bytes().to_vec();
        registry.extend_from_slice(&[seed, tweak, size].concat());
    }
    let compressed_registry = encode_all(&registry[..], 3)?;
    let mut channel = sess.channel_session()?;
    channel.exec("cat > chaosregenflow/registry.zst")?;
    channel.write_all(&compressed_registry)?;
    channel.close()?;
    total_sent += compressed_registry.len();
    println!("List Setup Sent: {} bytes (~{}MB)", total_sent, total_sent / 1024 / 1024);

    println!("CRF Pre-Known Access...");
    let mut hasher = Xxh64::new(0);
    let mut file = BufReader::new(File::open(file_path)?);
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;
    hasher.update(&buffer);
    let file_id = hasher.digest().to_le_bytes()[0..6].to_vec();
    let tweak = rand::thread_rng().gen::<u16>().to_le_bytes().to_vec();
    let id = [file_id, tweak].concat();
    let mut channel = sess.channel_session()?;
    channel.exec("cat > chaosregenflow/file_id.bin")?;
    channel.write_all(&id)?;
    channel.close()?;
    println!("Sent: 8 bytes for {}", file_path);

    let mut channel = sess.channel_session()?;
    channel.exec(&format!("cd {} && cargo run --release", receiver_dir))?;
    let mut output = String::new();
    channel.read_to_string(&mut output)?;
    println!("Receiver output: {}", output);

    Ok(())
}

fn setup_environment(ssh_cmd: &str, scp_cmd: &str, sender_user: &str, sender_ip: &str, receiver_user: &str, receiver_ip: &str, receiver_dir: &str) -> io::Result<()> {
    println!("Setting up sender environment...");
    if !Command::new("rustc").arg("--version").status()?.success() {
        println!("Installing Rust on sender...");
        Command::new("curl")
            .args(["--proto", "=https", "--tlsv1.2", "-sSf", "https://sh.rustup.rs"])
            .stdout(Stdio::piped())
            .spawn()?
            .stdout
            .unwrap()
            .pipe_to(&mut Command::new("sh").arg("-s").arg("--").arg("-y").stdin(Stdio::piped()).spawn()?)?;
    }
    Command::new("cargo").args(["init", "--bin", "chaosregenflow"]).status()?;
    fs::write("chaosregenflow/Cargo.toml", include_str!("../Cargo.toml"))?;
    fs::write("chaosregenflow/src/main.rs", include_str!("main.rs"))?;
    fs::write("chaosregenflow/src/receiver.rs", include_str!("receiver.rs"))?;
    fs::write("chaosregenflow/chaos_kernel.cu", include_str!("chaos_kernel.cu"))?;
    Command::new("nvcc").args(["-ptx", "chaos_kernel.cu", "-o", "chaos_kernel.ptx"]).current_dir("chaosregenflow").status()?;
    Command::new("cargo").args(["build", "--release"]).current_dir("chaosregenflow").status()?;

    println!("Setting up receiver environment...");
    let mut channel = Session::new()?;
    let tcp = std::net::TcpStream::connect(format!("{}:22", receiver_ip))?;
    channel.set_tcp_stream(tcp);
    channel.handshake()?;
    channel.userauth_agent(receiver_user)?;

    let mut cmd = channel.channel_session()?;
    cmd.exec("command -v rustc || curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y")?;
    cmd.close()?;
    let mut cmd = channel.channel_session()?;
    cmd.exec(&format!("mkdir -p {} && cd {} && cargo init --bin", receiver_dir, receiver_dir))?;
    cmd.close()?;
    let mut cmd = channel.channel_session()?;
    cmd.exec(&format!("echo '{}' > {}/Cargo.toml", include_str!("../Cargo.toml"), receiver_dir))?;
    cmd.close()?;
    let mut cmd = channel.channel_session()?;
    cmd.exec(&format!("echo '{}' > {}/src/main.rs", include_str!("receiver.rs"), receiver_dir))?;
    cmd.close()?;
    let mut cmd = channel.channel_session()?;
    cmd.exec(&format!("echo '{}' > {}/chaos_kernel.cu", include_str!("chaos_kernel.cu"), receiver_dir))?;
    cmd.close()?;
    let mut cmd = channel.channel_session()?;
    cmd.exec(&format!("cd {} && nvcc -ptx chaos_kernel.cu -o chaos_kernel.ptx && cargo build --release", receiver_dir))?;
    cmd.close()?;

    Ok(())
}

trait Pipe {
    fn pipe_to(&mut self, other: &mut Command) -> io::Result<()>;
}

impl Pipe for std::process::ChildStdout {
    fn pipe_to(&mut self, other: &mut Command) -> io::Result<()> {
        let mut stdin = other.stdin.take().unwrap();
        std::io::copy(self, &mut stdin)?;
        Ok(())
    }
}
