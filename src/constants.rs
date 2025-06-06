use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

pub fn generate_mask(seed: u64, size: usize) -> Vec<u8> {
    let mut rng = StdRng::seed_from_u64(seed);
    (0..size).map(|_| rng.gen::<u8>()).collect()
}

pub const PI_SEED: u64 = 3141592653;
pub const PHI_SEED: u64 = 1618033988;
pub const LOGISTIC_SEED: u64 = 2718281828;
