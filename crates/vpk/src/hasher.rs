use std::{
    collections::HashMap,
    hash::{BuildHasherDefault, DefaultHasher, Hasher},
};

/// A case-insensitive hash function wrapper that converts any strings to lowercase before hashing, effectively making hash lookups case-insensitive.
///
/// Note: To be compatible with stable, this hooks into the `write` function, converting any byte slice that looks like a valid string to lowercase. Use with caution when hashing non-string data.
pub struct CaseInsensitiveHasher<H: Hasher>(H);

impl<H: Hasher + Default> Default for CaseInsensitiveHasher<H> {
    fn default() -> Self {
        CaseInsensitiveHasher(H::default())
    }
}

impl<H: Hasher> Hasher for CaseInsensitiveHasher<H> {
    fn finish(&self) -> u64 {
        self.0.finish()
    }

    fn write(&mut self, bytes: &[u8]) {
        if let Ok(s) = str::from_utf8(bytes) {
            let lower = s.to_lowercase();
            self.0.write(lower.as_bytes());
        } else {
            self.0.write(bytes)
        }
    }

    fn write_u8(&mut self, i: u8) {
        self.0.write_u8(i)
    }

    fn write_u16(&mut self, i: u16) {
        self.0.write_u16(i)
    }

    fn write_u32(&mut self, i: u32) {
        self.0.write_u32(i)
    }

    fn write_u64(&mut self, i: u64) {
        self.0.write_u64(i)
    }

    fn write_u128(&mut self, i: u128) {
        self.0.write_u128(i)
    }

    fn write_usize(&mut self, i: usize) {
        self.0.write_usize(i)
    }

    fn write_i8(&mut self, i: i8) {
        self.0.write_u8(i as u8)
    }

    fn write_i16(&mut self, i: i16) {
        self.0.write_u16(i as u16)
    }

    fn write_i32(&mut self, i: i32) {
        self.0.write_u32(i as u32)
    }

    fn write_i64(&mut self, i: i64) {
        self.0.write_u64(i as u64)
    }

    fn write_i128(&mut self, i: i128) {
        self.0.write_u128(i as u128)
    }

    fn write_isize(&mut self, i: isize) {
        self.0.write_usize(i as usize)
    }

    // TODO(cohae): Use this when stable
    // fn write_str(&mut self, s: &str) {
    //     self.0.write_str(&s.to_lowercase());
    // }
}

pub type RandomStateCaseInsensitive = BuildHasherDefault<CaseInsensitiveHasher<DefaultHasher>>;
pub type HashMapCaseInsensitive<K, V> = HashMap<K, V, RandomStateCaseInsensitive>;
