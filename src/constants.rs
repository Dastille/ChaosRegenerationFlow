use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

/// Parameter for the logistic map used when generating chaotic masks
pub const LOGISTIC_R: f64 = 3.99;

/// Generate a chaotic byte mask using the logistic map `x_{n+1} = r*x_n*(1-x_n)`.
///
/// The seed is used to initialise `x_0` by normalising it to the range (0,1).
pub fn logistic_mask(seed: u64, size: usize, r: f64) -> Vec<u8> {
    let mut x = (seed as f64) / (u64::MAX as f64);
    let mut out = Vec::with_capacity(size);
    for _ in 0..size {
        x = r * x * (1.0 - x);
        // scale to byte range
        out.push((x * 255.0) as u8);
    }
    out
}

pub fn generate_mask(seed: u64, size: usize) -> Vec<u8> {
    if seed == LOGISTIC_SEED {
        return logistic_mask(seed, size, LOGISTIC_R);
    }
    let mut rng = StdRng::seed_from_u64(seed);
    (0..size).map(|_| rng.gen::<u8>()).collect()
}

pub const PI_SEED: u64 = 3141592653;
pub const PHI_SEED: u64 = 1618033988;
pub const LOGISTIC_SEED: u64 = 2718281828;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn logistic_mask_deterministic() {
        let mask1 = logistic_mask(12345, 16, LOGISTIC_R);
        let mask2 = logistic_mask(12345, 16, LOGISTIC_R);
        assert_eq!(mask1, mask2);
    }

    #[test]
    fn generate_mask_with_logistic_seed() {
        let mask1 = generate_mask(LOGISTIC_SEED, 16);
        let mask2 = generate_mask(LOGISTIC_SEED, 16);
        assert_eq!(mask1, mask2);
        assert_eq!(mask1, logistic_mask(LOGISTIC_SEED, 16, LOGISTIC_R));
    }
}
