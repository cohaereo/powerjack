use binrw::BinRead;

#[derive(BinRead, Debug, Clone)]
// #[repr(C, packed)]
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
#[repr(C, packed)]
pub struct BspModel {
    pub mins: [f32; 3],
    pub maxs: [f32; 3],
    pub origin: [f32; 3],
    pub head_node: i32,
    pub first_face: i32,
    pub num_faces: i32,
}

#[derive(BinRead, Debug, Clone)]
#[repr(C, packed)]
pub struct BspPlane {
    pub normal: [f32; 3],
    pub dist: f32,
    pub axis_type: i32,
}

#[derive(BinRead, Debug, Clone)]
#[repr(C, packed)]
pub struct BspTexInfo {
    pub texture_vecs: [[f32; 4]; 2],
    pub lightmap_vecs: [[f32; 4]; 2],
    pub flags: u32,
    pub tex_data: i32,
}

#[derive(BinRead, Debug, Clone)]
#[repr(C, packed)]
pub struct BspTexData {
    pub reflectivity: [f32; 3],
    /// Index into texdata_string_table
    pub name_index: i32,
    pub width: i32,
    pub height: i32,
    pub view_width: i32,
    pub view_height: i32,
}

// pub struct BspData {
//     pub planes: Vec<BspPlane>,
//     pub vertices: Vec<[f32; 3]>,
//     pub edges: Vec<[u16; 2]>,
//     pub surfedges: Vec<i32>,
//     pub faces: Vec<BspFace>,
//     pub models: Vec<BspModel>,
//     pub tex_infos: Vec<BspTexInfo>,
//     pub tex_datas: Vec<BspTexData>,
// }
//
// impl BspData {
//     pub fn parse<R: Read + Seek>(input: &mut R) -> anyhow::Result<Self> {
//         let header = input.read_u32::<LittleEndian>()?;
//         let version = input.read_u32::<LittleEndian>()?;
//
//         let mut bsp_data = BspData {
//             planes: Vec::new(),
//             vertices: Vec::new(),
//             edges: Vec::new(),
//             surfedges: Vec::new(),
//             faces: Vec::new(),
//             models: Vec::new(),
//             tex_infos: Vec::new(),
//             tex_datas: Vec::new(),
//         };
//
//         let mut lumps: Vec<BspLumpRaw> = Vec::new();
//         for i in 0..64 {
//             let offset = input.read_u32::<LittleEndian>()?;
//             let length = input.read_u32::<LittleEndian>()?;
//             let version = input.read_u32::<LittleEndian>()?;
//             let fourcc = input.read_u32::<LittleEndian>()?;
//             assert!(matches!(version, 19 | 20));
//
//             let save_pos = input.stream_position()?;
//
//             // Skip anything that's not geometry FOR NOW
//             if i != 1
//                 && i != 3
//                 && i != 7
//                 && i != 12
//                 && i != 13
//                 && i != 14
//                 && i != 2
//                 && i != 6
//                 && i != 40
//             {
//                 continue;
//             }
//
//             input.seek(SeekFrom::Start(offset as u64))?;
//             let mut data = vec![0; length as usize];
//             input.read_exact(&mut data)?;
//
//             // Painful to look at, i know
//             let data =
//                 if data.len() >= 4 && data[..4] == ['L' as u8, 'Z' as u8, 'M' as u8, 'A' as u8] {
//                     let mut cursor = Cursor::new(&data);
//
//                     cursor.read_u32::<LittleEndian>()?; // LZMA id
//                     let actual_size = cursor.read_u32::<LittleEndian>()?;
//                     let lzma_size = cursor.read_u32::<LittleEndian>()?;
//                     let mut lzma_properties = vec![0u8; 5];
//                     cursor.read_exact(&mut lzma_properties)?;
//
//                     println!("Lump {:02} compressed: YES", i);
//
//                     let mut fixed_lump = Vec::new();
//                     fixed_lump.write_all(&lzma_properties)?;
//                     fixed_lump.write_u64::<LittleEndian>(actual_size as u64)?;
//                     let pos = cursor.position() as usize;
//                     fixed_lump.write_all(&data[pos..])?;
//
//                     let mut decompressed_data = Vec::new();
//
//                     // TODO: Don't stop parsing when single lump fails
//                     let mut buftmp = BufReader::new(Cursor::new(fixed_lump)); // God help us
//                     lzma_rs::lzma_decompress(&mut buftmp, &mut decompressed_data)?;
//
//                     decompressed_data
//                 } else {
//                     println!("Lump {:02} compressed: NO", i);
//                     // let mut f = File::create(format!("lump_{i}.bin"))?;
//                     // f.write_all(data.as_slice())?;
//                     data
//                 };
//
//             if i == 40 {
//                 let mut f = File::create(format!("data.zip"))?;
//                 f.write_all(data.as_slice())?;
//             }
//
//             let data_len = data.len();
//             bsp_data.parse_raw_lump(&mut Cursor::new(data), data_len, i);
//
//             // let mut f = File::create(format!("lump_{i}.bin"))?;
//             // f.write(data.as_slice())?;
//             // lumps.push(BspLumpRaw { data });
//
//             input.seek(SeekFrom::Start(save_pos))?;
//         }
//
//         Ok(bsp_data)
//     }
//
//     fn parse_raw_lump<R: Read + Seek>(&mut self, input: &mut R, len: usize, id: i32) {
//         match id {
//             1 => {
//                 self.planes.clear();
//                 for _ in 0..len / std::mem::size_of::<BspPlane>() {
//                     if let Ok(plane) = input.read_le() {
//                         self.planes.push(plane);
//                     }
//                 }
//             }
//             3 => {
//                 self.vertices.clear();
//                 for _ in 0..len / 12 {
//                     if let Ok(vert) = input.read_le() {
//                         self.vertices.push(vert);
//                     }
//                 }
//             }
//             2 => {
//                 self.tex_datas.clear();
//                 for _ in 0..len / std::mem::size_of::<BspTexData>() {
//                     if let Ok(tex_data) = input.read_le() {
//                         self.tex_datas.push(tex_data);
//                     }
//                 }
//             }
//             6 => {
//                 self.tex_infos.clear();
//                 for _ in 0..len / std::mem::size_of::<BspTexInfo>() {
//                     if let Ok(tex_info) = input.read_le() {
//                         self.tex_infos.push(tex_info);
//                     }
//                 }
//             }
//             7 => {
//                 self.faces.clear();
//                 for _ in 0..len / std::mem::size_of::<BspFace>() {
//                     if let Ok(face) = input.read_le() {
//                         self.faces.push(face);
//                     }
//                 }
//             }
//             12 => {
//                 self.edges.clear();
//                 for _ in 0..len / 4 {
//                     if let Ok(edge) = input.read_le() {
//                         self.edges.push(edge);
//                     }
//                 }
//             }
//             13 => {
//                 self.surfedges.clear();
//                 for _ in 0..len / 4 {
//                     if let Ok(edge) = input.read_le() {
//                         self.surfedges.push(edge);
//                     }
//                 }
//             }
//             14 => {
//                 self.models.clear();
//                 for _ in 0..len / std::mem::size_of::<BspModel>() {
//                     if let Ok(model) = input.read_le() {
//                         self.models.push(model);
//                     }
//                 }
//             }
//             40 => {}
//             _ => println!("Unknown lump: {}", id),
//         }
//     }
// }

#[derive(BinRead, Clone, Copy, Debug)]
#[repr(C, packed)]
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
