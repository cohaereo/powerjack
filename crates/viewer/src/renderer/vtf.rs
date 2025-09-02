use anyhow::Context;
use binrw::BinReaderExt;
use powerjack_vtf::{VtfHeader, VtfTextureFormat};
use std::io::{Cursor, Read, Seek, SeekFrom};
use wgpu::util::DeviceExt;

use crate::{
    fs::SharedFilesystem, renderer::iad::InstanceAdapterDevice, util::ensure_path_has_extension,
};

pub fn load_vtf(
    fs: &SharedFilesystem,
    iad: &InstanceAdapterDevice,
    path: &str,
) -> anyhow::Result<(wgpu::Texture, wgpu::TextureView)> {
    let path = ensure_path_has_extension(path, "vtf");
    let Some(vtf_data) = fs
        .lock()
        .read_path(&path)
        .context("Failed to read VTF texture data")?
    else {
        anyhow::bail!("VTF file does not exist");
    };

    let mut cur = Cursor::new(vtf_data);
    let (data, format, width, height) = load_vtf_data(&mut cur)?;

    let texture = iad.create_texture_with_data(
        &iad.queue,
        &wgpu::TextureDescriptor {
            label: Some(&path),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[format],
        },
        wgpu::wgt::TextureDataOrder::LayerMajor,
        &data,
    );
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

    Ok((texture, view))
}

pub fn load_vtf_data<R: Read + Seek>(
    c: &mut R,
) -> anyhow::Result<(Vec<u8>, wgpu::TextureFormat, u32, u32)> {
    let vtf: VtfHeader = c.read_le()?;
    let offset = vtf
        .calculate_data_offset(0)
        .context("Missing high-res image")?;
    let fmt = vtf.high_res_image_format;
    let mut data = vec![0; fmt.data_size(vtf.width as _, vtf.height as _, 1) as usize];
    c.seek(SeekFrom::Start(offset as u64))?;
    c.read_exact(&mut data)?;

    Ok((
        data,
        vtf_texture_format_to_wgpu(fmt)
            .with_context(|| format!("Failed to convert VTF image format {fmt:?} to WGPU"))?,
        vtf.width as u32,
        vtf.height as u32,
    ))
}

pub fn vtf_texture_format_to_wgpu(fmt: VtfTextureFormat) -> Option<wgpu::TextureFormat> {
    Some(match fmt {
        VtfTextureFormat::None => return None,
        VtfTextureFormat::Rgba8888 => wgpu::TextureFormat::Rgba8UnormSrgb,
        VtfTextureFormat::Abgr8888 => return None,
        VtfTextureFormat::Rgb888 => return None,
        VtfTextureFormat::Bgr888 => return None,
        VtfTextureFormat::Rgb565 => return None,
        VtfTextureFormat::I8 => wgpu::TextureFormat::R8Unorm,
        VtfTextureFormat::Ia88 => wgpu::TextureFormat::Rg8Unorm,
        VtfTextureFormat::P8 => return None,
        VtfTextureFormat::A8 => wgpu::TextureFormat::R8Unorm,
        VtfTextureFormat::Rgb888Bluescreen => return None,
        VtfTextureFormat::Bgr888Bluescreen => return None,
        VtfTextureFormat::Argb8888 => return None,
        VtfTextureFormat::Bgra8888 => wgpu::TextureFormat::Bgra8UnormSrgb,
        VtfTextureFormat::Dxt1 => wgpu::TextureFormat::Bc1RgbaUnormSrgb,
        VtfTextureFormat::Dxt3 => wgpu::TextureFormat::Bc2RgbaUnormSrgb,
        VtfTextureFormat::Dxt5 => wgpu::TextureFormat::Bc3RgbaUnormSrgb,
        VtfTextureFormat::Bgrx8888 => wgpu::TextureFormat::Bgra8UnormSrgb,
        VtfTextureFormat::Bgr565 => return None,
        VtfTextureFormat::Bgrx5551 => return None,
        VtfTextureFormat::Bgra4444 => return None,
        VtfTextureFormat::Dxt1A => wgpu::TextureFormat::Bc1RgbaUnormSrgb,
        VtfTextureFormat::Bgra5551 => return None,
        VtfTextureFormat::Uv88 => return None,
        VtfTextureFormat::Uvwq8888 => return None,
        VtfTextureFormat::Rgba16161616F => wgpu::TextureFormat::Rgba16Float,
        VtfTextureFormat::Rgba16161616 => wgpu::TextureFormat::Rgba16Unorm,
        VtfTextureFormat::Uvlx8888 => return None,
    })
}

pub fn create_pink_checkerboard(width: u32, height: u32, color: [u8; 3]) -> Vec<u8> {
    let mut data = vec![0; (width * height * 4) as usize];
    for y in 0..height {
        for x in 0..width {
            let i = (y * width + x) as usize;
            let grid_x = x.div_ceil(4);
            let grid_y = y.div_ceil(4);
            let even_row = grid_y % 2 == 0;
            if (grid_x % 2 == 0 && even_row) || (grid_x % 2 == 1 && !even_row) {
                data[i * 4] = color[0];
                data[i * 4 + 1] = color[1];
                data[i * 4 + 2] = color[2];
            }
            data[i * 4 + 3] = 255;
        }
    }
    data
}

pub fn create_fallback_texture(
    iad: &InstanceAdapterDevice,
    color: [u8; 3],
) -> (wgpu::Texture, wgpu::TextureView) {
    let data = create_pink_checkerboard(32, 32, color);
    let texture = iad.create_texture_with_data(
        &iad.queue,
        &wgpu::TextureDescriptor {
            label: Some("Fallback Texture"),
            size: wgpu::Extent3d {
                width: 32,
                height: 32,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[wgpu::TextureFormat::Rgba8UnormSrgb],
        },
        wgpu::wgt::TextureDataOrder::LayerMajor,
        &data,
    );

    let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());

    (texture, texture_view)
}
