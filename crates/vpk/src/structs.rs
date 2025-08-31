use binrw::BinRead;

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
