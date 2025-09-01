use binrw::{binread, BinRead};

#[derive(BinRead, Debug, Clone)]
pub struct BspFace {
    pub plane_num: u16,
    pub side: u8,
    pub on_node: u8,
    pub first_edge: i32,
    pub num_edges: i16,
    pub tex_info: i16,
    pub disp_info: i16,
    pub surface_fog_volume_id: i16,
    pub styles: [u8; 4],
    pub lightmap_data_offset: i32,
    pub area: f32,
    pub lightmap_mins: [i32; 2],
    pub lightmap_size: [i32; 2],
    pub orig_face: i32,
    pub num_primitives: u16,
    pub first_primitive: u16,
    pub smoothing_groups: u32,
}

#[derive(BinRead, Debug, Clone)]
pub struct BspModel {
    pub mins: [f32; 3],
    pub maxs: [f32; 3],
    pub origin: [f32; 3],
    pub head_node: i32,
    pub first_face: i32,
    pub num_faces: i32,
}

#[derive(BinRead, Debug, Clone)]
pub struct BspPlane {
    pub normal: [f32; 3],
    pub dist: f32,
    pub axis_type: i32,
}

#[derive(BinRead, Debug, Clone)]
pub struct BspTexInfo {
    pub texture_vecs: [[f32; 4]; 2],
    pub lightmap_vecs: [[f32; 4]; 2],
    pub flags: u32,
    pub tex_data: i32,
}

#[derive(BinRead, Debug, Clone)]
pub struct BspTexData {
    pub reflectivity: [f32; 3],
    /// Index into texdata_string_table
    pub name_index: i32,
    pub width: i32,
    pub height: i32,
    pub view_width: i32,
    pub view_height: i32,
}

#[derive(BinRead, Clone, Copy, Debug)]
pub struct BspColorRgbExp {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub exponent: i8,
}

impl BspColorRgbExp {
    pub fn to_srgb(&self) -> [u8; 3] {
        let rgb = self.to_rgb();
        [
            (rgb[0] * 255.0) as u8,
            (rgb[1] * 255.0) as u8,
            (rgb[2] * 255.0) as u8,
        ]
    }

    pub fn to_rgb(&self) -> [f32; 3] {
        [
            (self.r as f32 / 255.0) * 2f32.powi(self.exponent as i32),
            (self.b as f32 / 255.0) * 2f32.powi(self.exponent as i32),
            (self.g as f32 / 255.0) * 2f32.powi(self.exponent as i32),
        ]
    }
}

#[derive(BinRead, Debug, Clone)]
pub struct BspDispInfo {
    pub start_position: [f32; 3],
    pub disp_vert_start: i32,
    pub disp_tri_start: i32,
    pub power: i32,
    pub min_tess: i32,
    pub smoothing_angle: f32,
    pub contents: i32,
    pub map_face: u16,
    pub lightmap_alpha_start: i32,
    pub lightmap_sample_position_start: i32,
    pub neighbor_data: [u8; 90],
    pub allowed_verts: [u32; 10],
}

#[derive(BinRead, Debug, Clone)]
pub struct BspDispVert {
    pub vec: [f32; 3],
    pub dist: f32,
    pub alpha: f32,
}

#[derive(BinRead, Debug, Clone)]
pub struct BspDispTri {
    pub tags: u16,
}

#[binread]
#[derive(Debug, Clone)]
pub struct BspGameLumpHeader {
    #[br(temp)]
    count: u32,
    #[br(count = count)]
    pub lumps: Vec<BspGameLump>,
}

#[derive(BinRead, Clone, Copy, Debug)]
pub struct BspGameLump {
    pub id: u32,
    pub flags: u16,
    pub version: u16,
    pub fileofs: u32,
    pub filelen: u32,
}
