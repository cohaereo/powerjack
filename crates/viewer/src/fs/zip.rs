use crate::fs::Mountable;
use std::io::{Read, Seek};
use std::path::Path;
use zip_lzma::result::ZipError;

impl<R: Read + Seek + Send + Sync> Mountable for zip_lzma::read::ZipArchive<R> {
    fn read_path(&mut self, path: &str) -> anyhow::Result<Option<Vec<u8>>> {
        let path = path.to_lowercase().replace("\\", "/");
        let path = Path::new(&path);

        let mut file = match self.by_name(&path.to_string_lossy()) {
            Ok(o) => o,
            Err(e) => {
                return match e {
                    ZipError::FileNotFound => Ok(None),
                    _ => Err(e.into()),
                }
            }
        };

        let mut data = vec![0u8; file.size() as usize];
        file.read_exact(&mut data)?;
        Ok(Some(data))
    }
}
