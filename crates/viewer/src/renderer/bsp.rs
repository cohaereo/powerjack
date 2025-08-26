use std::{
    fs::File,
    io::{Read, Seek, Write},
    ops::Range,
};

use anyhow::Context;
use bytemuck::{Pod, Zeroable};
use glam::{IVec2, Mat4, Vec2, Vec3, Vec4};
use powerjack_bsp::{lumps::BspFace, Bsp, BspFile};
use wgpu::util::DeviceExt;

use crate::renderer::{
    iad::InstanceAdapterDevice,
    reloadable_pipeline::{ReloadablePipeline, ShaderSource},
};

pub struct BspStaticRenderer {
    data: Bsp,

    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    index_count: u32,

    lightmap_bindgroup: wgpu::BindGroup,

    faces: Vec<(Range<u32>, BspFace)>,
    pipeline: ReloadablePipeline,
}

impl BspStaticRenderer {
    pub fn load<R: Read + Seek>(reader: R, iad: &InstanceAdapterDevice) -> anyhow::Result<Self> {
        let mut file = BspFile::new(reader)?;
        let bsp = Bsp::parse(&mut file)?;

        let mut gpu_faces = Vec::with_capacity(bsp.faces.len());
        let mut face_data: Vec<StaticMapVertex> = vec![];
        let mut faces = Vec::with_capacity(bsp.faces.len());
        let mut indices: Vec<u32> = vec![];
        let mut i = 0;
        let model = bsp.models.first().unwrap();
        for (fi, f) in bsp.faces
            [model.first_face as usize..(model.first_face + model.num_faces) as usize]
            .iter()
            .enumerate()
        {
            gpu_faces.push(GpuMapFace {
                lightmap_face_size: IVec2::from(f.lightmap_size) + IVec2::ONE,
                lightmap_offset: f.lightmap_data_offset / 4,
                padding: 0xFEEDBEEF,
            });

            if f.disp_info != -1 {
                continue;
            }

            let normal = Vec3::from(bsp.planes[f.plane_num as usize].normal);
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

            let ti = &bsp.tex_info[f.tex_info as usize];
            let td = &bsp.tex_data[ti.tex_data as usize];
            let color = Vec3::from([td.reflectivity[0], td.reflectivity[1], td.reflectivity[2]]);

            // First vertex for this face
            let face_data_start = face_data.len();
            for ii in 2..face_indices.len() {
                face_data.push(StaticMapVertex::new(
                    bsp.vertices[face_indices[ii] as usize].into(),
                    normal,
                    Vec2::ZERO,
                    Vec2::ZERO,
                    color,
                    fi as u32,
                ));
                face_data.push(StaticMapVertex::new(
                    bsp.vertices[face_indices[ii - 1] as usize].into(),
                    normal,
                    Vec2::ZERO,
                    Vec2::ZERO,
                    color,
                    fi as u32,
                ));
                face_data.push(StaticMapVertex::new(
                    bsp.vertices[face_indices[0] as usize].into(),
                    normal,
                    Vec2::ZERO,
                    Vec2::ZERO,
                    color,
                    fi as u32,
                ));

                indices.push(i);
                indices.push(i + 1);
                indices.push(i + 2);

                i += 3;
            }

            for v in &mut face_data[face_data_start..] {
                // Texture UV
                {
                    let tu = Vec4::from(ti.texture_vecs[0]);
                    let tv = Vec4::from(ti.texture_vecs[1]);
                    v.uv.x = tu.x * v.position.x + tu.y * v.position.y + tu.z * v.position.z + tu.w;
                    v.uv.y = tv.x * v.position.x + tv.y * v.position.y + tv.z * v.position.z + tv.w;

                    v.uv.x /= td.width as f32;
                    v.uv.y /= td.height as f32;
                }

                // Lightmap UV
                if f.lightmap_data_offset >= 0 {
                    let lu = Vec4::from(ti.lightmap_vecs[0]);
                    let lv = Vec4::from(ti.lightmap_vecs[1]);
                    v.lightmap_uv.x = lu.dot(v.position.extend(1.0)) - f.lightmap_mins[0] as f32;
                    v.lightmap_uv.y = lv.dot(v.position.extend(1.0)) - f.lightmap_mins[1] as f32;
                } else {
                    v.lightmap_uv = Vec2::ZERO;
                }
            }
            // warn!(
            //     "Can't handle a face with more/less than 4 vertices (has {})",
            //     face.len()
            // );
            // }

            let face_data_end = face_data.len();
            let draw_range = (face_data_start as u32)..(face_data_end as u32);

            faces.push((draw_range, f.clone()));
        }

        let vertex_buffer = iad.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Static Map Geometry Vertex Buffer"),
            usage: wgpu::BufferUsages::VERTEX,
            contents: bytemuck::cast_slice(&face_data),
        });

        let index_buffer = iad.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Static Map Geometry Index Buffer"),
            usage: wgpu::BufferUsages::INDEX,
            contents: bytemuck::cast_slice(&indices),
        });

        let lightmap_data = bsp
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

        let pipeline_layout = iad.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[&lightmap_bindgroup_layout],
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
                        depth_compare: wgpu::CompareFunction::Less,
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

        Ok(Self {
            data: bsp,
            vertex_buffer,
            index_buffer,
            index_count: indices.len() as u32,
            lightmap_bindgroup,
            faces,
            pipeline,
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
        pass.set_push_constants(
            wgpu::ShaderStages::VERTEX,
            0,
            bytemuck::cast_slice(&[camera]),
        );
        pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);

        // pass.draw_indexed(0..self.index_count, 0, 0..1);
        for (draw_range, face) in &self.faces {
            pass.draw_indexed(draw_range.clone(), 0, 0..1);
        }
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
    pub lightmap_face_size: IVec2,
    pub lightmap_offset: i32,
    pub padding: u32,
}
