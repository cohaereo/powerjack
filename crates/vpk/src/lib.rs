use anyhow::Context;
use binrw::{BinReaderExt, NullString};
use case_insensitive_hashmap::CaseInsensitiveHashMap;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

use crate::structs::{VpkDirectoryEntry, VpkHeader};

mod structs;

#[derive(Debug)]
pub struct VpkDirectoryPath {
    pub files: CaseInsensitiveHashMap<VpkDirectoryEntry>,
}

pub struct VpkFile<R: Read + Seek> {
    reader: R,
    pub header: VpkHeader,
    /// Maps file extensions to a list of paths
    pub directory: CaseInsensitiveHashMap<CaseInsensitiveHashMap<VpkDirectoryPath>>,

    dir_path: Option<String>,
}

impl<R: Read + Seek> VpkFile<R> {
    /// If filename is not given, you will not be able to read non-preload files through this struct
    pub fn new(mut reader: R, filename: Option<String>) -> anyhow::Result<Self> {
        let header = reader.read_le::<VpkHeader>()?;
        Ok(Self {
            directory: Self::read_directory(&mut reader).context("Failed to read VPK directory")?,
            reader,
            header,
            dir_path: filename,
        })
    }

    fn read_directory(
        r: &mut R,
    ) -> anyhow::Result<CaseInsensitiveHashMap<CaseInsensitiveHashMap<VpkDirectoryPath>>> {
        let mut directory = CaseInsensitiveHashMap::new();
        loop {
            let extension = r
                .read_le::<NullString>()
                .context("Failed to read directory extension string")?
                .to_string();
            if extension.is_empty() {
                break;
            }

            let mut paths = CaseInsensitiveHashMap::with_capacity(4096);
            loop {
                let path = r
                    .read_le::<NullString>()
                    .context("Failed to read directory path string")?
                    .to_string();
                if path.is_empty() {
                    break;
                }

                let mut path_files = CaseInsensitiveHashMap::with_capacity(4096);

                loop {
                    let filename = r
                        .read_le::<NullString>()
                        .context("Failed to read directory filename string")?
                        .to_string();
                    if filename.is_empty() {
                        break;
                    }

                    let entry = r
                        .read_le::<VpkDirectoryEntry>()
                        .context("Failed to read directory entry")?;

                    // Skip the preload bytes, if any
                    r.seek(SeekFrom::Current(entry.preload_bytes as i64))?;

                    path_files.insert(filename, entry);
                }

                paths.insert(path, VpkDirectoryPath { files: path_files });
            }

            directory.insert(extension, paths);
        }

        Ok(directory)
    }

    pub fn read_data_from_path(
        &mut self,
        path: impl AsRef<str>,
    ) -> anyhow::Result<Option<Vec<u8>>> {
        let path = path.as_ref().replace("\\", "/");
        let path = Path::new(&path);
        let extension = path
            .extension()
            .context("Failed to get path extension")?
            .to_str()
            .context("Failed to convert path extension to string")?;

        let Some(file_path) = path.parent() else {
            return Ok(None);
        };

        let filename = path.file_stem().context("Path does not have a filename")?;

        let Some(extension) = self.directory.get(extension) else {
            return Ok(None);
        };
        let Some(path) = extension.get(
            file_path
                .as_os_str()
                .to_str()
                .context("Failed to convert path to string")?,
        ) else {
            return Ok(None);
        };

        let Some(entry) = path.files.get(
            filename
                .to_str()
                .context("Failed to convert filename to string")?,
        ) else {
            return Ok(None);
        };

        if entry.is_preload() {
            anyhow::bail!("Preload files are not supported");
        }

        let archive_path = self
            .dir_path
            .as_ref()
            .context("No VPK filename given, cannot read files")?
            .replace("_dir.vpk", &format!("_{:03}.vpk", entry.archive_index));

        let mut archive_file = File::open(&archive_path)?;
        archive_file.seek(SeekFrom::Start(entry.entry_offset as u64))?;
        let mut data = vec![0; entry.entry_length as usize];
        archive_file.read_exact(&mut data)?;

        Ok(Some(data))
    }

    /// Reclaim the reader (destroys the VpkFile)
    pub fn reclaim(self) -> R {
        self.reader
    }
}
