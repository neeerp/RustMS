use aes::block_cipher::generic_array::GenericArray;
use aes::block_cipher::{BlockCipher, NewBlockCipher};
use aes::Aes256;

use crate::constants;

pub struct MapleAES {
    iv: Vec<u8>,
    maple_version: i16,
}

impl MapleAES {
    /// Instantiate a new Maple AES Cipher
    pub fn new(iv: Vec<u8>, maple_version: i16) -> MapleAES {
        let maple_version: i16 =
            ((maple_version >> 8) & 0xFF) | ((((maple_version as u32) << 8) & 0xFF00) as i16);

        MapleAES { iv, maple_version }
    }

    /// Encrypt data using Maplestory's custom AES encryption
    pub fn crypt(&mut self, data: &mut [u8]) {
        let mut remaining = data.len();
        let mut llength = 0x5B0;
        let mut start = 0;

        let mut key = self.get_trimmed_user_key();
        let key = GenericArray::from_mut_slice(&mut key);
        let cipher = Aes256::new(&key);

        while remaining > 0 {
            let mut iv = self.repeat_bytes(&self.iv, 4);
            let mut iv = GenericArray::from_mut_slice(&mut iv);

            if remaining < llength {
                llength = remaining;
            }
            for i in start..(start + llength) {
                if (i - start) % iv.len() == 0 {
                    cipher.encrypt_block(&mut iv);
                }
                data[i] ^= iv[(i - start) % iv.len()];
            }
            start += llength;
            remaining -= llength;
            llength = 0x5B4;
        }

        self.update_iv();
    }

    /// Check if header is for a valid maplestory packet
    pub fn check_header(&self, packet: &[u8]) -> bool {
        ((packet[0] ^ self.iv[2]) & 0xFF) == ((self.maple_version >> 8) as u8 & 0xFF)
            && ((packet[1] ^ self.iv[3]) & 0xFF) == (self.maple_version & 0xFF) as u8
    }

    /// Check if header is for a valid maplestory packet, taking the header as a u32
    pub fn check_header_u32(&self, packet_header: u32) -> bool {
        let header_buf: Vec<u8> = vec![
            (packet_header >> 24) as u8 & 0xFF,
            (packet_header >> 16) as u8 & 0xFF,
        ];
        self.check_header(&header_buf)
    }

    /// Generate a packet header for a packet of a given length using the
    /// stored Maplestory version and nonce.
    pub fn gen_packet_header(&self, length: i16) -> Vec<u8> {
        let mut iiv: u32 = self.iv[3] as u32 & 0xFF;
        iiv |= ((self.iv[2] as u32) << 8) & 0xFF00;
        iiv ^= self.maple_version as u32;
        let mlength = (((length as u32) << 8) & 0xFF00) | ((length as u32) >> 8);
        let xored_iv = iiv ^ mlength;

        vec![
            (iiv >> 8) as u8 & 0xFF,
            iiv as u8 & 0xFF,
            (xored_iv >> 8) as u8 & 0xFF,
            xored_iv as u8 & 0xFF,
        ]
    }

    /// Get packet length from header
    pub fn get_packet_length(&self, header: &[u8]) -> i16 {
        if header.len() < 4 {
            return -1;
        }

        (header[0] as i16 + ((header[1] as i16) << 8))
            ^ (header[2] as i16 + ((header[3] as i16) << 8))
    }

    // Get packet length from header, treating header as a u32
    pub fn get_packet_length_from_u32(&self, packet_header: u32) -> i32 {
        let mut packet_length = (packet_header >> 16) ^ (packet_header & 0xFFFF);
        packet_length = ((packet_length << 8) & 0xFF00) | ((packet_length >> 8) & 0xFF);

        packet_length as i32
    }

    fn get_trimmed_user_key(&self) -> [u8; 32] {
        let mut key = [0u8; 32];
        for i in (0..128).step_by(16) {
            key[i / 4] = constants::USER_KEY[i];
        }
        key
    }

