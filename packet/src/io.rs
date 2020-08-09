pub trait PktWrite {
    fn write_byte(&mut self, byte: u8);

    fn write_bytes(&mut self, bytes: &[u8]);

    fn write_short(&mut self, short: i16);

    fn write_int(&mut self, int: i32);

    fn write_long(&mut self, long: i64);

    fn write_str(&mut self, string: &str);

    fn write_str_with_length(&mut self, string: &str);
}

pub trait PktRead {
    fn read_byte(&mut self) -> u8;

    fn read_bytes(&mut self, length: i16) -> &[u8];

    fn read_short(&mut self) -> i16;

    fn read_int(&mut self) -> i32;

    fn read_long(&mut self) -> i64;

    fn read_str(&mut self, length: i16) -> &str;

    fn read_str_with_length(&mut self) -> &str;
}
