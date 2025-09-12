use std::{
    f32,
    io::{Read, Seek},
    num::NonZeroU32,
    ops::Range,
};

use anyhow::Context;
use bitflags::bitflags;
use bytemuck::{Pod, Zeroable};
use glam::{IVec2, Mat4, Vec2, Vec3, Vec4, Vec4Swizzles, vec2};
use powerjack_bsp::{Bsp, BspFile, lumps::BspFace};
use serde::Deserialize;
use wgpu::util::DeviceExt;

use crate::renderer::{
    Renderer,
    iad::InstanceAdapterDevice,
    reloadable_pipeline::{ReloadablePipeline, ShaderSource},
    vmt::get_basetexture_for_vmt,
    vtf::{create_fallback_texture, load_vtf},
};

pub struct BspStaticRenderer {
    pub data: Bsp,

    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    index_count: u32,

    lightmap_bindgroup: wgpu::BindGroup,
    texture_bindgroup: wgpu::BindGroup,

    faces: Vec<(Range<u32>, BspFace)>,
    pipeline: ReloadablePipeline,
    pub entities: Vec<vdf_reader::entry::Entry>,
}

impl BspStaticRenderer {
    pub fn load<R: Read + Seek>(reader: R, renderer: &Renderer) -> anyhow::Result<Self> {
        let iad = &renderer.iad;

        let mut file = BspFile::new(reader)?;
        let bsp = Bsp::parse(&mut file)?;
        let pakfile = file.read_lump_raw(40)?;
        renderer.fs.lock().mount_zip(pakfile)?;

        let mut textures = Vec::new();
        for td in &bsp.tex_data {
            let name = &bsp.texdata_string_table[td.name_index as usize];
            let path = format!("MATERIALS/{name}");
            let (texture, view) = match get_basetexture_for_vmt(&renderer.fs, &path) {
                Ok(Some(basetexture)) => {
                    let path = format!("MATERIALS/{basetexture}");
                    match load_vtf(&renderer.fs, iad, &path) {
                        Ok(o) => o,
                        Err(e) => {
                            error!("Failed to load texture {path}: {e}");
                            create_fallback_texture(iad, [255, 0, 255])
                        }
                    }
                }
                Ok(None) => create_fallback_texture(iad, [255, 0, 0]),
                Err(e) => {
                    error!("Failed to load VMT {path}: {e}");
                    create_fallback_texture(iad, [255, 0, 255])
                }
            };

            textures.push((texture, view));
        }

        let mut gpu_faces = Vec::with_capacity(bsp.faces.len());
        let mut face_vertices: Vec<StaticMapVertex> = vec![];
        let mut faces = Vec::with_capacity(bsp.faces.len());
        let mut indices: Vec<u32> = vec![];
        let mut i = 0;
        let model = bsp.models.first().unwrap();
        for (fi, f) in bsp.faces
            [model.first_face as usize..(model.first_face + model.num_faces) as usize]
            .iter()
            .enumerate()
        {
            let ti = &bsp.tex_info[f.tex_info as usize];
            let td = &bsp.tex_data[ti.tex_data as usize];
            let texture = &bsp.texdata_string_table[td.name_index as usize];

            let mut flags = FaceFlags::empty();
            flags.set(FaceFlags::DISPLACEMENT, f.disp_info >= 0);
            flags.set(
                FaceFlags::SKY2D,
                texture.to_lowercase().ends_with("toolsskybox2d"),
            );
            flags.set(
                FaceFlags::SKY3D,
                texture.to_lowercase().ends_with("toolsskybox"),
            );
            let lightmap_face_size = IVec2::from(f.lightmap_size) + IVec2::ONE;
            gpu_faces.push(GpuMapFace {
                lightmap_face_size_packed: (lightmap_face_size.x as u32 & 0xFFFF) << 16
                    | (lightmap_face_size.y as u32 & 0xFFFF),
                lightmap_offset: f.lightmap_data_offset / 4,
                flags,
                texture_index: ti.tex_data,
            });

            // if f.disp_info != -1 {
            //     let gf = gpu_faces.last_mut().unwrap();
            //     gf.lightmap_offset += (lightmap_face_size.x * lightmap_face_size.y) * 1;
            // }

            let color = Vec3::from([
                td.reflectivity[0].sqrt(),
                td.reflectivity[1].sqrt(),
                td.reflectivity[2].sqrt(),
            ]);
            let normal = Vec3::from(bsp.planes[f.plane_num as usize].normal);

            // First vertex for this face
            let face_data_start = face_vertices.len();
            // let mut add_vert = |v: Vec3, n: Vec3| {
            //     face_vertices.push(StaticMapVertex::new(
            //         v,
            //         n,
            //         Vec2::ZERO,
            //         Vec2::ZERO,
            //         color,
            //         fi as u32,
            //     ));
            // };

            macro_rules! add_vert {
                ($v:expr, $luv:expr, $n:expr) => {
                    face_vertices.push(StaticMapVertex::new(
                        $v,
                        $n,
                        Vec2::ZERO,
                        $luv,
                        color,
                        fi as u32,
                    ));
                };
            }

            if f.disp_info != -1 {
                let dispinfo = &bsp.disp_info[f.disp_info as usize];
                let low_base = Vec3::from(dispinfo.start_position);
                if f.num_edges != 4 {
                    error!("Bad displacement (face {fi})");
                    continue;
                }

                let mut corner_verts = [Vec3::ZERO; 4];
                let mut base_i = 0;
                let mut base_dist = f32::INFINITY;
                for (k, vert) in corner_verts.iter_mut().enumerate() {
                    let edge = bsp.surfedges[f.first_edge as usize + k];
                    let e = if edge < 0 {
                        bsp.edges[edge.unsigned_abs() as usize][0] as u32
                    } else {
                        bsp.edges[edge.unsigned_abs() as usize][1] as u32
                    };

                    *vert = bsp.vertices[e as usize].into();
                    let this_dist = (vert.x - low_base.x).abs()
                        + (vert.y - low_base.y).abs()
                        + (vert.z - low_base.z).abs();
                    if this_dist < base_dist {
                        base_dist = this_dist;
                        base_i = k;
                    }
                }

                let high_base = corner_verts[(base_i + 3) % 4];
                let high_ray = corner_verts[(base_i + 2) % 4] - high_base;
                let low_ray = corner_verts[(base_i + 1) % 4] - low_base;
                let verts_wide = ((2 << (dispinfo.power - 1)) + 1) as usize;
                let mut base_verts = vec![Vec3::ZERO; verts_wide * verts_wide];
                let mut base_vert_luvs = vec![Vec2::ZERO; verts_wide * verts_wide];
                let mut base_alphas = vec![0.0; verts_wide * verts_wide];
                let base_dispvert = dispinfo.disp_vert_start.unsigned_abs() as usize;

                for y in 0..verts_wide {
                    let fy = y as f32 / (verts_wide as f32 - 1.0);

                    let mid_base = low_base + low_ray * fy;
                    let mid_ray = high_base + high_ray * fy - mid_base;

                    for x in 0..verts_wide {
                        let fx = x as f32 / (verts_wide as f32 - 1.0);
                        let ii = y * verts_wide + x;

                        let vert = &bsp.disp_verts[base_dispvert + ii];
                        let offset = Vec3::from(vert.vec);
                        let scale = vert.dist;
                        let alpha = vert.alpha / 255.0;

                        base_verts[ii] = mid_base + mid_ray * fx + offset * scale;
                        base_vert_luvs[ii] = vec2(
                            lightmap_face_size.x as f32 * fx - 1.0,
                            lightmap_face_size.y as f32 * fy - 1.0,
                        );
                        base_alphas[ii] = alpha;
                    }
                }

                for y in 0..(verts_wide - 1) {
                    for x in 0..(verts_wide - 1) {
                        let ii = y * verts_wide + x;

                        let v0 = base_verts[ii];
                        let v1 = base_verts[ii + 1];
                        let v2 = base_verts[ii + verts_wide];
                        let v3 = base_verts[ii + verts_wide + 1];

                        let v0_luv = base_vert_luvs[ii];
                        let v1_luv = base_vert_luvs[ii + 1];
                        let v2_luv = base_vert_luvs[ii + verts_wide];
                        let v3_luv = base_vert_luvs[ii + verts_wide + 1];

                        add_vert!(v0, v0_luv, normal);
                        add_vert!(v1, v1_luv, normal);
                        add_vert!(v2, v2_luv, normal);
                        add_vert!(v3, v3_luv, normal);

                        if ii.is_multiple_of(2) {
                            indices.extend_from_slice(&[i, i + 1, i + 2, i + 1, i + 3, i + 2]);
                        } else {
                            indices.extend_from_slice(&[i, i + 3, i + 2, i + 1, i + 3, i]);
                        }

                        i += 4;
                    }
                }
            } else {
                let mut face_indices = vec![];
                for i in 0..f.num_edges as usize {
                    let edge = bsp.surfedges[f.first_edge as usize + i];
                    let e = if edge < 0 {
                        bsp.edges[edge.unsigned_abs() as usize][0] as u32
                    } else {
                        bsp.edges[edge.unsigned_abs() as usize][1] as u32
                    };
                    face_indices.push(e);
                }

                for ii in 2..face_indices.len() {
                    let v0: Vec3 = bsp.vertices[face_indices[ii] as usize].into();
                    let v1: Vec3 = bsp.vertices[face_indices[ii - 1] as usize].into();
                    let v2: Vec3 = bsp.vertices[face_indices[0] as usize].into();

                    add_vert!(v0, Vec2::ZERO, normal);
                    add_vert!(v1, Vec2::ZERO, normal);
                    add_vert!(v2, Vec2::ZERO, normal);

                    indices.extend_from_slice(&[i, i + 1, i + 2]);

                    i += 3;
                }
                // warn!(
                //     "Can't handle a face with more/less than 4 vertices (has {})",
                //     face.len()
                // );
                // }
            }

            for v in &mut face_vertices[face_data_start..] {
                // Texture UV
                {
                    let tu = Vec4::from(ti.texture_vecs[0]);
                    let tv = Vec4::from(ti.texture_vecs[1]);
                    v.uv.x = tu.dot(v.position.extend(1.0));
                    v.uv.y = tv.dot(v.position.extend(1.0));

                    v.uv.x /= td.width as f32;
                    v.uv.y /= td.height as f32;
                }

                // Lightmap UV
                if f.disp_info == -1 {
                    if f.lightmap_data_offset >= 0 {
                        let lu = Vec4::from(ti.lightmap_vecs[0]);
                        let lv = Vec4::from(ti.lightmap_vecs[1]);
                        v.lightmap_uv.x =
                            (lu.xyz().dot(v.position) + lu.w) - f.lightmap_mins[0] as f32;
                        v.lightmap_uv.y =
                            (lv.xyz().dot(v.position) + lv.w) - f.lightmap_mins[1] as f32;
                    } else {
                        v.lightmap_uv = Vec2::ONE / 2.0;
                    }
                }
            }

            let face_data_end = face_vertices.len();
            let draw_range = (face_data_start as u32)..(face_data_end as u32);

            faces.push((draw_range, f.clone()));
        }

        let vertex_buffer = iad.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Static Map Geometry Vertex Buffer"),
            usage: wgpu::BufferUsages::VERTEX,
            contents: bytemuck::cast_slice(&face_vertices),
        });

        let index_buffer = iad.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Static Map Geometry Index Buffer"),
            usage: wgpu::BufferUsages::INDEX,
            contents: bytemuck::cast_slice(&indices),
        });

        let mut lightmap_data = bsp
            .lightmap_data
            .iter()
            .map(|t| {
                let exponent_quantized = (t.exponent as i32 + 127) as u8;
                (t.r as u32) << 24
                    | (t.g as u32) << 16
                    | (t.b as u32) << 8
                    | exponent_quantized as u32
            })
            .collect::<Vec<_>>();

        if lightmap_data.is_empty() {
            lightmap_data.push(0);
            gpu_faces.iter_mut().for_each(|face| {
                face.lightmap_offset = -1;
            });
        }

        let lightmap_buffer = iad.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Static Map Lightmap Data"),
            usage: wgpu::BufferUsages::STORAGE,
            contents: bytemuck::cast_slice(&lightmap_data),
        });

        let faceinfo_buffer = iad.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Static Map Face Info"),
            usage: wgpu::BufferUsages::STORAGE,
            contents: bytemuck::cast_slice(&gpu_faces),
        });

        let lightmap_bindgroup_layout =
            iad.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: None,
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            });

        let lightmap_bindgroup = iad.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &lightmap_bindgroup_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: &lightmap_buffer,
                        offset: 0,
                        size: None,
                    }),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: &faceinfo_buffer,
                        offset: 0,
                        size: None,
                    }),
                },
            ],
        });

        let texture_bindgroup_layout =
            iad.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: None,
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: NonZeroU32::new(textures.len() as u32),
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            });

        let sampler = iad.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            address_mode_w: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let texture_views = textures.iter().map(|(_, view)| view).collect::<Vec<_>>();
        let texture_bindgroup = iad.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &texture_bindgroup_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureViewArray(&texture_views),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });

        let pipeline_layout = iad.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[&lightmap_bindgroup_layout, &texture_bindgroup_layout],
            push_constant_ranges: &[wgpu::PushConstantRange {
                range: 0..64,
                stages: wgpu::ShaderStages::VERTEX,
            }],
        });

        let pipeline = ReloadablePipeline::new(
            pipeline_layout,
            ShaderSource::new_file("shaders/world.wgsl").context("Failed to load world shader")?,
            Box::new(|device: &wgpu::Device, layout, shader| {
                device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: None,
                    layout: Some(&layout),
                    vertex: wgpu::VertexState {
                        module: &shader,
                        entry_point: Some("vs_main"),
                        compilation_options: Default::default(),
                        buffers: std::slice::from_ref(&StaticMapVertex::LAYOUT),
                    },
                    primitive: wgpu::PrimitiveState {
                        topology: wgpu::PrimitiveTopology::TriangleList,
                        strip_index_format: None,
                        front_face: wgpu::FrontFace::Ccw,
                        cull_mode: Some(wgpu::Face::Back),
                        unclipped_depth: false,
                        polygon_mode: wgpu::PolygonMode::Fill,
                        conservative: false,
                    },
                    depth_stencil: Some(wgpu::DepthStencilState {
                        format: wgpu::TextureFormat::Depth32Float,
                        depth_write_enabled: true,
                        depth_compare: wgpu::CompareFunction::Greater,
                        stencil: Default::default(),
                        bias: Default::default(),
                    }),
                    multisample: Default::default(),
                    fragment: Some(wgpu::FragmentState {
                        module: &shader,
                        entry_point: Some("fs_main"),
                        compilation_options: Default::default(),
                        targets: &[Some(wgpu::ColorTargetState {
                            format: wgpu::TextureFormat::Bgra8UnormSrgb,
                            blend: None,
                            write_mask: wgpu::ColorWrites::ALL,
                        })],
                    }),
                    multiview: None,
                    cache: None,
                })
            }),
        );

        let mut entities = vec![];
        let mut de = vdf_reader::serde::Deserializer::from_str(&bsp.entities);
        while let Ok(e) = vdf_reader::entry::Entry::deserialize(&mut de) {
            entities.push(e);
        }

        Ok(Self {
            data: bsp,
            vertex_buffer,
            index_buffer,
            index_count: indices.len() as u32,
            lightmap_bindgroup,
            texture_bindgroup,
            faces,
            pipeline,
            entities,
        })
    }

    pub fn render(
        &mut self,
        iad: &InstanceAdapterDevice,
        pass: &mut wgpu::RenderPass,
        camera: Mat4,
    ) {
        pass.set_pipeline(&self.pipeline.compiled_pipeline(iad));
        pass.set_bind_group(0, &self.lightmap_bindgroup, &[]);
        pass.set_bind_group(1, &self.texture_bindgroup, &[]);
        pass.set_push_constants(
            wgpu::ShaderStages::VERTEX,
            0,
            bytemuck::cast_slice(&[camera]),
        );
        pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);

        pass.draw_indexed(0..self.index_count, 0, 0..1);
        // for (draw_range, _face) in &self.faces {
        //     pass.draw_indexed(draw_range.clone(), 0, 0..1);
        // }
    }
}

