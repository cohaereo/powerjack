use std::{ops::Deref, sync::Arc};

pub struct InstanceAdapterDevice {
    pub instance: Arc<wgpu::Instance>,
    pub adapter: Arc<wgpu::Adapter>,
    pub device: Arc<wgpu::Device>,
    pub queue: Arc<wgpu::Queue>,
}

impl InstanceAdapterDevice {
    pub async fn new() -> anyhow::Result<Arc<Self>> {
        let instance = Arc::new(wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            ..Default::default()
        }));

        info!("Available adapters:");
        for (i, adapter) in instance
            .enumerate_adapters(wgpu::Backends::PRIMARY)
            .iter()
            .enumerate()
        {
            let info = adapter.get_info();
            info!(
                "  - #{i} - {}, {} (backend {})",
                info.name, info.driver, info.backend
            );
        }

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: None,
                force_fallback_adapter: false,
            })
            .await?;

        let required_limits = wgpu::Limits {
            max_texture_dimension_1d: 8192,
            max_texture_dimension_2d: 8192,
            max_push_constant_size: 256,
            max_binding_array_elements_per_shader_stage: 4096,
            ..wgpu::Limits::defaults()
        };
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                required_features: wgpu::Features::PUSH_CONSTANTS
                    | wgpu::Features::TEXTURE_COMPRESSION_BC
                    | wgpu::Features::TEXTURE_BINDING_ARRAY
                    | wgpu::Features::BUFFER_BINDING_ARRAY
                    | wgpu::Features::SAMPLED_TEXTURE_AND_STORAGE_BUFFER_ARRAY_NON_UNIFORM_INDEXING,
                required_limits,
                label: None,
                memory_hints: wgpu::MemoryHints::Performance,
                trace: wgpu::Trace::Off,
            })
            .await?;

        device.on_uncaptured_error(Box::new(|error| {
            let error_str = error.to_string();
            if error_str.contains("set_pipeline") {
                return;
            }
            error!("wgpu error: {error}");
        }));

        Ok(Arc::new(Self {
            instance,
            adapter: Arc::new(adapter),
            device: Arc::new(device),
            queue: Arc::new(queue),
        }))
    }

    pub fn backend(&self) -> wgpu::Backend {
        self.adapter.get_info().backend
    }

    pub fn backend_name(&self) -> &'static str {
        match self.backend() {
            wgpu::Backend::Noop => "Noop",
            wgpu::Backend::Vulkan => "Vulkan",
            wgpu::Backend::Metal => "Metal",
            wgpu::Backend::Dx12 => "Direct3D 12",
            wgpu::Backend::Gl => "OpenGL",
            wgpu::Backend::BrowserWebGpu => "WebGPU (Browser)",
        }
    }

    pub fn adapter_name(&self) -> String {
        self.adapter.get_info().name.clone()
    }
}

impl Deref for InstanceAdapterDevice {
    type Target = wgpu::Device;

    fn deref(&self) -> &Self::Target {
        &self.device
    }
}
