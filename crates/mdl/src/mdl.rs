use binrw::BinReaderExt;
use binrw::FilePtr32;
use binrw::NullString;
use binrw::PosValue;
use binrw::binread;
use binrw::file_ptr::FilePtrArgs;
use std::io::SeekFrom;
use std::io::{Read, Seek};

use crate::mdl;

#[binread]
#[br(magic = b"IDST")]
#[derive(Debug, Clone)]
pub struct StudioHeader {
    #[br(assert((44..=48).contains(&version)))]
    pub version: u32,
    pub checksum: u32,

    #[br(map(|b: [u8; 64]| String::from_utf8_lossy(&b).trim_end_matches('\0').to_owned()))]
    pub name: String,

    pub data_length: u32,

    pub eye_position: [f32; 3],
    pub illum_position: [f32; 3],
    pub hull_min: [f32; 3],
    pub hull_max: [f32; 3],
    pub view_bbmin: [f32; 3],
    pub view_bbmax: [f32; 3],

    pub flags: u32,

    pub num_bones: u32,
    pub bone_offset: i32,

    pub num_bone_controllers: u32,
    pub bone_controller_offset: i32,

    pub num_hitbox_sets: u32,
    pub hitbox_set_offset: i32,

    pub num_local_animations: u32,
    pub local_animation_offset: i32,

    pub num_local_sequences: u32,
    pub local_sequence_offset: i32,

    pub activity_list_version: u32,
    pub events_indexed: u32,

    pub num_textures: u32,
    pub texture_offset: i32,

    pub num_texture_dirs: u32,
    pub texture_dir_offset: i32,

    pub num_skin_ref: u32,
    pub num_skin_families: u32,
    pub skin_offset: i32,

    pub num_body_parts: u32,
    pub body_part_offset: i32,
    // TODO: attachments, localnodes, flexdescs, flexcontrollers, flexrules, ikchains, mouths, localposeparams, surfaceprops, keyvalues, iklocks, includemodels, etc
}

#[binread]
#[derive(Debug, Clone)]
pub struct StudioBodyPart {
    // pub name: String,
    pub name_offset: i32,
    pub num_models: u32,
    pub base: u32,
    pub model_offset: i32,
}

#[binread]
#[derive(Debug, Clone)]
pub struct StudioModel {
    #[br(map(|b: [u8; 64]| String::from_utf8_lossy(&b).trim_end_matches('\0').to_owned()))]
    pub name: String,
    pub kind: u32,
    pub bounding_radius: f32,

    pub num_meshes: u32,
    pub mesh_offset: i32,

    pub num_vertices: u32,
    pub vertex_offset: i32,
    pub tangent_offset: i32,

    pub num_attachments: u32,
    pub attachment_offset: i32,

    pub num_eyeballs: u32,
    pub eyeball_offset: i32,

    #[br(temp)]
    _unused: [u32; 8 + 4],
}

pub const MAX_NUM_LODS: usize = 8;
#[binread]
#[derive(Debug, Clone)]
pub struct StudioMesh {
    pub material: i32,
    pub model_index: i32,
    pub num_vertices: u32,
    pub vertex_offset: i32,
    pub num_flexes: u32,
    pub flex_offset: i32,
    pub material_type: u32,
    pub material_param: u32,
    pub mesh_id: i32,
    pub center: [f32; 3],

    #[br(temp)]
    _unused: [u32; 8 + 1 + MAX_NUM_LODS],
}
#[binread]
#[derive(Debug, Clone)]
pub struct StudioTexture {
    #[br(temp)]
    base: PosValue<()>,

    #[br(temp, offset = base.pos)]
    raw_name: FilePtr32<NullString>,

    #[br(calc(raw_name.to_string()))]
    pub name: String,

    pub flags: u32,
    pub used: u32,
    #[br(temp)]
    _unused1: u32,

    #[br(temp)]
    _unused: [u32; 10 + 2],
}

#[derive(Debug, Clone)]
pub struct MdlData {
    pub header: StudioHeader,

    pub body_parts: Vec<(StudioBodyPart, Vec<(StudioModel, Vec<StudioMesh>)>)>,
    pub textures: Vec<StudioTexture>,
    pub texture_dirs: Vec<String>,
}

impl MdlData {
    pub fn parse<R: Read + Seek>(input: &mut R) -> anyhow::Result<Self> {
        let header = input.read_le::<StudioHeader>()?;

        let mut mdl_data = Self {
            header: header.clone(),
            body_parts: Vec::new(),
            textures: Vec::new(),
            texture_dirs: Vec::new(),
        };

        // Body parts
        input.seek(SeekFrom::Start(header.body_part_offset as u64))?;
        for _ in 0..header.num_body_parts {
            let struct_pos = input.stream_position()?;
            let body_part = input.read_le::<StudioBodyPart>()?;

            // Models
            let save_pos = input.stream_position()?;
            input.seek(SeekFrom::Start(struct_pos + body_part.model_offset as u64))?;
            let mut models = vec![];
            for _ in 0..body_part.num_models {
                let struct_pos = input.stream_position()?;
                let model = input.read_le::<StudioModel>()?;

                // Meshes
                let save_pos = input.stream_position()?;
                input.seek(SeekFrom::Start(struct_pos + model.mesh_offset as u64))?;
                let mut meshes = vec![];
                for _ in 0..model.num_meshes {
                    // let struct_pos = input.stream_position()?;
                    let mesh = input.read_le::<StudioMesh>()?;

                    meshes.push(mesh);
                }

                models.push((model, meshes));
                input.seek(SeekFrom::Start(save_pos))?;
            }

            mdl_data.body_parts.push((body_part, models));
            input.seek(SeekFrom::Start(save_pos))?;
        }

        input.seek(SeekFrom::Start(header.texture_offset as u64))?;
        for _ in 0..header.num_textures {
            mdl_data.textures.push(input.read_le()?);
        }

        input.seek(SeekFrom::Start(header.texture_dir_offset as u64))?;
        for _ in 0..header.num_texture_dirs {
            let n = input.read_le_args::<FilePtr32<NullString>>(
                FilePtrArgs::builder().offset(0).finalize(),
            )?;
            mdl_data.texture_dirs.push(n.to_string());
        }

        Ok(mdl_data)
    }
}
