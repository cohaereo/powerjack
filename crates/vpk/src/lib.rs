use binrw::{BinReaderExt, NullString};
use case_insensitive_hashmap::CaseInsensitiveHashMap;
use eyre::{Context, OptionExt};
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

use crate::structs::{VpkDirectoryEntry, VpkHeader};

mod structs;

type PathMap<V> = CaseInsensitiveHashMap<V>;
type ExtensionMap<V> = CaseInsensitiveHashMap<V>;

pub struct VpkFile<R: Read + Seek> {
    reader: R,
    pub header: VpkHeader,
    /// Maps file extensions to a list of paths
    pub directory: ExtensionMap<PathMap<VpkDirectoryEntry>>,

    dir_path: Option<String>,
}

impl<R: Read + Seek> VpkFile<R> {
    /// If filename is not given, you will not be able to read non-preload files through this struct
    pub fn new(mut reader: R, filename: Option<String>) -> eyre::Result<Self> {
        let header = reader.read_le::<VpkHeader>()?;
        Ok(Self {
            directory: Self::read_directory(&mut reader).context("Failed to read VPK directory")?,
            reader,
            header,
            dir_path: filename,
        })
    }

    fn read_directory(r: &mut R) -> eyre::Result<ExtensionMap<PathMap<VpkDirectoryEntry>>> {
        let mut directory = ExtensionMap::with_capacity_and_hasher(32, Default::default());
        loop {
            let extension = r
                .read_le::<NullString>()
                .context("Failed to read directory extension string")?
                .to_string();
            if extension.is_empty() {
                break;
            }

            let mut paths = PathMap::with_capacity(4096);
            loop {
                let path = r
                    .read_le::<NullString>()
                    .context("Failed to read directory path string")?
                    .to_string();
                if path.is_empty() {
                    break;
                }

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

                    paths.insert(format!("{path}/{filename}"), entry);
                }
            }

            paths.shrink_to_fit();
            directory.insert(extension, paths);
        }

        Ok(directory)
    }

    pub fn read_data_from_path(&mut self, path: impl AsRef<str>) -> eyre::Result<Option<Vec<u8>>> {
        let mut path = path.as_ref().replace("\\", "/");
        // Eliminate double path separators
        while path.contains("//") {
            path = path.replace("//", "/");
        }

        let path = Path::new(&path);
        let extension = path
            .extension()
            .ok_or_eyre("Failed to get path extension")?
            .to_str()
            .ok_or_eyre("Failed to convert path extension to string")?;

        let file_path = path.with_extension("");

        let Some(extension) = self.directory.get(extension) else {
            return Ok(None);
        };
        let Some(entry) = extension.get(
            file_path
                .as_os_str()
                .to_str()
                .ok_or_eyre("Failed to convert path to string")?,
        ) else {
            return Ok(None);
        };

        if entry.is_preload() {
            eyre::bail!("Preload files are not supported");
        }

        let archive_path = self
            .dir_path
            .as_ref()
            .ok_or_eyre("No VPK filename given, cannot read files")?
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
