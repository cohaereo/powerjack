use anyhow::Context;
use binrw::{BinRead, BinReaderExt, BinWriterExt};
use lumps::{BspColorRgbExp, BspFace, BspModel, BspPlane, BspTexData, BspTexInfo};
use std::io::{Cursor, Read, Seek, SeekFrom, Write};

pub const BSP_LUMP_COUNT: usize = 64;

pub mod lumps;

#[derive(BinRead, Debug)]
#[br(magic = b"VBSP")]
pub struct BspHeader {
    pub version: i32,
    #[br(count = BSP_LUMP_COUNT)]
    pub lumps: Vec<BspLump>,
}

#[derive(BinRead, Debug)]
pub struct BspLump {
    pub offset: u32,
    pub length: u32,
    pub version: i32,
    pub fourcc: [u8; 4],
}

pub struct BspFile<R: Read + Seek> {
    reader: R,
    pub header: BspHeader,
}

impl<R: Read + Seek> BspFile<R> {
    pub fn new(mut reader: R) -> anyhow::Result<Self> {
        let header = reader.read_le::<BspHeader>()?;
        Ok(Self { reader, header })
    }

    pub fn read_lump_raw(&mut self, index: usize) -> anyhow::Result<Vec<u8>> {
        let lump = self
            .header
            .lumps
            .get(index)
            .context("Lump index out of bounds")?;

        self.reader.seek(SeekFrom::Start(lump.offset as u64))?;
        let id = self.reader.read_le::<[u8; 4]>()?;
        if lump.length >= 4 && &id == b"LZMA" {
            let actual_size = self.reader.read_le::<u32>()?;
            let lzma_size = self.reader.read_le::<u32>()?;
            let lzma_properties: [u8; 5] = self.reader.read_le()?;

            let mut fixed_lump = Cursor::new(Vec::with_capacity(lump.length as usize));
            fixed_lump.write_all(&lzma_properties)?;
            fixed_lump.write_le(&(actual_size as u64))?;
            let mut remaining_data = vec![0u8; lzma_size as usize];
            self.reader.read_exact(&mut remaining_data)?;
            fixed_lump.write_all(&remaining_data)?;

            let mut decompressed_data = Vec::new();
            fixed_lump.seek(SeekFrom::Start(0))?;
            lzma_rs::lzma_decompress(&mut fixed_lump, &mut decompressed_data)?;

            Ok(decompressed_data)
        } else {
            self.reader.seek(SeekFrom::Start(lump.offset as u64))?;
            let mut data = vec![0u8; lump.length as usize];
            self.reader.read_exact(&mut data)?;
            Ok(data)
        }
    }

    pub fn read_lump<'a, T>(&mut self, index: usize) -> anyhow::Result<Vec<T>>
    where
        T: BinRead,
        T::Args<'a>: Default,
    {
        let data = self.read_lump_raw(index)?;
        let mut cursor = Cursor::new(&data);
        let mut v = vec![];
        // TOOO(cohae): Might go wrong
        while cursor.position() < data.len() as u64 {
            v.push(cursor.read_le()?);
        }

        Ok(v)
    }
}

#[derive(BinRead, Debug)]
pub struct BspLumpHeader {
    pub offset: u32,
    pub length: u32,
    pub version: i32,
    pub fourcc: [u8; 4],
}

/// Fully parsed BSP file
///
/// NOTE: Due to it's size, the embedded pak file is not included in this struct. It can be obtained by calling [`BspFile::read_lump`] for lump 40
pub struct Bsp {
    pub planes: Vec<BspPlane>,
    pub vertices: Vec<[f32; 3]>,
    pub edges: Vec<[u16; 2]>,
    pub surfedges: Vec<i32>,
    pub faces: Vec<BspFace>,
    pub models: Vec<BspModel>,
    pub tex_info: Vec<BspTexInfo>,
    pub tex_data: Vec<BspTexData>,
    pub lightmap_data: Vec<BspColorRgbExp>,

    pub texdata_string_table: Vec<String>,
}

impl Bsp {
    pub fn parse<R: Read + Seek>(file: &mut BspFile<R>) -> anyhow::Result<Self> {
        let texdata_string_data = file.read_lump_raw(43)?;
        let texdata_string_offsets: Vec<u32> = file.read_lump(44)?;

        let texdata_string_table = texdata_string_offsets
            .iter()
            .map(|offset| {
                let data = &texdata_string_data[(*offset as usize)..];
                if let Some(size) = data.iter().position(|&c| c == 0) {
                    String::from_utf8_lossy(&data[0..size]).to_string()
                } else {
                    String::from("INVALID_STRING")
                }
            })
            .collect();

        Ok(Self {
            planes: file.read_lump(1)?,
            vertices: file.read_lump(3)?,
            edges: file.read_lump(12)?,
            surfedges: file.read_lump(13)?,
            faces: file.read_lump(7)?,
            models: file.read_lump(14)?,
            tex_info: file.read_lump(6)?,
            tex_data: file.read_lump(2)?,
            lightmap_data: file.read_lump(8)?,
            texdata_string_table,
        })
    }
}