#[derive(Default, Pod, Copy, Clone, Zeroable)]
#[repr(C, packed)]
pub struct StaticMapVertex {
    pub position: Vec3,
    pub normal: Vec3,
    pub uv: Vec2,
    pub lightmap_uv: Vec2,
    pub color: Vec4,
    pub face: u32,
}

impl StaticMapVertex {
    pub fn new(
        position: Vec3,
        normal: Vec3,
        uv: Vec2,
        lightmap_uv: Vec2,
        color: Vec3,
        face: u32,
    ) -> StaticMapVertex {
        StaticMapVertex {
            position,
            normal,
            uv,
            lightmap_uv,
            color: color.extend(1.0),
            face,
        }
    }

    pub const ATTRIBUTES: &[wgpu::VertexAttribute] = &wgpu::vertex_attr_array![
        0 => Float32x3,
        1 => Float32x3,
        2 => Float32x2,
        3 => Float32x2,
        4 => Float32x4,
        5 => Uint32,
    ];

    pub const LAYOUT: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
        array_stride: std::mem::size_of::<StaticMapVertex>() as u64,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: Self::ATTRIBUTES,
    };
}

#[derive(Default, Pod, Copy, Clone, Zeroable, Debug)]
#[repr(C, packed)]
pub struct GpuMapFace {
    pub lightmap_face_size_packed: u32,
    pub lightmap_offset: i32,
    pub flags: FaceFlags,
    pub texture_index: i32,
}

bitflags! {
    #[derive(Default, Copy, Clone, Debug)]
    #[repr(C)]
    pub struct FaceFlags: u32 {
        const DISPLACEMENT = (1 << 0);
        const SKY2D = (1 << 1);
        const SKY3D = (1 << 2);
    }
}

unsafe impl bytemuck::Zeroable for FaceFlags {}
unsafe impl bytemuck::Pod for FaceFlags {}
