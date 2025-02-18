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
use log::{info, error};
use clap::Parser;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Receiver address (user@IP, e.g., user@192.168.1.100)
    #[arg(short, long)]
    receiver: String,

    /// File path to transfer
    #[arg(short, long)]
    file: String,

    /// Chunk size in MB (default: auto based on file size)
    #[arg(short, long, default_value_t = 0)]
    chunk_size_mb: usize,
}

#[tokio::main]
async fn main() -> io::Result<()> {
    env_logger::init();
    let args = Args::parse();

    let sender_user = std::env::var("USER").unwrap_or("user".to_string());
    let sender_ip = String::from_utf8_lossy(&Command::new("hostname").arg("-I").output()?.stdout)
        .split_whitespace().next().unwrap_or("127.0.0.1").to_string();
    info!("Sender: {}@{}", sender_user, sender_ip);

    let receiver = args.receiver;
    let receiver_parts: Vec<&str> = receiver.split('@').collect();
    if receiver_parts.len() != 2 {
        error!("Invalid receiver format. Use user@IP (e.g., user@192.168.1.100)");
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "Invalid receiver format"));
    }
    let receiver_user = receiver_parts[0];
    let receiver_ip = receiver_parts[1];
    let receiver_dir = format!("/home/{}/chaosregenflow", receiver_user);

    let file_path = args.file;
    if !Path::new(&file_path).exists() {
        error!("File not found: {}", file_path);
        return Err(io::Error::new(io::ErrorKind::NotFound, "File not found"));
    }

    let ssh_cmd = format!("ssh -C {}", receiver);
    let scp_cmd = format!("scp -C");
    setup_environment(&ssh_cmd, &scp_cmd, &sender_user, &sender_ip, &receiver_user, &receiver_ip, &receiver_dir)?;

    let file_size = fs::metadata(&file_path)?.len() as usize;
    let chunk_size = if args.chunk_size_mb > 0 {
        args.chunk_size_mb * 1024 * 1024
    } else {
        if file_size < 100 * 1024 * 1024 { 1 * 1024 * 1024 } else { 4 * 1024 * 1024 }
    };
    let chunks = (file_size + chunk_size - 1) / chunk_size;
    info!("File: {}, Size: {} bytes, Chunks: {} ({}MB each)", file_path, file_size, chunks, chunk_size / 1024 / 1024);

    let tcp = match std::net::TcpStream::connect(format!("{}:22", receiver_ip)) {
        Ok(tcp) => tcp,
        Err(e) => {
            error!("Failed to connect to {}: {}", receiver_ip, e);
            return Err(e);
        }
    };
    let mut sess = Session::new()?;
    sess.set_tcp_stream(tcp);
    sess.handshake()?;
    if let Err(e) = sess.userauth_agent(receiver_user) {
        error!("SSH authentication failed: {}", e);
        return Err(io::Error::new(io::ErrorKind::PermissionDenied, e));
    }

    let device = Device::get_device(0).ok();
    let ctx = device.map(|d| Context::new(d)).transpose()?;
    let module = ctx.as_ref().map(|c| Module::from_ptx_file("chaos_kernel.ptx")).transpose()?;
    let stream = ctx.as_ref().map(|_| Stream::new()).transpose()?;

    info!("CRF Ad-Hoc Mode: Sending {}...", file_path);
    let mut file = BufReader::new(File::open(&file_path)?);
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
        let compressed = match encode_all(&residuals[..], 3) {
            Ok(data) => data,
            Err(e) => {
                error!("Failed to compress residuals for chunk {}: {}", i, e);
                return Err(e);
            }
        };

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
    info!("Ad-Hoc Sent: {} bytes (~{}MB)", total_sent, total_sent / 1024 / 1024);
    info!("Time: {:.3}s", time);
    info!("Ratio: {}:1", file_size / total_sent);

    info!("CRF Pre-Known List Setup...");
    let mut registry = Vec::new();
    let mut file = BufReader::new(File::open(&file_path)?);
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
    let compressed_registry = encode_all(®istry[..], 3)?;
    let mut channel = sess.channel_session()?;
    channel.exec("cat > chaosregenflow/registry.zst")?;
    channel.write_all(&compressed_registry)?;
    channel.close()?;
    total_sent += compressed_registry.len();
    info!("List Setup Sent: {} bytes (~{}MB)", total_sent, total_sent / 1024 / 1024);

    info!("CRF Pre-Known Access...");
    let mut hasher = Xxh64::new(0);
    let mut file = BufReader::new(File::open(&file_path)?);
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;
    hasher.update(&buffer);
    let file_id = has
