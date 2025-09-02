use std::{io::Cursor, ops::Range, path::Path};

use anyhow::Context;
use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Vec3};
use powerjack_mdl::{
    vtx::{StripFlags, VtxData},
    vvd::VvdData,
};
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
    buffers: Vec<(wgpu::Buffer, wgpu::Buffer, Range<u32>)>,
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

        let mut buffers = vec![];
        for body_part in &vtx.body_parts {
            for (_model, lods) in body_part {
                let (_lod, meshes) = &lods[0];
                for (_mesh, strip_groups) in meshes {
                    for strip_group in strip_groups {
                        // for vertex in &strip_group.vertices {
                        //     let orig_vertex = &vvd.vertices[vertex.orig_mesh_vert_id as usize];
                        //     vertices.push(MdlVertex {
                        //         position: orig_vertex.position.into(),
                        //         normal: orig_vertex.normal.into(),
                        //     });
                        // }

                        let mut vertices = vec![];
                        // let mut indices: Vec<u16> = vec![];
                        for strip in &strip_group.strips {
                            if !strip.flags.contains(StripFlags::IS_TRILIST) {
                                continue;
                            }

                            for i in (0..strip.num_indices).step_by(3) {
                                let idxs = [
                                    strip_group.indices[strip.index_offset as usize + i as usize],
                                    strip_group.indices
                                        [strip.index_offset as usize + i as usize + 1],
                                    strip_group.indices
                                        [strip.index_offset as usize + i as usize + 2],
                                ];

                                let verts = [
                                    strip_group.vertices[idxs[0] as usize],
                                    strip_group.vertices[idxs[1] as usize],
                                    strip_group.vertices[idxs[2] as usize],
                                ];

                                for vertex in &verts {
                                    let orig_vertex =
                                        &vvd.vertices[vertex.orig_mesh_vert_id as usize];
                                    vertices.push(MdlVertex {
                                        position: orig_vertex.position.into(),
                                        normal: orig_vertex.normal.into(),
                                    });
                                }
                            }
                        }

                        let vertex_buffer =
                            iad.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                                label: None,
                                contents: bytemuck::cast_slice(&vertices),
                                usage: wgpu::BufferUsages::VERTEX,
                            });

                        let index_buffer =
                            iad.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                                label: None,
                                contents: bytemuck::cast_slice(&strip_group.indices),
                                usage: wgpu::BufferUsages::INDEX,
                            });

                        let range = 0..vertices.len() as u32;
                        buffers.push((vertex_buffer, index_buffer, range))
                    }
                }
            }
        }

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
                        // topology: wgpu::PrimitiveTopology::PointList,
                        topology: wgpu::PrimitiveTopology::TriangleList,
                        strip_index_format: None,
                        front_face: wgpu::FrontFace::Cw,
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

        Ok(Self { pipeline, buffers })
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

        for (vb, _ib, range) in &self.buffers {
            pass.set_vertex_buffer(0, vb.slice(..));
            // pass.set_index_buffer(ib.slice(..), wgpu::IndexFormat::Uint16);

            // pass.draw_indexed(strip.clone(), 0, 0..1);
            pass.draw(range.clone(), 0..1);
        }
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
