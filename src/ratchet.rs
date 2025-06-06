use crate::constants::{
    generate_mask, logistic_mask, LOGISTIC_R, LOGISTIC_SEED, PHI_SEED, PI_SEED,
};
use crc32fast::Hasher;
use std::convert::TryInto;

const CHUNK_SIZE: usize = 256 * 1024; // 256KB blocks
const MICROZONE_SIZE: usize = 1024; // 1KB microzones
const MAX_PASSES: usize = 3;
const ENTROPY_THRESHOLD: f64 = 0.1; // bits per byte
const PRIMES: [u16; 3] = [251, 241, 239];

/// Magic header used in `.sg1` files.
///
/// `SGIL` is short for "Sigil", superseding the previous `CRGN` header.
const MAGIC: &[u8; 4] = b"SGIL";

pub fn compress_data(input: &[u8]) -> Vec<u8> {
    let size = input.len();
    let pi_mask = generate_mask(PI_SEED, size);
    let phi_mask = generate_mask(PHI_SEED, size);
    let logistic_mask = logistic_mask(LOGISTIC_SEED, size, LOGISTIC_R);

    let mut output = input.to_vec();

    let mut offset = 0;
    for chunk in output.chunks_mut(CHUNK_SIZE) {
        let mut zone_offset = 0;
        for micro in chunk.chunks_mut(MICROZONE_SIZE) {
            let global = offset + zone_offset;
            let pi_slice = &pi_mask[global..global + micro.len()];
            let phi_slice = &phi_mask[global..global + micro.len()];
            let logistic_slice = &logistic_mask[global..global + micro.len()];

            let mut passes = 0;
            let mut used = [false; PRIMES.len()];
            let mut prev_entropy = shannon_entropy(micro);
            while passes < MAX_PASSES {
                if let Some((idx, next_data, new_entropy)) = select_best_prime(
                    micro,
                    pi_slice,
                    phi_slice,
                    logistic_slice,
                    &used,
                    prev_entropy,
                ) {
                    micro.copy_from_slice(&next_data);
                    used[idx] = true;
                    prev_entropy = new_entropy;
                    passes += 1;
                } else {
                    break;
                }
            }

            zone_offset += micro.len();
        }
        offset += chunk.len();
    }

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
    let logistic_mask = logistic_mask(LOGISTIC_SEED, size, LOGISTIC_R);

    let mut output = compressed_data.to_vec();

    let mut offset = 0;
    for chunk in output.chunks_mut(CHUNK_SIZE) {
        let mut zone_offset = 0;
        for micro in chunk.chunks_mut(MICROZONE_SIZE) {
            let global = offset + zone_offset;
            let pi_slice = &pi_mask[global..global + micro.len()];
            let phi_slice = &phi_mask[global..global + micro.len()];
            let logistic_slice = &logistic_mask[global..global + micro.len()];

            let mut used = [false; PRIMES.len()];
            let mut passes = 0;
            let mut prev_entropy = shannon_entropy(micro);
            while passes < MAX_PASSES {
                if let Some((idx, next_data)) = select_inverse_prime(
                    micro,
                    pi_slice,
                    phi_slice,
                    logistic_slice,
                    &used,
                    prev_entropy,
                ) {
                    micro.copy_from_slice(&next_data);
                    used[idx] = true;
                    prev_entropy = shannon_entropy(micro);
                    passes += 1;
                } else {
                    break;
                }
            }

            zone_offset += micro.len();
        }
        offset += chunk.len();
    }

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
    let logistic_mask = logistic_mask(LOGISTIC_SEED, original_size, LOGISTIC_R);

    let mut output = padded;

    let mut offset = 0;
    for chunk in output.chunks_mut(CHUNK_SIZE) {
        let mut zone_offset = 0;
        for micro in chunk.chunks_mut(MICROZONE_SIZE) {
            let global = offset + zone_offset;
            let pi_slice = &pi_mask[global..global + micro.len()];
            let phi_slice = &phi_mask[global..global + micro.len()];
            let logistic_slice = &logistic_mask[global..global + micro.len()];

            let mut used = [false; PRIMES.len()];
            let mut passes = 0;
            let mut prev_entropy = shannon_entropy(micro);
            while passes < MAX_PASSES {
                if let Some((idx, next_data)) = select_inverse_prime(
                    micro,
                    pi_slice,
                    phi_slice,
                    logistic_slice,
                    &used,
                    prev_entropy,
                ) {
                    micro.copy_from_slice(&next_data);
                    used[idx] = true;
                    prev_entropy = shannon_entropy(micro);
                    passes += 1;
                } else {
                    break;
                }
            }

            zone_offset += micro.len();
        }
        offset += chunk.len();
    }

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

fn shannon_entropy(data: &[u8]) -> f64 {
    let mut counts = [0usize; 256];
    for &b in data {
        counts[b as usize] += 1;
    }
    let len = data.len() as f64;
    let mut entropy = 0.0;
    for &c in &counts {
        if c > 0 {
            let p = c as f64 / len;
            entropy -= p * p.log2();
        }
    }
    entropy
}

fn forward_pass(data: &mut [u8], pi: &[u8], phi: &[u8], logistic: &[u8], prime: u16) {
    for (b, &m) in data.iter_mut().zip(pi.iter()) {
        *b ^= m;
    }
    for b in data.iter_mut() {
        *b = ((*b as u16 * prime) % 257) as u8;
    }
    for (b, &m) in data.iter_mut().zip(phi.iter()) {
        *b ^= m;
    }
    if data.len() % 2 == 0 {
        for chunk in data.chunks_exact_mut(2) {
            chunk.swap(0, 1);
        }
    }
    for (b, &m) in data.iter_mut().zip(logistic.iter()) {
        *b ^= m;
    }
}

fn inverse_pass(data: &mut [u8], pi: &[u8], phi: &[u8], logistic: &[u8], prime: u16) {
    for (b, &m) in data.iter_mut().zip(logistic.iter()) {
        *b ^= m;
    }
    if data.len() % 2 == 0 {
        for chunk in data.chunks_exact_mut(2) {
            chunk.swap(0, 1);
        }
    }
    for (b, &m) in data.iter_mut().zip(phi.iter()) {
        *b ^= m;
    }
    for b in data.iter_mut() {
        *b = modular_inverse_map(*b, prime, 257);
    }
    for (b, &m) in data.iter_mut().zip(pi.iter()) {
        *b ^= m;
    }
}

fn entropy_score(before: f64, after: f64) -> f64 {
    before - after
}

fn select_best_prime(
    data: &[u8],
    pi: &[u8],
    phi: &[u8],
    logistic: &[u8],
    used: &[bool; PRIMES.len()],
    prev_entropy: f64,
) -> Option<(usize, Vec<u8>, f64)> {
    let mut best_score = 0.0;
    let mut best_idx = None;
    let mut best_data = Vec::new();
    for (i, &prime) in PRIMES.iter().enumerate() {
        if used[i] {
            continue;
        }
        let mut tmp = data.to_vec();
        forward_pass(&mut tmp, pi, phi, logistic, prime);
        let entropy = shannon_entropy(&tmp);
        let score = entropy_score(prev_entropy, entropy);
        if score > best_score {
            best_score = score;
            best_idx = Some(i);
            best_data = tmp;
        }
    }
    if let Some(idx) = best_idx {
        if best_score >= ENTROPY_THRESHOLD {
            let ent = shannon_entropy(&best_data);
            return Some((idx, best_data, ent));
        }
    }
    None
}

fn select_inverse_prime(
    data: &[u8],
    pi: &[u8],
    phi: &[u8],
    logistic: &[u8],
    used: &[bool; PRIMES.len()],
    prev_entropy: f64,
) -> Option<(usize, Vec<u8>)> {
    let mut best_score = 0.0;
    let mut best_idx = None;
    let mut best_data = Vec::new();
    for (i, &prime) in PRIMES.iter().enumerate() {
        if used[i] {
            continue;
        }
        let mut tmp = data.to_vec();
        inverse_pass(&mut tmp, pi, phi, logistic, prime);
        let entropy = shannon_entropy(&tmp);
        let score = entropy_score(entropy, prev_entropy);
        if score > best_score {
            best_score = score;
            best_idx = Some(i);
            best_data = tmp;
        }
    }
    if let Some(idx) = best_idx {
        if best_score >= ENTROPY_THRESHOLD {
            return Some((idx, best_data));
        }
    }
    None
}
