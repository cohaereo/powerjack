use std::io::Read;

use glam::Vec3;

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

    pub fn read_bits_vec(&mut self, count: usize) -> Vec<u8> {
        let mut bytes = vec![0; count.div_ceil(8)];
        for i in 0..count {
            if self.read_bit() {
                bytes[i / 8] |= 1 << (i % 8);
            }
        }
        bytes
    }

    pub fn read_float_compressed(&mut self) -> f32 {
        let has_int = self.read_bit();
        let has_frac = self.read_bit();
        if !has_int && !has_frac {
            return 0.0;
        }
        let sign = if self.read_bit() { -1.0 } else { 1.0 };
        let int_part = if has_int {
            self.read_bits(14) as f32 + 1.0
        } else {
            0.0
        };
        let frac_part = if has_frac {
            self.read_bits(5) as f32 * (1.0 / 32.0)
        } else {
            0.0
        };
        sign * (int_part + frac_part)
    }

    pub fn read_vec3_compressed(&mut self) -> Vec3 {
        let mut v = Vec3::ZERO;
        let has_x = self.read_bit();
        let has_y = self.read_bit();
        let has_z = self.read_bit();

        if has_x {
            v.x = self.read_float_compressed();
        }
        if has_y {
            v.y = self.read_float_compressed();
        }
        if has_z {
            v.z = self.read_float_compressed();
        }

        v
    }

    pub fn read_angle(&mut self, bits: usize) -> f32 {
        let shift = 1 << bits;
        let value = self.read_bits(bits) as f32;
        value * (360.0 / shift as f32)
    }

    pub fn read_varint32(&mut self) -> u32 {
        let mut value = 0;
        let mut shift = 0;
        loop {
            let byte = self.read_bits(8);
            value |= (byte & 0x7F) << shift;
            if byte & 0x80 == 0 {
                break;
            }
            shift += 7;
        }
        value
    }

    pub fn remaining_bytes(&self) -> &[u8] {
        &self.data[self.bit.div_ceil(8)..]
    }

    pub fn bits_remaining(&self) -> usize {
        self.data.len() * 8 - self.bit
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
