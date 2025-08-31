use std::{
    fs::File,
    io::{BufReader, Cursor},
    path::Path,
    sync::Arc,
};

use ::zip::ZipArchive;
use parking_lot::Mutex;
use powerjack_vpk::VpkFile;

pub mod vpk;
pub mod zip;

pub trait Mountable: Send + Sync {
    fn read_path(&mut self, path: &str) -> anyhow::Result<Option<Vec<u8>>>;

    // /// Get all available paths in the mount point. All paths are lowercase
    // fn get_all_paths(&self) -> Vec<PathBuf>;
}

pub type SharedFilesystem = Arc<Mutex<Filesystem>>;
pub struct Filesystem {
    mounts: Vec<Box<dyn Mountable>>,
}

impl Filesystem {
    pub fn new() -> Self {
        Filesystem { mounts: Vec::new() }
    }

    pub fn add_mount(&mut self, mount: Box<dyn Mountable>) {
        self.mounts.push(mount);
    }

    pub fn mount_vpk(&mut self, path: impl AsRef<Path>) -> anyhow::Result<()> {
        info!("Mounting VPK '{}'", path.as_ref().display());
        let f = BufReader::with_capacity(1024 * 1024, File::open(&path)?);
        self.add_mount(Box::new(VpkFile::new(
            f,
            Some(path.as_ref().to_string_lossy().to_string()),
        )?));
        Ok(())
    }

    pub fn mount_zip(&mut self, zip: Vec<u8>) -> anyhow::Result<()> {
        self.add_mount(Box::new(ZipArchive::new(Cursor::new(zip))?));
        Ok(())
    }

    pub fn read_path(&mut self, path: &str) -> anyhow::Result<Option<Vec<u8>>> {
        for mount in &mut self.mounts {
            if let Some(data) = mount.read_path(path)? {
                return Ok(Some(data));
            }
        }
        Ok(None)
    }
}

impl Default for Filesystem {
    fn default() -> Self {
        Self::new()
    }
}
