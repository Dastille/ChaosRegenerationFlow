mod constants;
mod ratchet;
#[cfg(feature = "gf")]
mod gf;

use ratchet::{compress_data, decompress_data, repair_data, verify_data};
use std::env;
use std::fs;
use std::io::{self, Write};

fn main() -> io::Result<()> {
    let args: Vec<String> = env::args().collect();

    if args.len() < 3 {
        eprintln!("Usage:");
        eprintln!("  chaosregen compress <input_file> <output_file>");
        eprintln!("  chaosregen decompress <input_file> <output_file>");
        eprintln!("  chaosregen verify <input_file>");
        eprintln!("  chaosregen repair <input_file> <output_file>");
        std::process::exit(1);
    }

    let mode = &args[1];
    let input_file = &args[2];
    let output_file = args.get(3);

    let input_data = fs::read(input_file)?;

    match mode.as_str() {
        "compress" => {
            if let Some(out) = output_file {
                let output_data = compress_data(&input_data);
                let mut file = fs::File::create(out)?;
                file.write_all(&output_data)?;
            } else {
                eprintln!("Output file required for compress mode");
                std::process::exit(1);
            }
        }
        "decompress" => {
            if let Some(out) = output_file {
                let output_data = decompress_data(&input_data);
                let mut file = fs::File::create(out)?;
                file.write_all(&output_data)?;
            } else {
                eprintln!("Output file required for decompress mode");
                std::process::exit(1);
            }
        }
        "verify" => {
            match verify_data(&input_data) {
                Ok(()) => println!("Verification succeeded."),
                Err(e) => {
                    eprintln!("Verification failed: {}", e);
                    std::process::exit(1);
                }
            }
            return Ok(());
        }
        "repair" => {
            if let Some(out) = output_file {
                let (output_data, valid) = repair_data(&input_data);
                if !valid {
                    eprintln!(
                        "Warning: file appears corrupted or truncated; output may be incomplete."
                    );
                }
                let mut file = fs::File::create(out)?;
                file.write_all(&output_data)?;
            } else {
                eprintln!("Output file required for repair mode");
                std::process::exit(1);
            }
        }
        _ => {
            eprintln!("Unknown mode: {}", mode);
            std::process::exit(1);
        }
    }

    println!("Operation '{}' completed successfully.", mode);

    Ok(())
}
