use std::{io::Cursor, ops::Range, path::Path};

use anyhow::Context;
use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Vec2, Vec3};
use powerjack_mdl::{
    mdl::MdlData,
    vtx::{StripFlags, VtxData},
    vvd::VvdData,
};
use wgpu::util::DeviceExt;

use crate::{
    fs::SharedFilesystem,
    renderer::{
        iad::InstanceAdapterDevice,
        reloadable_pipeline::{ReloadablePipeline, ShaderSource},
        vmt::get_basetexture_for_vmt,
        vtf::{create_fallback_texture, load_vtf},
    },
};

pub struct MdlRenderer {
    pipeline: ReloadablePipeline,
    buffers: Vec<(usize, wgpu::Buffer, wgpu::Buffer, Range<u32>)>,
    materials: Vec<wgpu::BindGroup>,
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

        let mdl = {
            let mdl_data = fs
                .lock()
                .read_path(&mdl_path.to_string_lossy())?
                .context("MDL file not found")?;

            MdlData::parse(&mut Cursor::new(mdl_data))?
        };

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
        let mut accum_index = 0;
        for ((_body_part, models), vtx_models) in mdl.body_parts.iter().zip(vtx.body_parts.iter()) {
            for ((model, meshes), (_vtx_model, vtx_lods)) in models.iter().zip(vtx_models.iter()) {
                let mut fixup_vertices = Vec::with_capacity(model.num_vertices as usize);
                if vvd.header.num_fixups > 0 {
                    for fixup in &vvd.fixups {
                        let verts = &vvd.vertices[fixup.source_vertex_id as usize
                            ..(fixup.source_vertex_id + fixup.num_vertices) as usize];
                        fixup_vertices.extend(verts);
                    }
                } else {
                    fixup_vertices = vvd.vertices[0..model.num_vertices as usize].to_vec();
                }

                let (_vtx_lod, vtx_meshes) = &vtx_lods[0];
                for (mesh, (_vtx_mesh, strip_groups)) in meshes.iter().zip(vtx_meshes.iter()) {
                    let mut vertices = vec![];
                    for strip_group in strip_groups {
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
                                    let orig_vertex = &fixup_vertices[accum_index
                                        + mesh.vertex_offset as usize
                                        + vertex.orig_mesh_vert_id as usize];
                                    vertices.push(MdlVertex {
                                        position: orig_vertex.position.into(),
                                        normal: orig_vertex.normal.into(),
                                        uv: orig_vertex.uv.into(),
                                    });
                                }
                            }
                        }
                    }

                    let vertex_buffer = iad.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: None,
                        contents: bytemuck::cast_slice(&vertices),
                        usage: wgpu::BufferUsages::VERTEX,
                    });

                    let index_buffer = iad.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: None,
                        contents: bytemuck::cast_slice(&[0u16]),
                        usage: wgpu::BufferUsages::INDEX,
                    });

                    let range = 0..vertices.len() as u32;
                    buffers.push((mesh.material as usize, vertex_buffer, index_buffer, range))
                }

                accum_index += model.num_vertices as usize;
            }
        }

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
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            });

        let pipeline_layout = iad.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[&texture_bindgroup_layout],
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
                        topology: wgpu::PrimitiveTopology::TriangleList,
                        strip_index_format: None,
                        front_face: wgpu::FrontFace::Cw,
                        cull_mode: None,
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

        let sampler = iad.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            address_mode_w: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let mut materials = vec![];
        for t in &mdl.textures {
            let mut texture = create_fallback_texture(iad, [255, 0, 255]).1;
            let mut found = false;
            // Try every texture dir until we find the material
            'next_dir: for dir in &mdl.texture_dirs {
                let path = format!("materials/{dir}/{}.vmt", t.name);
                println!("Testing path {path}");
                texture = match get_basetexture_for_vmt(fs, &path) {
                    Ok(Some(basetexture)) => {
                        found = true;
                        let path = format!("materials/{basetexture}.vtf");
                        match load_vtf(fs, iad, &path) {
                            Ok(o) => o,
                            Err(e) => {
                                error!("Failed to load texture {path}: {e}");
                                create_fallback_texture(iad, [255, 0, 255])
                            }
                        }
                    }
                    Ok(None) => continue 'next_dir,
                    Err(e) => {
                        found = true;
                        error!("Failed to load VMT {path}: {e}");
                        create_fallback_texture(iad, [255, 0, 255])
                    }
                }
                .1;

                break;
            }

            if !found {
                error!("Couldn't find material {}", t.name);
            }

            let texture_bindgroup = iad.create_bind_group(&wgpu::BindGroupDescriptor {
                label: None,
                layout: &texture_bindgroup_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&texture),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&sampler),
                    },
                ],
            });

            materials.push(texture_bindgroup);
        }

        Ok(Self {
            pipeline,
            buffers,
            materials,
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

        for (material, vb, _ib, range) in &self.buffers {
            pass.set_bind_group(0, &self.materials[*material], &[]);
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
    pub uv: Vec2,
}

impl MdlVertex {
    pub fn new(position: Vec3, normal: Vec3, uv: Vec2) -> MdlVertex {
        MdlVertex {
            position,
            normal,
            uv,
        }
    }

    pub const ATTRIBUTES: &[wgpu::VertexAttribute] = &wgpu::vertex_attr_array![
        0 => Float32x3,
        1 => Float32x3,
        2 => Float32x2,
    ];

    pub const LAYOUT: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
        array_stride: std::mem::size_of::<MdlVertex>() as u64,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: Self::ATTRIBUTES,
    };
}
