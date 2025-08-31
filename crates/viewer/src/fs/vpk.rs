use std::io::{Read, Seek};

use powerjack_vpk::VpkFile;

use crate::fs::Mountable;

impl<R: Read + Seek + Send + Sync> Mountable for VpkFile<R> {
    fn read_path(&mut self, path: &str) -> anyhow::Result<Option<Vec<u8>>> {
        self.read_data_from_path(path)
    }
}
