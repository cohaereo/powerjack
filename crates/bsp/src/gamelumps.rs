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
pub struct StaticPropLump {
    pub origin: [f32; 3],
    pub angles: [f32; 3],
    pub model_index: u16,
    pub first_leaf: u16,
    pub leaf_count: u16,
    pub solid: u8,
    pub flags: u8,
    pub skin: i32,
    pub fade_min_dist: f32,
    pub fade_max_dist: f32,
    pub lighting_origin: [f32; 3],
    pub forced_fade_scale: f32,
    pub min_dx_level: u16,
    pub max_dx_level: u16,
}
