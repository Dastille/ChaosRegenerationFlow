use crate::constants::{generate_mask, LOGISTIC_SEED, PHI_SEED, PI_SEED};
use crc32fast::Hasher;
use std::convert::TryInto;

const MAGIC: &[u8; 4] = b"CRGN";

pub fn compress_data(input: &[u8]) -> Vec<u8> {
    let size = input.len();
    let pi_mask = generate_mask(PI_SEED, size);
    let phi_mask = generate_mask(PHI_SEED, size);
    let logistic_mask = generate_mask(LOGISTIC_SEED, size);

    let mut output = input.to_vec();

    // XOR with the π mask
    output = output
        .iter()
        .zip(pi_mask.iter())
        .map(|(&b, &m)| b ^ m)
        .collect();

    output = output
        .iter()
        .map(|&b| ((b as u16 * 251) % 257) as u8)
        .collect();

    // XOR with the φ mask
    output = output
        .iter()
        .zip(phi_mask.iter())
        .map(|(&b, &m)| b ^ m)
        .collect();

    if output.len() % 2 == 0 {
        output = output
            .chunks_exact(2)
            .flat_map(|chunk| vec![chunk[1], chunk[0]])
            .collect();
    }

    // XOR with the logistic mask
    output = output
        .iter()
        .zip(logistic_mask.iter())
        .map(|(&b, &m)| b ^ m)
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

pub fn verify_data(input: &[u8]) -> Result<(), String> {
    if input.len() < 16 {
        return Err("Input too small to be valid ChaosRegen compressed file.".into());
    }

    let magic = &input[..4];
    let original_size_bytes = &input[4..12];
    let checksum_bytes = &input[12..16];
    let compressed_data = &input[16..];

    if magic != MAGIC {
        return Err("Invalid ChaosRegen magic header".into());
    }

    let original_size = u64::from_le_bytes(original_size_bytes.try_into().unwrap());
    let expected_crc32 = u32::from_le_bytes(checksum_bytes.try_into().unwrap());

    if compressed_data.len() != original_size as usize {
        return Err("Size encoded in header does not match data length".into());
    }

    let mut hasher = Hasher::new();
    hasher.update(compressed_data);
    let actual_crc32 = hasher.finalize();

    if expected_crc32 != actual_crc32 {
        return Err("CRC32 checksum mismatch".into());
    }

    Ok(())
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

    // Reverse XOR with the logistic mask
    output = output
        .iter()
        .zip(logistic_mask.iter())
        .map(|(&b, &m)| b ^ m)
        .collect();

    if output.len() % 2 == 0 {
        output = output
            .chunks_exact(2)
            .flat_map(|chunk| vec![chunk[1], chunk[0]])
            .collect();
    }

    // Reverse XOR with the φ mask
    output = output
        .iter()
        .zip(phi_mask.iter())
        .map(|(&b, &m)| b ^ m)
        .collect();

    output = output
        .iter()
        .map(|&b| modular_inverse_map(b, 251, 257))
        .collect();

    // Reverse XOR with the π mask
    output = output
        .iter()
        .zip(pi_mask.iter())
        .map(|(&b, &m)| b ^ m)
        .collect();

    output
}

/// Attempt to recover the original data even if the `.sg1` file is
/// truncated or fails CRC checks. Missing bytes are padded with zeroes
/// before the chaotic reverse transforms are applied.
pub fn repair_data(input: &[u8]) -> (Vec<u8>, bool) {
    if input.len() < 16 {
        panic!("Input too small to be valid ChaosRegen compressed file.");
    }

    let magic = &input[..4];
    if magic != MAGIC {
        panic!("Invalid ChaosRegen magic header. File may be corrupted or wrong format.");
    }

    let original_size = u64::from_le_bytes(input[4..12].try_into().unwrap()) as usize;
    let checksum_bytes = &input[12..16];
    let compressed_data = if input.len() > 16 { &input[16..] } else { &[] };

    let expected_crc32 = u32::from_le_bytes(checksum_bytes.try_into().unwrap());
    let mut hasher = Hasher::new();
    hasher.update(compressed_data);
    let actual_crc32 = hasher.finalize();
    let crc_ok = expected_crc32 == actual_crc32 && compressed_data.len() == original_size;

    // Pad missing bytes with zeros
    let mut padded = vec![0u8; original_size];
    let copy_len = compressed_data.len().min(original_size);
    padded[..copy_len].copy_from_slice(&compressed_data[..copy_len]);

    let pi_mask = generate_mask(PI_SEED, original_size);
    let phi_mask = generate_mask(PHI_SEED, original_size);
    let logistic_mask = generate_mask(LOGISTIC_SEED, original_size);

    let mut output = padded;

    // Reverse XOR with the logistic mask
    output = output
        .iter()
        .zip(logistic_mask.iter())
        .map(|(&b, &m)| b ^ m)
        .collect();

    if output.len() % 2 == 0 {
        output = output
            .chunks_exact(2)
            .flat_map(|chunk| vec![chunk[1], chunk[0]])
            .collect();
    }

    // Reverse XOR with the φ mask
    output = output
        .iter()
        .zip(phi_mask.iter())
        .map(|(&b, &m)| b ^ m)
        .collect();

    output = output
        .iter()
        .map(|&b| modular_inverse_map(b, 251, 257))
        .collect();

    // Reverse XOR with the π mask
    output = output
        .iter()
        .zip(pi_mask.iter())
        .map(|(&b, &m)| b ^ m)
        .collect();

    (output, crc_ok)
}

fn modular_inverse_map(value: u8, prime: u16, modulo: u16) -> u8 {
    for candidate in 0..=255u16 {
        if (candidate * prime) % modulo == value as u16 {
            return candidate as u8;
        }
    }
    0
}
