use crate::constants::{generate_mask, PI_SEED, PHI_SEED, LOGISTIC_SEED};

/// Compress data using ChaosRegen method
pub fn compress_data(input: &[u8]) -> Vec<u8> {
    let size = input.len();
    let pi_mask = generate_mask(PI_SEED, size);
    let phi_mask = generate_mask(PHI_SEED, size);
    let logistic_mask = generate_mask(LOGISTIC_SEED, size);

    let mut output = input.to_vec();

    // Phase 1: XOR with π mask + drag
    output = output.iter()
        .zip(pi_mask.iter())
        .map(|(&b, &m)| (b ^ m) >> 2)
        .collect();

    // Phase 2: Modular map (multiply by prime mod 257)
    output = output.iter()
        .map(|&b| ((b as u16 * 251) % 257) as u8)
        .collect();

    // Phase 3: XOR with φ mask + drag
    output = output.iter()
        .zip(phi_mask.iter())
        .map(|(&b, &m)| (b ^ m) >> 1)
        .collect();

    // Phase 4: Matrix shuffle (swap every pair if even length)
    if output.len() % 2 == 0 {
        output = output.chunks_exact(2)
            .flat_map(|chunk| vec![chunk[1], chunk[0]])
            .collect();
    }

    // Phase 5: XOR with logistic mask + drag
    output = output.iter()
        .zip(logistic_mask.iter())
        .map(|(&b, &m)| (b ^ m) >> 3)
        .collect();

    output
}

/// Decompress data back to original using ChaosRegen method
pub fn decompress_data(input: &[u8]) -> Vec<u8> {
    let size = input.len();
    let pi_mask = generate_mask(PI_SEED, size);
    let phi_mask = generate_mask(PHI_SEED, size);
    let logistic_mask = generate_mask(LOGISTIC_SEED, size);

    let mut output = input.to_vec();

    // Reverse Phase 5: XOR with logistic mask (reverse drag)
    output = output.iter()
        .zip(logistic_mask.iter())
        .map(|(&b, &m)| ((b << 3) ^ m))
        .collect();

    // Reverse Phase 4: Matrix unshuffle (swap back)
    if output.len() % 2 == 0 {
        output = output.chunks_exact(2)
            .flat_map(|chunk| vec![chunk[1], chunk[0]])
            .collect();
    }

    // Reverse Phase 3: XOR with φ mask (reverse drag)
    output = output.iter()
        .zip(phi_mask.iter())
        .map(|(&b, &m)| ((b << 1) ^ m))
        .collect();

    // Reverse Phase 2: Modular map inverse
    output = output.iter()
        .map(|&b| modular_inverse_map(b, 251, 257))
        .collect();

    // Reverse Phase 1: XOR with π mask (reverse drag)
    output = output.iter()
        .zip(pi_mask.iter())
        .map(|(&b, &m)| ((b << 2) ^ m))
        .collect();

    output
}

/// Helper: Modular inverse for prime modulo
fn modular_inverse_map(value: u8, prime: u16, modulo: u16) -> u8 {
    for candidate in 0..=255u16 {
        if (candidate * prime) % modulo == value as u16 {
            return candidate as u8;
        }
    }
    0 // Fallback if no inverse found (should not happen with good primes)
}