    fn update_iv(&mut self) {
        self.iv = self.get_new_iv(&self.iv);
    }

    /// Shuffle algorithm to generate a new Initialization Vector based off
    /// of the old one. Based directly off of HeavenClient's implementation.
    fn get_new_iv(&self, iv: &Vec<u8>) -> Vec<u8> {
        let mut new_iv: Vec<u8> = constants::DEFAULT_AES_KEY_VALUE.to_vec();
        let shuffle_bytes = constants::SHUFFLE_BYTES;

        for i in 0..4 {
            let byte = iv[i];
            new_iv[0] = new_iv[0]
                .wrapping_add(shuffle_bytes[(new_iv[1] & 0xFF) as usize].wrapping_sub(byte));
            new_iv[1] =
                new_iv[1].wrapping_sub(new_iv[2] ^ shuffle_bytes[(byte & 0xFF) as usize] & 0xFF);
            new_iv[2] = new_iv[2] ^ (shuffle_bytes[(new_iv[3] & 0xFF) as usize].wrapping_add(byte));
            new_iv[3] = new_iv[3].wrapping_add(
                (shuffle_bytes[(byte & 0xFF) as usize] & 0xFF).wrapping_sub(new_iv[0] & 0xFF),
            );

            let mut mask = 0usize;
            mask |= (new_iv[0] as usize) & 0xFF;
            mask |= ((new_iv[1] as usize) << 8) & 0xFF00;
            mask |= ((new_iv[2] as usize) << 16) & 0xFF0000;
            mask |= ((new_iv[3] as usize) << 24) & 0xFF000000;
            mask = (mask >> 0x1D) | (mask << 3);

            for j in 0..4 {
                new_iv[j] = ((mask >> (8 * j)) & 0xFF) as u8;
            }
        }

        new_iv
    }

    /// Repeat the bytes in the given vector `mul` times.
    fn repeat_bytes(&self, input: &[u8], mul: usize) -> Vec<u8> {
        let mut result: Vec<u8> = Vec::new();
        let iv_len = input.len();

        for i in 0..(iv_len * mul) {
            result.push(input[(i % iv_len) as usize]);
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use byteorder::{BigEndian, ReadBytesExt};
    use rand::{random, thread_rng, Rng};

    #[test]
    fn test_aes_round_trip() {
        for _ in 0..25 {
            // Generate an IV
            let mut iv: Vec<u8> = Vec::new();
            for _ in 0..4 {
                iv.push(random());
            }

            // Create an encryption and decryption cipher instance
            let mut encrypt_cipher = super::MapleAES::new(iv.clone(), 27);
            let mut decrypt_cipher = super::MapleAES::new(iv.clone(), 27);

            let length: u16 = random();

            let mut to_crypt: Vec<u8> = Vec::new();
            let mut expected: Vec<u8> = Vec::new();

            for _ in 0..length {
                let byte: u8 = random();
                to_crypt.push(byte);
                expected.push(byte);
            }

            // Encrypt the bytes
            encrypt_cipher.crypt(to_crypt.as_mut_slice());
            assert_ne!(to_crypt, expected);

            // Decrypt them and verify they match
            decrypt_cipher.crypt(to_crypt.as_mut_slice());
            assert_eq!(to_crypt, expected);
        }
    }

    #[test]
    fn packet_header_round_trip() {
        let mut rng = thread_rng();

        for _ in 0..100 {
            let mut iv: Vec<u8> = Vec::new();
            for _ in 0..4 {
                iv.push(random());
            }
            let cipher = super::MapleAES::new(iv, 27);

            let initial_length: i16 = rng.gen_range(0, 16000);

            let header = cipher.gen_packet_header(initial_length);
            let header_int: u32 = header.as_slice().read_u32::<BigEndian>().unwrap();

            assert!(cipher.check_header_u32(header_int));

            let length: i16 = cipher.get_packet_length(&header);
            assert_eq!(length, initial_length);
        }
    }
}
