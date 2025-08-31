use std::{fs::File, path::PathBuf, rc::Rc, sync::Arc, time::Instant};

use anyhow::Context as _;
use clap::Parser;
use game_detector::InstalledGame;
use image::EncodableLayout;
use parking_lot::Mutex;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::{layer::SubscriberExt, EnvFilter};

use crate::{
    fs::{Filesystem, SharedFilesystem},
    renderer::bsp::BspStaticRenderer,
};

pub mod args;
pub mod fs;
pub mod renderer;

#[macro_use]
extern crate tracing;

static ICON_DATA: &[u8] = include_bytes!("../../../powerjack.png");

fn main() -> anyhow::Result<()> {
    tracing::subscriber::set_global_default(
        tracing_subscriber::registry()
            .with(tracing_subscriber::fmt::layer().compact().without_time())
            .with(
                EnvFilter::builder()
                    .with_default_directive(LevelFilter::INFO.into())
                    .from_env_lossy(),
            ),
    )
    .expect("Failed to set global tracing subscriber");

    let tf2_path = if let Some(InstalledGame::Steam(appstate)) =
        game_detector::find_all_games().into_iter().find(|g| {
            if let InstalledGame::Steam(g) = g {
                g.appid == 440
            } else {
                false
            }
        }) {
        appstate.game_path
    } else {
        panic!("TF2 installation not found");
    };

    let tf2_path = PathBuf::from(tf2_path);
    let tf_dir = tf2_path.join("tf");
    let args = args::Args::parse();

    let mut fs = Filesystem::default();
    fs.mount_vpk(tf_dir.join("tf2_textures_dir.vpk"))?;
    // fs.mount_vpk(tf_dir.join("tf2_misc_dir.vpk"))?;
    // fs.mount_vpk(tf_dir.join("tf2_sound_misc_dir.vpk"))?;
    // if let Err(e) = fs.mount_vpk(tf_dir.join("tf2_sound_vo_english_dir.vpk")) {
    //     error!("Failed to mount tf2_sound_vo_english_dir.vpk: {e}");
    // }
    let fs: SharedFilesystem = Arc::new(Mutex::new(fs));

    let sdl_context = Rc::new(sdl3::init().unwrap());
    let video_subsystem = sdl_context.video().unwrap();

    let mut window = video_subsystem
        .window("Powerjack", 1920, 1080)
        .position_centered()
        .resizable()
        .build()
        .expect("Failed to create window");

    let icon = image::load_from_memory(ICON_DATA)
        .context("Failed to load icon")?
        .to_rgba8();

    let mut icon_bytes = icon.as_bytes().to_vec();
    let window_icon = sdl3::surface::Surface::from_data(
        &mut icon_bytes,
        icon.width(),
        icon.height(),
        icon.width() * 4,
        sdl3::pixels::PixelFormatEnum::ABGR8888.into(),
    )
    .context("Failed to create window icon")?;
    window.set_icon(window_icon);

    video_subsystem.text_input().start(&window);

    let mut renderer = renderer::Renderer::new(&window, &fs)?;
    let mut bsp = BspStaticRenderer::load(
        File::open(&args.bsp).context("Failed to open bsp file")?,
        &renderer.iad,
    )?;
    info!("Loaded '{}'", args.bsp);

    let mut event_pump = sdl_context.event_pump()?;
    let mut last_time = Instant::now();
    'running: loop {
        let now = Instant::now();
        let dt = last_time.elapsed().as_secs_f32();
        last_time = now;

        for event in event_pump.poll_iter() {
            renderer.camera.handle_event(&event);

            #[allow(clippy::single_match, clippy::collapsible_match)]
            match event {
                sdl3::event::Event::Quit { .. } => break 'running,
                sdl3::event::Event::Window { win_event, .. } => match win_event {
                    sdl3::event::WindowEvent::Resized(width, height) => {
                        renderer.resize(width as u32, height as u32);
                    }
                    _ => {}
                },
                _ => {}
            }
        }

        renderer.camera.update(dt);
        renderer.render(&mut bsp);
    }

    Ok(())
}
