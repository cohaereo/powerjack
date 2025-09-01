use std::{io::Cursor, path::Path};

use anyhow::Context;
use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Vec3};
use powerjack_mdl::{vtx::VtxData, vvd::VvdData};
use wgpu::util::DeviceExt;

use crate::{
    fs::SharedFilesystem,
    renderer::{
        iad::InstanceAdapterDevice,
        reloadable_pipeline::{ReloadablePipeline, ShaderSource},
    },
};

pub struct MdlRenderer {
    pipeline: ReloadablePipeline,
    vertex_buffer: wgpu::Buffer,
    // index_buffer: wgpu::Buffer,
    vertex_count: u32,
}

impl MdlRenderer {
    pub fn load(
        fs: &SharedFilesystem,
        iad: &InstanceAdapterDevice,
        path: impl AsRef<Path>,
    ) -> anyhow::Result<Self> {
        let mdl_path = path.as_ref();
        let vvd_path = mdl_path.with_extension("vvd");
        let vtx_path = mdl_path.with_extension("dx90.vtx");

        let vvd = {
            let vvd_data = fs
                .lock()
                .read_path(&vvd_path.to_string_lossy())?
                .context("VVD file not found")?;
            VvdData::parse(&mut Cursor::new(vvd_data))?
        };

        let vtx = {
            let vtx_data = fs
                .lock()
                .read_path(&vtx_path.to_string_lossy())?
                .context("VTX file not found")?;
            VtxData::parse(&mut Cursor::new(vtx_data))?
        };

        let mut vertices = vec![];
        for body_part in &vtx.body_parts {
            for model in body_part {
                let lod = &model[0];
                for mesh in lod {
                    for strip_group in mesh {
                        for vertex in &strip_group.vertices {
                            let orig_vertex = &vvd.vertices[vertex.orig_mesh_vert_id as usize];
                            vertices.push(MdlVertex {
                                position: orig_vertex.position.into(),
                                normal: orig_vertex.normal.into(),
                            });
                        }
                    }
                }
            }
        }
        let vertex_count = vertices.len() as u32;

        let vertex_buffer = iad.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let pipeline_layout = iad.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[],
            push_constant_ranges: &[wgpu::PushConstantRange {
                range: 0..128,
                stages: wgpu::ShaderStages::VERTEX,
            }],
        });

        let pipeline = ReloadablePipeline::new(
            pipeline_layout,
            ShaderSource::new_file("shaders/mdl.wgsl").context("Failed to load mdl shader")?,
            Box::new(|device: &wgpu::Device, layout, shader| {
                device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: None,
                    layout: Some(&layout),
                    vertex: wgpu::VertexState {
                        module: &shader,
                        entry_point: Some("vs_main"),
                        compilation_options: Default::default(),
                        buffers: std::slice::from_ref(&MdlVertex::LAYOUT),
                    },
                    primitive: wgpu::PrimitiveState {
                        topology: wgpu::PrimitiveTopology::PointList,
                        // topology: wgpu::PrimitiveTopology::TriangleList,
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

        Ok(Self {
            pipeline,
            vertex_buffer,
            vertex_count,
        })
    }

    pub fn render(
        &mut self,
        iad: &InstanceAdapterDevice,
        pass: &mut wgpu::RenderPass,
        camera: Mat4,
        model: Mat4,
    ) {
        pass.set_pipeline(&self.pipeline.compiled_pipeline(iad));
        // pass.set_bind_group(0, &self.lightmap_bindgroup, &[]);
        // pass.set_bind_group(1, &self.texture_bindgroup, &[]);
        pass.set_push_constants(
            wgpu::ShaderStages::VERTEX,
            0,
            bytemuck::cast_slice(&[camera, model]),
        );
        pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        // pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);

        pass.draw_indexed(0..self.vertex_count, 0, 0..1);
        // for (draw_range, _face) in &self.faces {
        //     pass.draw_indexed(draw_range.clone(), 0, 0..1);
        // }
    }
}

#[derive(Default, Pod, Copy, Clone, Zeroable)]
#[repr(C)]
pub struct MdlVertex {
    pub position: Vec3,
    pub normal: Vec3,
}

impl MdlVertex {
    pub fn new(position: Vec3, normal: Vec3) -> MdlVertex {
        MdlVertex { position, normal }
    }

    pub const ATTRIBUTES: &[wgpu::VertexAttribute] = &wgpu::vertex_attr_array![
        0 => Float32x3,
        1 => Float32x3,
    ];

    pub const LAYOUT: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
        array_stride: std::mem::size_of::<MdlVertex>() as u64,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: Self::ATTRIBUTES,
    };
}
