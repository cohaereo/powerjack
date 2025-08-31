use binrw::BinRead;
use std::io::SeekFrom;

#[derive(BinRead, Debug)]
pub struct VtfResourceDictionary {
    pub resource_count: u32,
    pub resource_offset: u32,
}

#[derive(BinRead, Debug)]
#[br(magic = b"VTF\0")]
pub struct VtfHeader {
    #[br(assert(version[0] == 7 && version[1] <= 4, "Version must be between 7.0 and 7.4"))]
    pub version: [u32; 2],
    /// Size of the header struct  (16 byte aligned; currently 80 bytes) + size of the resources dictionary (7.3+).
    pub header_size: u32,
    /// Width of the largest mipmap in pixels. Must be a power of 2.
    pub width: u16,
    /// Height of the largest mipmap in pixels. Must be a power of 2.
    pub height: u16,

    // 0x10
    pub flags: u32,
    pub frames: u16,
    pub first_frame: u16,

    // 0x20
    #[br(seek_before(SeekFrom::Start(0x20)))]
    pub reflectivity: [f32; 3],

    // 0x30
    #[br(seek_before(SeekFrom::Start(0x30)))]
    pub bumpmap_scale: f32,
    pub high_res_image_format: VtfTextureFormat,
    pub mipmap_count: u8,
    pub low_res_image_format: VtfTextureFormat,
    pub low_res_image_width: u8,
    pub low_res_image_height: u8,

    #[br(if(version[1] >= 2))]
    pub depth: Option<u16>,

    #[br(if(version[1] >= 3), seek_before(SeekFrom::Start(0x44)))]
    pub num_resources: Option<u32>,

    #[br(count = num_resources.unwrap_or_default()  , seek_before(SeekFrom::Start(0x50)))]
    pub resources: Vec<VtfResource>,
}

impl VtfHeader {
    /// May return None for 7.3+ VTFs that are missing the low-res image tag
    pub fn low_res_image_offset(&self) -> Option<u32> {
        if self.resources.is_empty() {
            Some(self.header_size)
        } else {
            self.get_resource_offset(VtfResource::TAG_LOWRES)
        }
    }

    /// May return None for 7.3+ VTFs that are missing the high-res image tag
    pub fn high_res_image_base_offset(&self) -> Option<u32> {
        if self.resources.is_empty() {
            Some(
                self.header_size
                    + self.low_res_image_format.data_size(
                        self.low_res_image_width as _,
                        self.low_res_image_height as _,
                        1,
                    ),
            )
        } else {
            self.get_resource_offset(VtfResource::TAG_HIGHRES)
        }
    }

    /// Returns the absolute offset to the specified mip. Returns the smallest mip if `mip` is larger than the number of mips in the VTF
    pub fn calculate_data_offset(&self, mut mip: u32) -> Option<u32> {
        mip = mip.clamp(0, self.mipmap_count as u32 - 1);

        let mut offset = 0;
        for i in (mip + 1..self.mipmap_count as u32).rev() {
            offset += self.calculate_mip_size(i as usize) * self.frames as u32;
        }

        Some(self.high_res_image_base_offset()? + offset)
    }

    pub fn calculate_mip_size(&self, mip: usize) -> u32 {
        let width = (self.width as u32) >> mip.max(1);
        let height = (self.height as u32) >> mip.max(1);
        self.high_res_image_format
            .data_size(width, height, self.depth.unwrap_or(1) as u32)
    }

    pub fn get_resource_offset(&self, tag: [u8; 3]) -> Option<u32> {
        self.resources
            .iter()
            .find(|r| r.tag == tag)
            .map(|r| r.offset)
    }

    // pub fn calculate_max_mip(&self) -> usize {
    //     for mip in 0..self.mipmap_count as usize {
    //         let width = (self.width as u32) >> mip;
    //         let height = (self.height as u32) >> mip;
    //         if width == 0 || height == 0 {
    //             return mip.saturating_sub(1);
    //         }
    //     }
    //
    //     self.mipmap_count as _
    // }
}

#[derive(BinRead, Debug)]
pub struct VtfResource {
    pub tag: [u8; 3],
    pub flags: u8,
    pub offset: u32,
}

impl VtfResource {
    pub const TAG_LOWRES: [u8; 3] = *b"\x01\x00\x00";
    pub const TAG_HIGHRES: [u8; 3] = *b"\x30\x00\x00";
    pub const TAG_PARTICLESHEET: [u8; 3] = *b"\x10\x00\x00";
    pub const TAG_CRC: [u8; 3] = *b"CRC";
    pub const TAG_LOD: [u8; 3] = *b"LOD";
    pub const TAG_EXT: [u8; 3] = *b"TSO";
    pub const TAG_KV: [u8; 3] = *b"KVD";
}

#[derive(BinRead, Debug)]
#[br(repr(u32))]
pub enum VtfTextureFormat {
    None = -1,
    Rgba8888 = 0,
    Abgr8888,
    Rgb888,
    Bgr888,
    Rgb565,
    I8,
    Ia88,
    P8,
    A8,
    Rgb888Bluescreen,
    Bgr888Bluescreen,
    Argb8888,
    Bgra8888,
    Dxt1,
    Dxt3,
    Dxt5,
    Bgrx8888,
    Bgr565,
    Bgrx5551,
    Bgra4444,
    Dxt1A,
    Bgra5551,
    Uv88,
    Uvwq8888,
    Rgba16161616F,
    Rgba16161616,
    Uvlx8888,
}

impl VtfTextureFormat {
    pub fn bpp(&self) -> u32 {
        match self {
            VtfTextureFormat::None => 0,
            VtfTextureFormat::Rgba8888 => 32,
            VtfTextureFormat::Abgr8888 => 32,
            VtfTextureFormat::Rgb888 => 24,
            VtfTextureFormat::Bgr888 => 24,
            VtfTextureFormat::Rgb565 => 16,
            VtfTextureFormat::I8 => 8,
            VtfTextureFormat::Ia88 => 16,
            VtfTextureFormat::P8 => 8,
            VtfTextureFormat::A8 => 8,
            VtfTextureFormat::Rgb888Bluescreen => 24,
            VtfTextureFormat::Bgr888Bluescreen => 24,
            VtfTextureFormat::Argb8888 => 32,
            VtfTextureFormat::Bgra8888 => 32,
            VtfTextureFormat::Dxt1 => 4,
            VtfTextureFormat::Dxt3 => 8,
            VtfTextureFormat::Dxt5 => 8,
            VtfTextureFormat::Bgrx8888 => 32,
            VtfTextureFormat::Bgr565 => 16,
            VtfTextureFormat::Bgrx5551 => 16,
            VtfTextureFormat::Bgra4444 => 16,
            VtfTextureFormat::Dxt1A => 4,
            VtfTextureFormat::Bgra5551 => 16,
            VtfTextureFormat::Uv88 => 16,
            VtfTextureFormat::Uvwq8888 => 32,
            VtfTextureFormat::Rgba16161616F => 64,
            VtfTextureFormat::Rgba16161616 => 64,
            VtfTextureFormat::Uvlx8888 => 32,
        }
    }

    pub fn data_size(&self, width: u32, height: u32, depth: u32) -> u32 {
        match self {
            VtfTextureFormat::Dxt1A | VtfTextureFormat::Dxt1 => {
                width.div_ceil(4) * height.div_ceil(4) * 8
            }
            VtfTextureFormat::Dxt3 | VtfTextureFormat::Dxt5 => {
                width.div_ceil(4) * height.div_ceil(4) * 16
            }
            _ => (self.bpp() * width * height * depth) / 8,
        }
    }
}
