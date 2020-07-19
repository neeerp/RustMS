/// Functions to encrypt and decrypt data using Maplestory's custom
/// encryption algorithm.

/// Encrypt bytes using Maplestory's custom encryption algorithm.
pub fn encrypt(data: &mut [u8]) {
    let size: usize = data.len();
    let mut c: u8;
    let mut a: u8;
    for _ in 0..3 {
        a = 0;
        for j in (1..(size + 1)).rev() {
            c = data[size - j];
            c = rotl(c, 3);
            c = (c as usize).overflowing_add(j).0 as u8;
            c ^= a;
            a = c;
            c = rotr(a, j as u32);
            c ^= 0xFF;
            c = c.overflowing_add(0x48).0;
            data[size - j] = c;
        }
        a = 0;
        for j in (1..(size + 1)).rev() {
            c = data[j - 1];
            c = rotl(c, 4);
            c = (c as usize).overflowing_add(j).0 as u8;
            c ^= a;
            a = c;
            c ^= 0x13;
            c = rotr(c, 3);
            data[j - 1] = c;
        }
    }
}

/// Decrypt bytes encrypted with Maplestory's custom encryption algorithm.
pub fn decrypt(data: &mut [u8]) {
    let size: usize = data.len();
    let mut a: u8;
    let mut b: u8;
    let mut c: u8;
    for _ in 0..3 {
        b = 0;
        for j in (1..(size + 1)).rev() {
            c = data[j - 1];
            c = rotl(c, 3);
            c ^= 0x13;
            a = c;
            c ^= b;
            c = (c as usize).overflowing_sub(j).0 as u8; // Guess this is supposed to be right?
            c = rotr(c, 4);
            b = a;
            data[j - 1] = c;
        }
        b = 0;
        for j in (1..(size + 1)).rev() {
            c = data[size - j];
            c = c.overflowing_sub(0x48).0;
            c ^= 0xFF;
            c = rotl(c, j as u32);
            a = c;
            c ^= b;
            c = (c as usize).overflowing_sub(j).0 as u8; // Guess this is supposed to be right?
            c = rotr(c, 3);
            b = a;
            data[size - j] = c;
        }
    }
}

/// Roll a byte left count times
fn rotl(byte: u8, count: u32) -> u8 {
    let count = count % 8;
    if count > 0 { (byte << count) | (byte >> (8 - count)) } else { byte }
}

/// Roll a byte right count times
fn rotr(byte: u8, count: u32) -> u8 {
    let count = count % 8;
    if count > 0 { (byte >> count) | (byte << (8 - count)) } else { byte }
}

#[cfg(test)]
mod tests {
    use rand::{random, thread_rng, Rng};

    #[test]
    fn test_rot_nop_on_max_and_zero() {
        let max: u8 = u8::MAX;
        let zero: u8 = 0;

        for i in 0..9 {
            assert_eq!(super::rotl(max, i), max);
            assert_eq!(super::rotr(max, i), max);
            assert_eq!(super::rotl(zero, i), zero);
            assert_eq!(super::rotr(zero, i), zero);
        }
    }

    #[test]
    fn test_rot_zero_nop() {
        for _ in 0..100 {
            let byte: u8 = random();
            assert_eq!(super::rotl(byte, 0), byte);
            assert_eq!(super::rotr(byte, 0), byte);
        }
    }

    #[test]
    fn test_rot_eight_nop() {
        for _ in 0..100 {
            let byte: u8 = random();
            assert_eq!(super::rotl(byte, 8), byte);
            assert_eq!(super::rotr(byte, 8), byte);
        }
    }

    #[test]
    fn test_rotl_rotr_equivalence() {
        for _ in 0..100 {
            let byte: u8 = random();
            for i in 0..9 {
                assert_eq!(super::rotl(byte, i), super::rotr(byte, 8 - i));
            }
        }
    }

    #[test]
    fn test_rotl_powers_of_two() {
        let num: u8 = 1;
        for i in 0..17 {
            assert_eq!(super::rotl(num, i), 2u8.pow(i % 8))
        }
    }
    
    #[test]
    fn test_rotr_powers_of_two() {
        let num: u8 = 1;
        for i in 0..17 {
            assert_eq!(super::rotr(num, i), 2u8.pow((8 - (i % 8)) % 8))
        }
    }


    #[test]
    fn test_encrypt_decrypt_original() {
        let mut rng = thread_rng();
        for _ in 0..100 {
            let length = rng.gen_range(1, 255);
            let mut bytes: Vec<u8> = Vec::new();
            let mut expected: Vec<u8> = Vec::new();

            for _ in 0..length {
                let byte: u8 = random();
                bytes.push(byte);
                expected.push(byte);
                
            }

            super::encrypt(&mut bytes);
            assert_ne!(expected, bytes);
            super::decrypt(&mut bytes);
            assert_eq!(expected, bytes);
        }
    }
}
