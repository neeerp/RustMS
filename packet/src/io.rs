pub trait PktWrite {
    /// Write a byte to the end of a packet.
    fn write_byte(&mut self, byte: u8);

    /// Write a byte array to the end of a packet.
    fn write_bytes(&mut self, bytes: &[u8]);

    /// Write a short integer to the end of a packet in Little Endian format.
    fn write_short(&mut self, short: i16);

    /// Write an integer to the end of a packet in Little Endian format.
    fn write_int(&mut self, int: i32);

    /// Write a long integer to the end of a packet in Little Endian format.
    fn write_long(&mut self, long: i64);

    /// Write a string to the end of a packet.
    fn write_str(&mut self, string: &str);

    /// Write a string's length followed by the string itself to the end of a packet.
    fn write_str_with_length(&mut self, string: &str);
}

pub trait PktRead {
    /// Read the `pos`th byte of the packet.
    fn read_byte(&self, pos: usize) -> u8;

    /// Read a byte array of a given `length` starting at the `pos`th byte of the packet.
    fn read_bytes(&self, pos: usize, length: usize) -> &[u8];

    /// Read a short integer from the `pos`th byte of the packet.
    fn read_short(&self, pos: usize) -> i16;

    /// Read an integer from the `pos`th byte of the packet.
    fn read_int(&self, pos: usize) -> i32;

    /// Read a long integer from the `pos`th byte of the packet.
    fn read_long(&self, pos: usize) -> i64;

    /// Read a string of a given `length` starting at the `pos`th byte of the packet.
    fn read_str(&self, pos: usize, length: usize) -> String;

    /// Read a length-headered string starting at the `pos`th byte of the packet.
    fn read_str_with_length(&self, pos: usize) -> String;
}
