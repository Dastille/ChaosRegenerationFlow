use std::fs::{self, File};
use std::io::{self, BufReader, BufWriter, Read};
use std::path::Path;
use ssh2::Session;
use zstd::stream::decode_all;
use cuda::{Context, Device, Module, Stream};
use log::{info, error};

fn main() -> io::Result<()> {
    env_logger::init();
    let receiver_dir = format!("/home/{}/chaosregenflow", std::env::var("USER").unwrap_or("user".to_string()));
    fs::create_dir_all(&receiver_dir)?;
    std::env::set_current_dir(&receiver_dir)?;

    let device = Device::get_device(0).ok();
    let ctx = device.map(|d| Context::new(d)).transpose()?;
    let module = ctx.as_ref().map(|c| Module::from_ptx_file("chaos_kernel.ptx")).transpose()?;
    let stream = ctx.as_ref().map(|_| Stream::new()).transpose()?;

    info!("Receiver running. Waiting for files in {}...", receiver_dir);

    loop {
        for id_file in fs::read_dir(".")?.filter_map(|e| e.ok()).filter(|e| e.file_name().to_string_lossy().starts_with("id_chunk_")) {
            let id_path = id_file.path();
            let mut id = vec![0u8; 24];
            let mut id_reader = BufReader::new(File::open(&id_path)?);
            if let Err(e) = id_reader.read_exact(&mut id) {
                error!("Failed to read ID file {}: {}", id_path.display(), e);
                continue;
            }
            let seed = &id[0..6];
            let tweak = &id[6..8];
            let chunk_name = id_path.file_name().unwrap().to_string_lossy().replace("id_chunk_", "");
            let residual_path = Path::new(&format!("residual_chunk_{}.zst", chunk_name));

            if residual_path.exists() {
                let mut residual_file = BufReader::new(File::open(residual_path)?);
                let mut compressed = Vec::new();
                if let Err(e) = residual_file.read_to_end(&mut compressed) {
                    error!("Failed to read residuals {}: {}", residual_path.display(), e);
                    continue;
                }
                let residuals = match decode_all(&compressed[..]) {
                    Ok(data) => data,
                    Err(e) => {
                        error!("Failed to decompress residuals {}: {}", chunk_name, e);
                        continue;
                    }
                };

                let mut gen = vec![0u8; residuals.len()];
                if let (Some(ref module), Some(ref stream)) = (&module, &stream) {
                    let seed_val = u64::from_le_bytes([seed[0], seed[1], seed[2], seed[3], seed[4], seed[5], 0, 0]);
                    let tweak_val = u16::from_le_bytes([tweak[0], tweak[1]]);
                    let kernel = module.get_function("chaos_gen")?;
                    let block_size = 256;
                    let grid_size = (residuals.len() as u32 + block_size - 1) / block_size;
                    unsafe {
                        kernel.launch(
                            &[&seed_val, &tweak_val, &residuals.len(), gen.as_mut_ptr()],
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

                let chunk: Vec<u8> = gen.iter().zip(residuals.iter()).map(|(&a, &b)| a ^ b).collect();
                fs::write(format!("chunk_{}", chunk_name), &chunk)?;
                fs::remove_file(id_path)?;
                fs::remove_file(residual
