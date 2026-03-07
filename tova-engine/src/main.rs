mod app;
mod camera;
mod geometry;
mod graphics;
mod hud;
mod input;
mod voxel;

use app::TovaApp;
use winit::event_loop::EventLoop;

#[cfg(target_arch = "wasm32")]
use winit::platform::web::EventLoopExtWebSys;

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    env_logger::init();

    let event_loop = EventLoop::new().expect("failed to create event loop");
    let mut app = TovaApp::default();
    event_loop
        .run_app(&mut app)
        .expect("failed to run rebuilt Tova app");
}

#[cfg(target_arch = "wasm32")]
fn main() {
    console_error_panic_hook::set_once();
    let _ = console_log::init_with_level(log::Level::Info);

    let event_loop = EventLoop::new().expect("failed to create web event loop");
    let app = TovaApp::default();
    event_loop.spawn_app(app);
}
