mod ratchet;
mod constants;

use std::env;
use std::fs;
use std::io::{self, Write};
use ratchet::{compress_data, decompress_data};

fn main() -> io::Result<()> {
    let args: Vec<String> = env::args().collect();

    if args.len() != 4 {
        eprintln!("Usage:");
        eprintln!("  chaosregen compress <input_file> <output_file>");
        eprintln!("  chaosregen decompress <input_file> <output_file>");
        std::process::exit(1);
    }

    let mode = &args[1];
    let input_file = &args[2];
    let output_file = &args[3];

    let input_data = fs::read(input_file)?;

    let output_data = match mode.as_str() {
        "compress" => compress_data(&input_data),
        "decompress" => decompress_data(&input_data),
        _ => {
            eprintln!("Unknown mode: {}", mode);
            std::process::exit(1);
        }
    };

    let mut file = fs::File::create(output_file)?;
    file.write_all(&output_data)?;

    println!("Operation '{}' completed successfully.", mode);

    Ok(())
}