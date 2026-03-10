mod audio;
mod renderer;
mod player;
mod voxel;
mod world;

use std::sync::Arc;

#[cfg(not(target_arch = "wasm32"))]
use std::time::Instant;
#[cfg(target_arch = "wasm32")]
use web_time::Instant;

use glam::Vec3;
use winit::application::ApplicationHandler;
use winit::event::{DeviceEvent, ElementState, KeyEvent, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{CursorGrabMode, Window, WindowAttributes, WindowId};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;
#[cfg(target_arch = "wasm32")]
use std::cell::RefCell;
#[cfg(target_arch = "wasm32")]
use std::rc::Rc;

use audio::AudioSystem;
use renderer::RenderState;
use player::{Input, Player};

struct App {
    state: Option<RenderState>,
    input: Input,
    player: Player,
    audio: AudioSystem,
    last_frame: Instant,
    cursor_grabbed: bool,
    window: Option<Arc<Window>>,
    #[cfg(target_arch = "wasm32")]
    pending_state: Rc<RefCell<Option<RenderState>>>,
}

impl App {
    fn new() -> Self {
        Self {
            state: None,
            input: Input::new(),
            player: Player::new(Vec3::new(0.0, 80.0, 0.0)),
            audio: AudioSystem::new(),
            last_frame: Instant::now(),
            cursor_grabbed: false,
            window: None,
            #[cfg(target_arch = "wasm32")]
            pending_state: Rc::new(RefCell::new(None)),
        }
    }

    fn grab_cursor(&mut self) {
        if let Some(window) = &self.window {
            let _ = window
                .set_cursor_grab(CursorGrabMode::Locked)
                .or_else(|_| window.set_cursor_grab(CursorGrabMode::Confined));
            window.set_cursor_visible(false);
            self.cursor_grabbed = true;
        }
    }

    fn release_cursor(&mut self) {
        if let Some(window) = &self.window {
            let _ = window.set_cursor_grab(CursorGrabMode::None);
            window.set_cursor_visible(true);
            self.cursor_grabbed = false;
        }
    }

    #[cfg(target_arch = "wasm32")]
    fn poll_pending_state(&mut self) {
        if self.state.is_none() {
            if let Some(state) = self.pending_state.borrow_mut().take() {
                self.state = Some(state);
                self.last_frame = Instant::now();
                log::info!("GPU initialized — Tova ready");
            }
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        #[allow(unused_mut)]
        let mut attrs = WindowAttributes::default().with_title("Tova");

        #[cfg(not(target_arch = "wasm32"))]
        {
            attrs = attrs.with_inner_size(winit::dpi::LogicalSize::new(1280, 720));
        }

        let window = Arc::new(event_loop.create_window(attrs).unwrap());

        #[cfg(target_arch = "wasm32")]
        {
            use winit::platform::web::WindowExtWebSys;
            let canvas = window.canvas().expect("winit window should have a canvas on web");

            let web_window = web_sys::window().expect("no global window");
            let document = web_window.document().expect("no document");
            let body = document.body().expect("no body");

            body.set_inner_html("");
            let _ = body.style().set_property("margin", "0");
            let _ = body.style().set_property("overflow", "hidden");
            let _ = canvas.style().set_property("width", "100vw");
            let _ = canvas.style().set_property("height", "100vh");
            let _ = canvas.style().set_property("display", "block");
            body.append_child(&canvas).expect("failed to append canvas");
        }

        self.window = Some(window.clone());

        #[cfg(not(target_arch = "wasm32"))]
        {
            self.state = Some(pollster::block_on(RenderState::new(window)));
            self.last_frame = Instant::now();
        }

        #[cfg(target_arch = "wasm32")]
        {
            let pending = self.pending_state.clone();
            wasm_bindgen_futures::spawn_local(async move {
                let state = RenderState::new(window).await;
                *pending.borrow_mut() = Some(state);
            });
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _id: WindowId,
        event: WindowEvent,
    ) {
        #[cfg(target_arch = "wasm32")]
        self.poll_pending_state();

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),

            WindowEvent::Resized(size) => {
                if let Some(state) = &mut self.state {
                    state.resize(size);
                }
            }

            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: PhysicalKey::Code(key),
                        state: key_state,
                        ..
                    },
                ..
            } => match key_state {
                ElementState::Pressed => {
                    if key == KeyCode::Escape {
                        self.release_cursor();
                    }
                    self.input.key_down(key);
                }
                ElementState::Released => {
                    self.input.key_up(key);
                }
            },

            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                ..
            } => {
                if !self.cursor_grabbed {
                    self.grab_cursor();
                }
            }

            WindowEvent::RedrawRequested => {
                let now = Instant::now();
                let dt = (now - self.last_frame).as_secs_f32();
                self.last_frame = now;

                if let Some(state) = &mut self.state {
                    // Player physics — reads chunk data for collision
                    self.player.update(dt, &self.input, &state.chunk_manager);

                    // Footstep audio
                    if let Some(step_id) = self.player.take_step() {
                        self.audio.play_footstep(step_id);
                    }

                    // Stream chunks around player
                    let (pcx, pcz) = self.player.chunk_pos();
                    state.update_chunks(pcx, pcz);

                    // Update first-person feet
                    state.update_feet(
                        self.player.position,
                        self.player.yaw,
                        self.player.walk_cycle(),
                    );

                    // Sync camera to player eye position
                    state.camera.position = self.player.eye_position();
                    state.camera.yaw = self.player.yaw;
                    state.camera.pitch = self.player.pitch;
                    state.update_camera();

                    match state.render() {
                        Ok(_) => {}
                        Err(wgpu::SurfaceError::Lost) => state.resize(state.size),
                        Err(wgpu::SurfaceError::OutOfMemory) => event_loop.exit(),
                        Err(e) => log::error!("Render error: {:?}", e),
                    }
                }

                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }

            _ => {}
        }
    }

    fn device_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        _device_id: winit::event::DeviceId,
        event: DeviceEvent,
    ) {
        if let DeviceEvent::MouseMotion { delta: (dx, dy) } = event {
            if self.cursor_grabbed {
                self.player.rotate(dx, dy);
            }
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }
}

// ─── Platform entry points ──────────────────────────────────

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    env_logger::init();

    let event_loop = EventLoop::new().unwrap();
    let mut app = App::new();
    event_loop.run_app(&mut app).unwrap();
}

#[cfg(target_arch = "wasm32")]
fn main() {}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(start)]
pub fn wasm_main() {
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));
    console_log::init_with_level(log::Level::Info).expect("could not init logger");

    let event_loop = EventLoop::new().unwrap();
    let app = App::new();

    use winit::platform::web::EventLoopExtWebSys;
    event_loop.spawn_app(app);
}
