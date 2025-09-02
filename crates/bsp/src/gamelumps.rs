use binrw::binread;

#[binread]
#[derive(Debug, Clone)]
pub struct StaticPropDictLump {
    #[br(temp)]
    count: u32,
    #[br(count = count, temp)]
    names_raw: Vec<[u8; 128]>,
    #[br(calc(names_raw.iter().map(|s| String::from_utf8_lossy(s).trim_end_matches('\0').to_string()).collect()))]
    pub names: Vec<String>,
}

#[binread]
#[derive(Debug, Clone)]
pub struct StaticPropLeafLump {
    #[br(temp)]
    count: u32,
    #[br(count = count)]
    pub leaf: Vec<u16>,
}

#[binread]
#[derive(Debug, Clone)]
// #[br(import(version: u16))]
pub struct StaticPropLump {
    pub origin: [f32; 3],
    pub angles: [f32; 3],

    pub model_index: u16,
    pub first_leaf: u16,
    pub leaf_count: u16,
    pub solid: u8,
    // #[br(if(!matches!(version, 7)))]
    // pub flags_old: Option<u8>,

    // pub skin: i32,
    // pub fade_min_dist: f32,
    // pub fade_max_dist: f32,
    // pub lighting_origin: [f32; 3],

    // #[br(if(version >= 5))]
    // pub forced_fade_scale: f32,

    // #[br(if(matches!(version, 6 | 7)))]
    // pub min_dx_level: Option<u16>,
    // #[br(if(matches!(version, 6 | 7)))]
    // pub max_dx_level: Option<u16>,

    // #[br(if(matches!(version, 7)))]
    // pub flags: Option<u8>,
    // #[br(if(matches!(version, 7)))]
    // pub lightmap_res: [u16; 2],

    // #[br(if(version >= 8))]
    // pub min_cpu_level: Option<u8>,
    // #[br(if(version >= 8))]
    // pub max_cpu_level: Option<u8>,
    // #[br(if(version >= 8))]
    // pub min_gpu_level: Option<u8>,
    // #[br(if(version >= 8))]
    // pub max_gpu_level: Option<u8>,

    // #[br(if(version >= 7))]
    // pub diffuse_modulation: Option<u32>,

    // #[br(if(matches!(version, 9 | 10), true), map(|b: u8| b != 0))]
    // pub disable_x360: bool,

    // #[br(if(version >= 10))]
    // pub flags_ex: u32,

    // #[br(if(version >= 11, 1.0))]
    // pub uniform_scale: f32,
}
