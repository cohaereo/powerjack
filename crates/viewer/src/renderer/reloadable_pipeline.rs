use std::{
    path::PathBuf,
    str::FromStr,
    sync::{Arc, Mutex},
    time::SystemTime,
};

pub struct ReloadablePipeline {
    pub layout: wgpu::PipelineLayout,
    pub shader: ShaderSource,
    pub cached_pipeline: Option<wgpu::RenderPipeline>,
    #[allow(clippy::type_complexity)]
    descriptor_create: Box<
        dyn FnMut(&wgpu::Device, wgpu::PipelineLayout, wgpu::ShaderModule) -> wgpu::RenderPipeline,
    >,
    _marker: std::marker::PhantomData<()>,
}

impl ReloadablePipeline {
    pub fn new<F>(
        layout: wgpu::PipelineLayout,
        shader: ShaderSource,
        descriptor_create: Box<F>,
    ) -> Self
    where
        F: FnMut(&wgpu::Device, wgpu::PipelineLayout, wgpu::ShaderModule) -> wgpu::RenderPipeline
            + 'static,
    {
        Self {
            layout,
            shader,
            descriptor_create,
            cached_pipeline: None,
            _marker: std::marker::PhantomData,
        }
    }

    /// Compiles the pipeline using the provided device, or uses the cached pipeline if it exists and is still valid
    pub fn compiled_pipeline(&mut self, device: &wgpu::Device) -> wgpu::RenderPipeline {
        self.shader.reload();
        if self.cached_pipeline.is_some() && !self.shader.is_changed() {
            self.cached_pipeline.as_ref().cloned().unwrap()
        } else {
            // info!("Compiling pipeline {:?}", self.shader.name());
            self.shader.set_changed(false);
            let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: self.shader.name(),
                source: wgpu::ShaderSource::Wgsl(self.shader.source().into()),
            });
            self.cached_pipeline = Some((*self.descriptor_create)(
                device,
                self.layout.clone(),
                shader,
            ));
            self.cached_pipeline.as_ref().cloned().unwrap()
        }
    }
}

pub enum ShaderSource {
    File {
        source: Arc<Mutex<WatchedSource>>,
        path: String,
    },
    Inline {
        source: String,
    },
}

impl ShaderSource {
    /// Creates a new shader source from a file. Errors if the given path does not exist or cannot be read.
    /// Path is relative crates/viewer, or to the current working directory if it's not found.
    pub fn new_file(path: &str) -> anyhow::Result<Self> {
        let path_dev = PathBuf::from_str("crates/viewer/")?.join(path);
        let (source, load_path) = std::fs::read_to_string(&path_dev)
            .map(|s| (s, path_dev))
            .or_else(|_| {
                std::fs::read_to_string(path).map(|s| (s, PathBuf::from_str(path).unwrap()))
            })?;

        let source = Arc::new(Mutex::new(WatchedSource {
            source,
            load_path,
            changed: false,
            last_change: SystemTime::UNIX_EPOCH,
        }));

        // SOURCES.write().unwrap().insert(load_path, source.clone());

        Ok(ShaderSource::File {
            source,
            path: path.to_string(),
        })
    }

    pub fn new_inline(source: String) -> Self {
        ShaderSource::Inline { source }
    }

    pub fn reload(&self) {
        let Self::File { source, .. } = self else {
            return;
        };

        let mut sauce = source.lock().unwrap();
        let Ok(modified) = std::fs::metadata(&sauce.load_path).and_then(|m| m.modified()) else {
            return;
        };

        if modified > sauce.last_change {
            sauce.last_change = modified;
            sauce.changed = true;
        }

        if let Ok(source) = std::fs::read_to_string(&sauce.load_path) {
            sauce.source = source;
        }
    }

    pub fn set_changed(&self, changed: bool) {
        match self {
            ShaderSource::File { source, .. } => source.lock().unwrap().changed = changed,
            ShaderSource::Inline { .. } => {}
        }
    }

    pub fn is_changed(&self) -> bool {
        match self {
            ShaderSource::File { source, .. } => source.lock().unwrap().changed,
            ShaderSource::Inline { .. } => false,
        }
    }

    pub fn source(&self) -> String {
        match self {
            ShaderSource::File { source, .. } => source.lock().unwrap().source.clone(),
            ShaderSource::Inline { source } => source.clone(),
        }
    }

    pub fn name(&self) -> Option<&str> {
        match self {
            ShaderSource::File { path, .. } => Some(path),
            ShaderSource::Inline { .. } => None,
        }
    }
}

pub struct WatchedSource {
    source: String,
    load_path: PathBuf,
    changed: bool,
    last_change: SystemTime,
}

// lazy_static! {
//     static ref SOURCES: RwLock<HashMap<PathBuf, Arc<RwLock<WatchedSource>>>> =
//         RwLock::new(HashMap::new());
//     static ref SOURCE_WATCHER: std::thread::JoinHandle<()> = std::thread::spawn(|| {
//         let mut last_changed: HashMap<PathBuf, SystemTime> = HashMap::new();
//         loop {
//             std::thread::sleep(std::time::Duration::from_millis(100));
//         }
//     });
// }
