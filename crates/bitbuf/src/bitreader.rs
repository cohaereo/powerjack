use std::io::Read;

pub struct BitReader {
    data: Vec<u8>,
    bit: usize,
}

impl BitReader {
    pub fn new(data: Vec<u8>) -> Self {
        BitReader { data, bit: 0 }
    }

    pub fn read_bit(&mut self) -> bool {
        let byte = self.data[self.bit / 8];
        let value = byte >> (self.bit % 8);
        self.bit += 1;
        value & 1 != 0
    }

    pub fn read_bits(&mut self, count: usize) -> u32 {
        let mut value = 0;
        for i in 0..count {
            value |= (self.read_bit() as u32) << i;
        }
        value
    }

    pub fn remaining_bytes(&self) -> &[u8] {
        &self.data[self.bit.div_ceil(8)..]
    }
}

impl Read for BitReader {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        for b in &mut *buf {
            *b = self.read_bits(8) as u8;
        }
        Ok(buf.len())
    }
}
