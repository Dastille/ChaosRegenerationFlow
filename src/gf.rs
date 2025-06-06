pub fn gf_mul(mut a: u8, mut b: u8) -> u8 {
    let mut p: u8 = 0;
    for _ in 0..8 {
        if (b & 1) != 0 {
            p ^= a;
        }
        let carry = a & 0x80;
        a <<= 1;
        if carry != 0 {
            a ^= 0x1b; // primitive polynomial x^8 + x^4 + x^3 + x + 1 (0x11b)
        }
        b >>= 1;
    }
    p
}

pub fn gf_pow(mut x: u8, mut power: u16) -> u8 {
    let mut result: u8 = 1;
    while power > 0 {
        if (power & 1) != 0 {
            result = gf_mul(result, x);
        }
        x = gf_mul(x, x);
        power >>= 1;
    }
    result
}

pub fn gf_inv(x: u8) -> u8 {
    if x == 0 { 0 } else { gf_pow(x, 254) }
}

#[cfg(all(test, feature = "gf"))]
mod tests {
    use super::*;

    #[test]
    fn inverse_roundtrip() {
        for a in 1u8..=255 {
            let inv = gf_inv(a);
            assert_eq!(gf_mul(a, inv), 1, "{}", a);
        }
    }
}
