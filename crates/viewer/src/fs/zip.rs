use crate::fs::Mountable;
use std::io::{Read, Seek};
use zip_lzma::result::ZipError;

impl<R: Read + Seek + Send + Sync> Mountable for zip_lzma::read::ZipArchive<R> {
    fn read_path(&mut self, path: &str) -> eyre::Result<Option<Vec<u8>>> {
        let mut path = path.to_lowercase().replace("\\", "/");
        // Eliminate double path separators
        while path.contains("//") {
            path = path.replace("//", "/");
        }

        let Some(real_path) = self
            .file_names()
            .find(|zip_path| zip_path.to_lowercase() == *path)
            .map(|s| s.to_string())
        else {
            return Ok(None);
        };

        let mut file = match self.by_name(&real_path) {
            Ok(o) => o,
            Err(e) => {
                return match e {
                    ZipError::FileNotFound => Ok(None),
                    _ => Err(e.into()),
                };
            }
        };

        let mut data = vec![0u8; file.size() as usize];
        file.read_exact(&mut data)?;
        Ok(Some(data))
    }
}
