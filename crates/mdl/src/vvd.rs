use binrw::{BinRead, BinReaderExt};
use std::io::{Read, Seek, SeekFrom};

const MAX_NUM_LODS: usize = 8;
const MAX_NUM_BONES_PER_VERT: usize = 3;

pub struct VvdData {
    pub header: VvdHeader,
    pub vertices: Vec<StudioVertex>,
    pub fixups: Vec<VvdFixup>,
}

#[derive(BinRead, Debug, Copy, Clone)]
pub struct VvdHeader {
    pub id: u32,
    pub version: i32,
    pub checksum: u32,
    pub num_lods: u32,
    pub num_lod_vertexes: [u32; MAX_NUM_LODS],
    pub num_fixups: u32,
    pub fixup_table_start: u32,
    pub vertex_data_start: u32,
    pub tangent_data_start: u32,
}

#[derive(BinRead, Debug, Copy, Clone)]
pub struct VvdFixup {
    pub lod: i32,
    pub source_vertex_id: u32,
    pub num_vertices: u32,
}

#[derive(BinRead, Debug, Copy, Clone)]
pub struct StudioBoneWeight {
    pub weight: [f32; MAX_NUM_BONES_PER_VERT],
    pub bone: [u8; MAX_NUM_BONES_PER_VERT],
    pub num_bones: u8,
}
#[derive(BinRead, Debug, Copy, Clone)]
pub struct StudioVertex {
    pub bone_weights: StudioBoneWeight,
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub uv: [f32; 2],
}

impl VvdData {
    pub fn parse<R: Read + Seek>(input: &mut R) -> eyre::Result<Self> {
        let header = input.read_le::<VvdHeader>()?;
        input.seek(SeekFrom::Start(header.vertex_data_start as u64))?;
        let vertices = input.read_le_args(
            binrw::VecArgs::builder()
                .count(header.num_lod_vertexes[0] as usize)
                .finalize(),
        )?;
        input.seek(SeekFrom::Start(header.fixup_table_start as u64))?;
        let fixups = input.read_le_args(
            binrw::VecArgs::builder()
                .count(header.num_fixups as usize)
                .finalize(),
        )?;

        Ok(VvdData {
            header,
            vertices,
            fixups,
        })
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn check_struct_sizes() {
        use super::*;
        assert_eq!(size_of::<VvdHeader>(), 64);
        assert_eq!(size_of::<StudioBoneWeight>(), 16);
        assert_eq!(size_of::<StudioVertex>(), 48);
    }
}
