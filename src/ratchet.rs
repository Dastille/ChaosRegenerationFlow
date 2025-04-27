use crate::constants::{generate_mask, PI_SEED, PHI_SEED, LOGISTIC_SEED};
use crc32fast::Hasher;
use std::convert::TryInto;

const MAGIC: &[u8; 4] = b"CRGN";

pub fn compress_data(input: &[u8]) -> Vec<u8> {
    let size = input.len();
    let pi_mask = generate_mask(PI_SEED, size);
    let phi_mask = generate_mask(PHI_SEED, size);
    let logistic_mask = generate_mask(LOGISTIC_SEED, size);

    let mut output = input.to_vec();

    output = output.iter()
        .zip(pi_mask.iter())
        .map(|(&b, &m)| (b ^ m) >> 2)
        .collect();

    output = output.iter()
        .map(|&b| ((b as u16 * 251) % 257) as u8)
        .collect();

    output = output.iter()
        .zip(phi_mask.iter())
        .map(|(&b, &m)| (b ^ m) >> 1)
        .collect();

    if output.len() % 2 == 0 {
        output = output.chunks_exact(2)
            .flat_map(|chunk| vec![chunk[1], chunk[0]])
            .collect();
    }

    output = output.iter()
        .zip(logistic_mask.iter())
        .map(|(&b, &m)| (b ^ m) >> 3)
        .collect();

    let mut hasher = Hasher::new();
    hasher.update(&output);
    let crc32 = hasher.finalize();

    let mut final_output = Vec::new();
    final_output.extend_from_slice(MAGIC);
    final_output.extend_from_slice(&(size as u64).to_le_bytes());
    final_output.extend_from_slice(&crc32.to_le_bytes());
    final_output.extend_from_slice(&output);

    final_output
}

pub fn decompress_data(input: &[u8]) -> Vec<u8> {
    if input.len() < 16 {
        panic!("Input too small to be valid ChaosRegen compressed file.");
    }

    let magic = &input[..4];
    let original_size_bytes = &input[4..12];
    let checksum_bytes = &input[12..16];
    let compressed_data = &input[16..];

    if magic != MAGIC {
        panic!("Invalid ChaosRegen magic header. File may be corrupted or wrong format.");
    }

    let original_size = u64::from_le_bytes(original_size_bytes.try_into().unwrap());
    let expected_crc32 = u32::from_le_bytes(checksum_bytes.try_into().unwrap());

    let mut hasher = Hasher::new();
    hasher.update(compressed_data);
    let actual_crc32 = hasher.finalize();

    if expected_crc32 != actual_crc32 {
        panic!("CRC32 checksum mismatch! Compressed file is corrupted.");
    }

    let size = original_size as usize;
    let pi_mask = generate_mask(PI_SEED, size);
    let phi_mask = generate_mask(PHI_SEED, size);
    let logistic_mask = generate_mask(LOGISTIC_SEED, size);

    let mut output = compressed_data.to_vec();

    output = output.iter()
        .zip(logistic_mask.iter())
        .map(|(&b, &m)| ((b << 3) ^ m))
        .collect();

    if output.len() % 2 == 0 {
        output = output.chunks_exact(2)
            .flat_map(|chunk| vec![chunk[1], chunk[0]])
            .collect();
    }

    output = output.iter()
        .zip(phi_mask.iter())
        .map(|(&b, &m)| ((b << 1) ^ m))
        .collect();

    output = output.iter()
        .map(|&b| modular_inverse_map(b, 251, 257))
        .collect();

    output = output.iter()
        .zip(pi_mask.iter())
        .map(|(&b, &m)| ((b << 2) ^ m))
        .collect();

    output
}

fn modular_inverse_map(value: u8, prime: u16, modulo: u16) -> u8 {
    for candidate in 0..=255u16 {
        if (candidate * prime) % modulo == value as u16 {
            return candidate as u8;
        }
    }
    0
}