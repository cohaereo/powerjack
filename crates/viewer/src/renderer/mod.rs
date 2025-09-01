use std::sync::Arc;

use anyhow::Context;
use glam::Mat4;
use wgpu::{
    rwh::{HasDisplayHandle, HasWindowHandle},
    RenderPass,
};

use crate::{
    fs::SharedFilesystem,
    renderer::{features::bsp::BspStaticRenderer, iad::InstanceAdapterDevice},
};

pub mod camera;
pub mod features;
pub mod iad;
pub mod reloadable_pipeline;
pub mod vmt;
pub mod vtf;

pub struct Renderer<'a> {
    pub iad: Arc<InstanceAdapterDevice>,
    pub surface: wgpu::Surface<'a>,
    pub surface_config: wgpu::SurfaceConfiguration,
    pub(super) fs: SharedFilesystem,

    depth: wgpu::Texture,
    depth_view: wgpu::TextureView,

    pub camera: camera::Camera,
}

impl<'a> Renderer<'a> {
    pub fn new(window: &sdl3::video::Window, fs: &SharedFilesystem) -> anyhow::Result<Self> {
        let iad = pollster::block_on(InstanceAdapterDevice::new())?;
        let surface = unsafe {
            iad.instance
                .create_surface_unsafe(wgpu::SurfaceTargetUnsafe::RawHandle {
                    raw_display_handle: window.display_handle()?.as_raw(),
                    raw_window_handle: window.window_handle()?.as_raw(),
                })?
        };
        let mut surface_config = surface
            .get_default_config(
                &iad.adapter,
                window.size_in_pixels().0,
                window.size_in_pixels().1,
            )
            .context("Failed to get surface configuration")?;
        surface_config.present_mode = wgpu::PresentMode::AutoNoVsync;
        surface.configure(&iad, &surface_config);

        let depth = iad.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size: wgpu::Extent3d {
                width: 1920,
                height: 1080,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });

        let depth_view = depth.create_view(&wgpu::TextureViewDescriptor::default());

        Ok(Self {
            iad,
            surface,
            surface_config,
            depth,
            depth_view,
            fs: Arc::clone(fs),
            camera: camera::Camera::default(),
        })
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.surface_config.width = width;
        self.surface_config.height = height;
        self.surface.configure(&self.iad, &self.surface_config);

        self.depth = self.iad.create_texture(&wgpu::TextureDescriptor {
            label: Some("Depth Texture"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });

        self.depth_view = self
            .depth
            .create_view(&wgpu::TextureViewDescriptor::default());
    }

    pub fn render<F>(&mut self, map: &mut BspStaticRenderer, render: F)
    where
        F: FnOnce(&mut Self, &mut RenderPass, Mat4),
    {
        let Ok(frame) = self.surface.get_current_texture() else {
            return;
        };

        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .iad
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        {
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(0.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                ..Default::default()
            });

            let size = frame.texture.size();
            let world_to_projective = self
                .camera
                .world_to_projective(size.width as f32 / size.height as f32);
            map.render(&self.iad, &mut rpass, world_to_projective);
            render(self, &mut rpass, world_to_projective);
        }

        self.iad.queue.submit(Some(encoder.finish()));

        frame.present();
    }
}
