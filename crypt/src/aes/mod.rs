use crate::constants;

pub mod cipher;

fn get_trimmed_user_key() -> [u8; 32] {
    let mut key = [0u8; 32];
    for i in (0..128).step_by(16) {
        key[i / 4] = constants::USER_KEY[i];
    }
    key
}

/// Generate a new initialization vector using the old one
fn get_new_iv(old_iv: &Vec<u8>) -> Vec<u8> {
    let mut start = constants::DEFAULT_AES_KEY_VALUE.to_vec();
    for i in 0..4 {
        shuffle(old_iv[i], &mut start);
    }
    start
}

/// Shuffles the bytes in the initialization vector
/// 
/// The input byte comes from the old initialization vector
///
/// Start is the default AES key
fn shuffle(input_byte: u8, start: &mut[u8]) {
    let mut a = start[1];
    let mut b = a;
    let mut c: u32;
    let mut d: u32;
    b = constants::SHUFFLE_BYTES[b as usize];
    b = b.overflowing_sub(input_byte).0;
    start[0] = start[0].overflowing_add(b).0;
    b = start[2];
    b ^= constants::SHUFFLE_BYTES[input_byte as usize];
    a = a.overflowing_sub(b).0;
    start[1] = a;
    a = start[3];
    b = a;
    a = a.overflowing_sub(start[0]).0;
    b = constants::SHUFFLE_BYTES[b as usize];
    b = b.overflowing_sub(input_byte).0;
    b ^= start[2];
    start[2] = b;
    a = a.overflowing_add(constants::SHUFFLE_BYTES[input_byte as usize]).0;
    start[3] = a;

    c = start[0] as u32 + start[1] as u32 * 0x100 + start[2] as u32 * 0x10000 + start[3] as u32 * 0x1000000;
    d = c;
    c >>= 0x1D;
    d <<= 0x03;
    c |= d;
    start[0] = (c % 0x100) as u8;
    c /= 0x100;
    start[1] = (c % 0x100) as u8;
    c /= 0x100;
    start[2] = (c % 0x100) as u8;
    start[3] = (c / 0x100) as u8;
}

fn multiply_bytes(input: &[u8], count: i32, mul: i32) -> Vec<u8> {
    let mut result: Vec<u8> = Vec::new();

    for i in 0..(count*mul) {
        result.push(input[(i % count) as usize]);
    }

    result
}
