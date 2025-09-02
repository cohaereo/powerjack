use binrw::{BinRead, BinReaderExt, BinResult, Endian};
use bitflags::bitflags;
use std::io::{Read, Seek, SeekFrom};

#[derive(BinRead, Debug, Copy, Clone)]
#[repr(C, packed)]
pub struct BodyPartHeader {
    pub num_models: u32,
    pub model_offset: u32,
}

#[derive(BinRead, Debug, Copy, Clone)]
#[repr(C, packed)]
pub struct ModelHeader {
    pub num_lods: u32,
    pub lod_offset: u32,
}

#[derive(BinRead, Debug, Copy, Clone)]
#[repr(C, packed)]
pub struct ModelLodHeader {
    pub num_meshes: u32,
    pub mesh_offset: u32,
    pub switch_point: f32,
}

#[derive(BinRead, Debug, Copy, Clone)]
#[repr(C, packed)]
pub struct MeshHeader {
    pub num_stripgroups: u32,
    pub stripgroup_header_offset: u32,
    pub flags: u8,
}

#[derive(BinRead, Debug, Copy, Clone)]
#[repr(C, packed)]
pub struct StripGroupHeader {
    pub num_verts: i32,
    pub vert_offset: i32,
    pub num_indices: i32,
    pub index_offset: i32,
    pub num_strips: i32,
    pub strip_offset: i32,
    pub flags: u8,
}

#[derive(BinRead, Debug, Copy, Clone)]
#[repr(C, packed)]
pub struct StripHeader {
    pub num_indices: i32,
    pub index_offset: i32,
    pub num_verts: i32,
    pub vert_offset: i32,
    pub num_bones: i16,
    pub flags: StripFlags,
    pub num_bone_state_changes: i32,
    pub bone_state_change_offset: i32,
}

#[derive(BinRead, Debug, Copy, Clone)]
#[repr(C, packed)]
pub struct VtxVertex {
    pub bone_weight_index: [u8; 3],
    pub num_bones: u8,
    pub orig_mesh_vert_id: u16,
    pub bone_ids: [i8; 3],
}

bitflags! {
    #[derive(Debug, Copy, Clone)]
    pub struct StripFlags: u8 {
        const IS_TRILIST = 0x1;
        const IS_TRISTRIP = 0x2;
    }
}

impl BinRead for StripFlags {
    type Args<'a> = ();

    fn read_options<R: Read + Seek>(
        reader: &mut R,
        endian: Endian,
        _args: Self::Args<'_>,
    ) -> BinResult<Self> {
        Ok(StripFlags::from_bits_truncate(
            reader.read_type::<u8>(endian)?,
        ))
    }
}

pub type BodyPartList = Vec<ModelList>;
pub type ModelList = Vec<(ModelHeader, LodList)>;
pub type LodList = Vec<(ModelLodHeader, MeshList)>;
pub type MeshList = Vec<(MeshHeader, StripGroupList)>;
pub type StripGroupList = Vec<StripGroup>;

pub struct StripGroup {
    pub header: StripGroupHeader,
    pub indices: Vec<u16>,
    pub vertices: Vec<VtxVertex>,
    pub strips: Vec<StripHeader>,
}

pub struct VtxData {
    pub header: VtxHeader,
    /// Bodyparts -> Models -> Lods -> Meshes -> Stripgroups -> Strips
    pub body_parts: BodyPartList,
}

#[derive(BinRead, Debug, Copy, Clone)]
pub struct VtxHeader {
    pub version: i32,
    pub vert_cache_size: i32,
    pub max_bones_per_stip: u16,
    pub max_bones_per_tri: u16,
    pub max_bones_per_vert: i32,
    pub checksum: u32,
    pub num_lods: i32,
    pub material_replacement_list_offset: i32,
    pub num_body_parts: i32,
    pub body_part_offset: i32,
}

