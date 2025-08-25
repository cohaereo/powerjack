use std::io::Read;

use glam::Vec3;

pub trait ReaderExt {
    fn read_u8(&mut self) -> std::io::Result<u8>;
    fn read_u16(&mut self) -> std::io::Result<u16>;
    fn read_u32(&mut self) -> std::io::Result<u32>;
    fn read_f32(&mut self) -> std::io::Result<f32>;

    fn read_i8(&mut self) -> std::io::Result<i8> {
        Ok(u8::cast_signed(self.read_u8()?))
    }

    fn read_i16(&mut self) -> std::io::Result<i16> {
        Ok(u16::cast_signed(self.read_u16()?))
    }

    fn read_i32(&mut self) -> std::io::Result<i32> {
        Ok(u32::cast_signed(self.read_u32()?))
    }

    fn read_vec3(&mut self) -> std::io::Result<Vec3> {
        let x = self.read_f32()?;
        let y = self.read_f32()?;
        let z = self.read_f32()?;
        Ok(Vec3::new(x, y, z))
    }

    fn read_string(&mut self, len: usize) -> std::io::Result<String>;
    fn read_nullstring(&mut self) -> std::io::Result<String>;

    fn read_bytes(&mut self, len: usize) -> std::io::Result<Vec<u8>>;
}

impl<R: Read> ReaderExt for R {
    fn read_u8(&mut self) -> std::io::Result<u8> {
        let mut buf = [0; 1];
        self.read_exact(&mut buf)?;
        Ok(buf[0])
    }

    fn read_u16(&mut self) -> std::io::Result<u16> {
        let mut buf = [0; 2];
        self.read_exact(&mut buf)?;
        Ok(u16::from_le_bytes(buf))
    }

    fn read_u32(&mut self) -> std::io::Result<u32> {
        let mut buf = [0; 4];
        self.read_exact(&mut buf)?;
        Ok(u32::from_le_bytes(buf))
    }

    fn read_f32(&mut self) -> std::io::Result<f32> {
        let mut buf = [0; 4];
        self.read_exact(&mut buf)?;
        Ok(f32::from_le_bytes(buf))
    }

    fn read_string(&mut self, len: usize) -> std::io::Result<String> {
        let mut buf = vec![0; len];
        self.read_exact(&mut buf)?;
        Ok(String::from_utf8_lossy(&buf)
            .trim_end_matches('\0')
            .to_owned())
    }

    fn read_nullstring(&mut self) -> std::io::Result<String> {
        let mut buf = Vec::new();
        loop {
            let byte = self.read_u8()?;
            if byte == 0 {
                break;
            }
            buf.push(byte);
        }
        Ok(String::from_utf8_lossy(&buf).to_string())
    }

    fn read_bytes(&mut self, len: usize) -> std::io::Result<Vec<u8>> {
        let mut buf = vec![0; len];
        self.read_exact(&mut buf)?;
        Ok(buf)
    }
}
