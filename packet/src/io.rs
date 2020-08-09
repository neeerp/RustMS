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
    fn read_byte(&self, pos: usize) -> u8;

    fn read_bytes(&self, pos: usize, length: i16) -> &[u8];

    fn read_short(&self, pos: usize) -> i16;

    fn read_int(&self, pos: usize) -> i32;

    fn read_long(&self, pos: usize) -> i64;

    fn read_str(&self, pos: usize, length: i16) -> &str;

    fn read_str_with_length(&self, pos: usize) -> &str;
}
