use aes::Aes256;
use ofb::Ofb;
use ofb::stream_cipher::{NewStreamCipher, SyncStreamCipher};

type AesOfb = Ofb<Aes256>;

pub struct MapleAES {
    cipher: AesOfb,
    iv: Vec<u8>,
    maple_version: i16,
}

impl MapleAES {
    /// Instantiate a new Maple AES Cipher
    pub fn new(iv: Vec<u8>, maple_version: i16) -> MapleAES {

        // TODO: Verify that it is actually necessary to swap the bytes here...
        let maple_version: i16 = ((maple_version >> 8) & 0xFF) | ((((maple_version as u32) << 8) & 0xFF00) as i16);

        // Initialize cipher with no nonce... is that right...?
        let key = super::get_trimmed_user_key();
        let cipher: Ofb<Aes256> = AesOfb::new_var(&key, &[0u8; 16]).unwrap();
        MapleAES {
            cipher,
            iv,
            maple_version
        }
    }

    /// Encrypt data using Maplestory's custom AES encryption
    pub fn crypt(&mut self, data: &mut[u8]) {
        let mut remaining = data.len();
        let mut llength = 0x5B0;
        let mut start = 0;

        while remaining > 0 {
            let mut iv = super::multiply_bytes(&self.iv, 4, 4);
            if remaining < llength {
                llength = remaining;
            }
            for i in start..(start+llength) {
                if (i - start) % iv.len() == 0 {
                    self.cipher.apply_keystream(&mut iv)
                }
                data[i] ^= iv[(i - start) % iv.len()];
            }
            start += llength;
            remaining -= llength;
            llength = 0x5B4;
        }

        self.update_iv();
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
            xored_iv as u8 & 0xFF
        ]
    }

    /// Check if header is for a valid maplestory packet
    pub fn check_header(&self, packet: &[u8]) -> bool {
        ((packet[0] ^ self.iv[2]) & 0xFF) == ((self.maple_version >> 8) as u8 & 0xFF) &&
        ((packet[1] ^ self.iv[3]) & 0xFF) == (self.maple_version & 0xFF) as u8
    }

    /// Check if header is for a valid maplestory packet, taking the header as a u32
    pub fn check_header_u32(&self, packet_header: u32) -> bool {
        let header_buf: Vec<u8> = vec![
            (packet_header >> 24) as u8 & 0xFF,
            (packet_header >> 16) as u8 & 0xFF
        ];
        self.check_header(&header_buf)
    }

    /// Get packet length from header
    pub fn get_packet_length(&self, header: &[u8]) -> i16 {
        if header.len() < 4 {
            return -1;
        }

        (header[0] as i16 + ((header[1] as i16) << 8)) ^
        (header[2] as i16 + ((header[3] as i16) << 8)) 
    }

    // Get packet length from header, treating header as a u32
    pub fn get_packet_length_from_u32(&self, packet_header: u32) -> i32 {
        let mut packet_length = (packet_header >> 16) ^ (packet_header & 0xFFFF);
        packet_length = ((packet_length << 8) & 0xFF00) | ((packet_length >> 8) & 0xFF);

        packet_length as i32
    }

    fn update_iv(&mut self) {
        self.iv = super::get_new_iv(&self.iv);
    }
}

#[cfg(test)]
mod tests {
    use rand::{random, thread_rng, Rng};
    use byteorder::{BigEndian, ReadBytesExt};

    #[test]
    fn test_aes_round_trip() {
        for _ in 0..25 {
            // Generate an IV
            let mut iv: Vec<u8> = Vec::new();
            for _ in 0..16 {
                iv.push(random());
            }

            // Create an encryption and decryption cipher instance
            let mut encrypt_cipher = super::MapleAES::new(iv.clone(),  27);
            let mut decrypt_cipher = super::MapleAES::new(iv.clone(),  27);

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
            let cipher = super::MapleAES::new(iv,  27);

            let initial_length: i16 = rng.gen_range(0, 16000);

            let header = cipher.gen_packet_header(initial_length);
            let header_int: u32 = header.as_slice().read_u32::<BigEndian>().unwrap();

            assert!(cipher.check_header_u32(header_int));

            let length: i16 = cipher.get_packet_length(&header);
            assert_eq!(length, initial_length);
        }
    }
}
