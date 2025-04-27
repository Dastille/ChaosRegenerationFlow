use rand::{Rng, SeedableRng};
use rand::rngs::StdRng;

/// Generate a pseudo-random mask based on a seed, simulating universal constants (π, φ, logistic map)
/// Ensures the same input seed always gives the same mask.
pub fn generate_mask(seed: u64, size: usize) -> Vec<u8> {
    let mut rng = StdRng::seed_from_u64(seed);
    (0..size).map(|_| rng.gen::<u8>()).collect()
}

/// Seeds representing our "universal constants"
pub const PI_SEED: u64 = 3141592653;
pub const PHI_SEED: u64 = 1618033988;
pub const LOGISTIC_SEED: u64 = 2718281828;