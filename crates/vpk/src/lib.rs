// VPK Headers (v2 only!)

use anyhow::Context;
use binrw::{BinRead, BinReaderExt, NullString};
use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

#[derive(BinRead)]
#[br(magic = b"\x34\x12\xAA\x55")]
pub struct VpkHeader {
    #[br(assert(version == 2, "VPK version must be 2 (v1 is not supported)"))]
    pub version: u32,

    /// The size, in bytes, of the directory tree
    pub tree_size: u32,
    /// How many bytes of file content are stored in this VPK file (0 in CSGO)
    pub file_data_section_size: u32,
    /// The size, in bytes, of the section containing MD5 checksums for external archive content
    pub archive_md5_section_size: u32,
    /// The size, in bytes, of the section containing MD5 checksums for content in this file (should always be 48)
    pub other_md5_section_size: u32,
    /// The size, in bytes, of the section containing the public key and signature. This is either 0 (CSGO & The Ship) or 296 (HL2, HL2:DM, HL2:EP1, HL2:EP2, HL2:LC, TF2, DOD:S & CS:S)
    pub signature_section_size: u32,
} // Total size: 28

#[derive(BinRead, Debug)]
pub struct VpkDirectoryEntry {
    pub crc: u32,
    pub preload_bytes: u16,
    pub archive_index: u16,
    pub entry_offset: u32,
    pub entry_length: u32,

    #[br(assert(terminator == 0xFFFF))]
    pub terminator: u16,
}

impl VpkDirectoryEntry {
    pub fn is_preload(&self) -> bool {
        self.preload_bytes > 0
    }
}

#[derive(Debug)]
pub struct VpkDirectoryPath {
    pub files: HashMap<String, VpkDirectoryEntry>,
}

pub struct VpkFile<R: Read + Seek> {
    reader: R,
    pub header: VpkHeader,
    /// Maps file extensions to a list of paths
    pub directory: HashMap<String, HashMap<String, VpkDirectoryPath>>,

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
    ) -> anyhow::Result<HashMap<String, HashMap<String, VpkDirectoryPath>>> {
        let mut directory = HashMap::default();
        loop {
            let extension = r
                .read_le::<NullString>()
                .context("Failed to read directory extension string")?
                .to_string();
            if extension.is_empty() {
                break;
            }

            let mut paths = HashMap::default();
            loop {
                let path = r
                    .read_le::<NullString>()
                    .context("Failed to read directory path string")?
                    .to_string();
                if path.is_empty() {
                    break;
                }

                let mut path_files = HashMap::default();

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
        let path = path.as_ref().replace("\\", "/").to_lowercase();
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