impl VtxData {
    pub fn parse<R: Read + Seek>(input: &mut R) -> anyhow::Result<Self> {
        let header = input.read_le::<VtxHeader>()?;

        let mut vtx_data = Self {
            header,
            body_parts: BodyPartList::new(),
        };

        // Body parts
        input.seek(SeekFrom::Start(header.body_part_offset as u64))?;
        for _ in 0..header.num_body_parts {
            let save_pos = input.stream_position()?;
            let body_part = input.read_le::<BodyPartHeader>()?;

            // Models
            input.seek(SeekFrom::Start(save_pos + body_part.model_offset as u64))?;
            let mut models: ModelList = vec![];
            for _ in 0..body_part.num_models {
                let save_pos = input.stream_position()?;
                let model = input.read_le::<ModelHeader>()?;

                // LODs
                input.seek(SeekFrom::Start(save_pos + model.lod_offset as u64))?;
                let mut lods: LodList = vec![];
                for _ in 0..model.num_lods {
                    let save_pos = input.stream_position()?;
                    let lod = input.read_le::<ModelLodHeader>()?;

                    // Meshes
                    input.seek(SeekFrom::Start(save_pos + lod.mesh_offset as u64))?;
                    let mut meshes: MeshList = vec![];
                    for _ in 0..lod.num_meshes {
                        let save_pos = input.stream_position()?;
                        let mesh = input.read_le::<MeshHeader>()?;

                        // Stripgroup
                        input.seek(SeekFrom::Start(
                            save_pos + mesh.stripgroup_header_offset as u64,
                        ))?;

                        let mut stripgroups: StripGroupList = vec![];
                        for _ in 0..mesh.num_stripgroups {
                            let save_pos = input.stream_position()?;
                            let stripgroup = input.read_le::<StripGroupHeader>()?;

                            let mut indices = vec![];
                            input
                                .seek(SeekFrom::Start(save_pos + stripgroup.index_offset as u64))?;
                            for _ in 0..stripgroup.num_indices {
                                let index = input.read_le::<u16>()?;
                                indices.push(index);
                            }

                            let mut vertices = vec![];
                            input
                                .seek(SeekFrom::Start(save_pos + stripgroup.vert_offset as u64))?;
                            for _ in 0..stripgroup.num_verts {
                                let vert = input.read_le::<VtxVertex>()?;
                                vertices.push(vert);
                            }

                            // Strips
                            input
                                .seek(SeekFrom::Start(save_pos + stripgroup.strip_offset as u64))?;
                            let mut strips = vec![];
                            for _ in 0..stripgroup.num_strips {
                                let save_pos = input.stream_position()?;
                                let strip = input.read_le::<StripHeader>()?;
                                strips.push(strip);

                                input.seek(SeekFrom::Start(
                                    save_pos + std::mem::size_of::<StripHeader>() as u64,
                                ))?;
                            }

                            // stripgroups.push((stripgroup, indices, vertices, strips));
                            stripgroups.push(StripGroup {
                                header: stripgroup,
                                indices,
                                vertices,
                                strips,
                            });

                            input.seek(SeekFrom::Start(
                                save_pos + std::mem::size_of::<StripGroupHeader>() as u64,
                            ))?;
                        }

                        meshes.push((mesh, stripgroups));

                        input.seek(SeekFrom::Start(
                            save_pos + std::mem::size_of::<MeshHeader>() as u64,
                        ))?;
                    }

                    lods.push((lod, meshes));

                    input.seek(SeekFrom::Start(
                        save_pos + std::mem::size_of::<ModelLodHeader>() as u64,
                    ))?;
                }

                models.push((model, lods));

                input.seek(SeekFrom::Start(
                    save_pos + std::mem::size_of::<ModelHeader>() as u64,
                ))?;
            }

            vtx_data.body_parts.push(models);

            input.seek(SeekFrom::Start(
                save_pos + std::mem::size_of::<BodyPartHeader>() as u64,
            ))?;
        }

        Ok(vtx_data)
    }
}
