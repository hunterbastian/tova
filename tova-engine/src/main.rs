mod audio;
mod player;
mod renderer;
mod voxel;

use std::sync::Arc;
use std::time::Instant;
use winit::application::ApplicationHandler;
use winit::event::{DeviceEvent, ElementState, KeyEvent, MouseButton, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{CursorGrabMode, Window, WindowAttributes, WindowId};

use player::Input;
use renderer::camera::MoveIntent;
use renderer::state::{OverlayMode, BTN_BOTTOM, BTN_LEFT, BTN_RIGHT, BTN_TOP};
use renderer::RenderState;

fn rustc_version_label() -> &'static str {
    option_env!("TOVA_RUSTC_VERSION").unwrap_or("rustc unknown")
}

fn rust_updated_at_label() -> &'static str {
    option_env!("TOVA_RUST_UPDATED_AT").unwrap_or("unknown")
}

struct App {
    state: Option<RenderState>,
    input: Input,
    last_frame: Instant,
    fps_accum_seconds: f32,
    fps_accum_frames: u32,
    fps_display: f32,
    cursor_grabbed: bool,
    window: Option<Arc<Window>>,
    // Game state
    title_screen: bool,
    paused: bool,
    god_mode: bool,
    fog_enabled: bool,
    typing_command: bool,
    command_buffer: String,
    mouse_pos: (f64, f64),
    ambient_audio: Option<audio::AmbientAudio>,
}

impl App {
    fn new() -> Self {
        Self {
            state: None,
            input: Input::new(),
            last_frame: Instant::now(),
            fps_accum_seconds: 0.0,
            fps_accum_frames: 0,
            fps_display: 0.0,
            cursor_grabbed: false,
            window: None,
            title_screen: true,
            paused: false,
            god_mode: false,
            fog_enabled: true,
            typing_command: false,
            command_buffer: String::new(),
            mouse_pos: (0.0, 0.0),
            ambient_audio: None,
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

    fn update_title(&self) {
        if let Some(window) = &self.window {
            let (quality_label, shader_pack_label, vsync_label) = if let Some(state) = &self.state {
                let settings = state.settings();
                (
                    settings.preset.label(),
                    if settings.shader_pack_enabled {
                        "Shaders ON"
                    } else {
                        "Shaders OFF"
                    },
                    if settings.vsync {
                        "VSync ON"
                    } else {
                        "VSync OFF"
                    },
                )
            } else {
                ("--", "Shaders --", "VSync --")
            };
            let base = if self.title_screen {
                "Tova — TITLE SCREEN | Press Enter or Click to Start".to_string()
            } else if self.typing_command {
                format!("Tova — /{}", self.command_buffer)
            } else if self.paused {
                let god = if self.god_mode { "ON" } else { "OFF" };
                format!("Tova — PAUSED | God Mode: {}", god)
            } else {
                let mut flags = Vec::new();
                if self.god_mode {
                    flags.push("GOD");
                }
                if !self.fog_enabled {
                    flags.push("NO FOG");
                }
                if flags.is_empty() {
                    "Tova".to_string()
                } else {
                    format!("Tova — {}", flags.join(" | "))
                }
            };
            let title = format!(
                "{base} | FPS {:.1} | {} | {} | {} | {} | updated {}",
                self.fps_display,
                quality_label,
                shader_pack_label,
                vsync_label,
                rustc_version_label(),
                rust_updated_at_label()
            );
            window.set_title(&title);
        }
    }

    fn start_game_from_title(&mut self) {
        self.title_screen = false;
        self.paused = false;
        self.typing_command = false;
        self.command_buffer.clear();
        self.last_frame = Instant::now();
        self.grab_cursor();
        if let Some(state) = &mut self.state {
            state.set_overlay_mode(OverlayMode::None);
        }
        self.update_title();
    }

    fn execute_command(&mut self) {
        let cmd = self.command_buffer.trim().to_lowercase();
        match cmd.as_str() {
            "fog" => {
                self.fog_enabled = !self.fog_enabled;
                if let Some(state) = &mut self.state {
                    state.set_fog(self.fog_enabled);
                }
            }
            "clear" => {
                self.fog_enabled = false;
                if let Some(state) = &mut self.state {
                    state.set_fog(false);
                }
            }
            _ => {}
        }
        self.command_buffer.clear();
    }
}

fn key_to_char(key: KeyCode) -> Option<char> {
    match key {
        KeyCode::KeyA => Some('a'),
        KeyCode::KeyB => Some('b'),
        KeyCode::KeyC => Some('c'),
        KeyCode::KeyD => Some('d'),
        KeyCode::KeyE => Some('e'),
        KeyCode::KeyF => Some('f'),
        KeyCode::KeyG => Some('g'),
        KeyCode::KeyH => Some('h'),
        KeyCode::KeyI => Some('i'),
        KeyCode::KeyJ => Some('j'),
        KeyCode::KeyK => Some('k'),
        KeyCode::KeyL => Some('l'),
        KeyCode::KeyM => Some('m'),
        KeyCode::KeyN => Some('n'),
        KeyCode::KeyO => Some('o'),
        KeyCode::KeyP => Some('p'),
        KeyCode::KeyQ => Some('q'),
        KeyCode::KeyR => Some('r'),
        KeyCode::KeyS => Some('s'),
        KeyCode::KeyT => Some('t'),
        KeyCode::KeyU => Some('u'),
        KeyCode::KeyV => Some('v'),
        KeyCode::KeyW => Some('w'),
        KeyCode::KeyX => Some('x'),
        KeyCode::KeyY => Some('y'),
        KeyCode::KeyZ => Some('z'),
        _ => None,
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        let attrs = WindowAttributes::default()
            .with_title("Tova")
            .with_inner_size(winit::dpi::LogicalSize::new(1280, 720));

        let window = Arc::new(event_loop.create_window(attrs).unwrap());
        self.window = Some(window.clone());

        let state = pollster::block_on(RenderState::new(window.clone()));
        self.state = Some(state);
        if let Some(state) = &mut self.state {
            state.set_overlay_mode(OverlayMode::Title);
        }
        self.last_frame = Instant::now();
        self.release_cursor();

        match audio::AmbientAudio::start() {
            Ok(ambient) => self.ambient_audio = Some(ambient),
            Err(error) => log::warn!("Ambient wind disabled: {}", error),
        }

        self.update_title();
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),

            WindowEvent::Resized(size) => {
                if let Some(state) = &mut self.state {
                    state.resize(size);
                }
            }

            WindowEvent::CursorMoved { position, .. } => {
                self.mouse_pos = (position.x, position.y);
            }

            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: PhysicalKey::Code(key),
                        state: key_state,
                        ..
                    },
                ..
            } => {
                match key_state {
                    ElementState::Pressed => {
                        if self.typing_command {
                            // Command input mode
                            match key {
                                KeyCode::Enter => {
                                    self.execute_command();
                                    self.typing_command = false;
                                    self.update_title();
                                }
                                KeyCode::Escape => {
                                    self.typing_command = false;
                                    self.command_buffer.clear();
                                    self.update_title();
                                }
                                KeyCode::Backspace => {
                                    self.command_buffer.pop();
                                    self.update_title();
                                }
                                _ => {
                                    if let Some(c) = key_to_char(key) {
                                        self.command_buffer.push(c);
                                        self.update_title();
                                    }
                                }
                            }
                        } else if key == KeyCode::F6 {
                            if let Some(state) = &mut self.state {
                                let next = state.settings().preset.next();
                                state.set_quality_preset(next);
                                state.camera.speed = if self.god_mode { 90.0 } else { 30.0 };
                                log::info!("Quality preset set to {}", next.label());
                            }
                            self.update_title();
                        } else if key == KeyCode::F7 {
                            if let Some(state) = &mut self.state {
                                let enabled = !state.settings().shader_pack_enabled;
                                state.set_shader_pack_enabled(enabled);
                                log::info!(
                                    "Shader pack {}",
                                    if enabled { "enabled" } else { "disabled" }
                                );
                            }
                            self.update_title();
                        } else if key == KeyCode::F8 {
                            if let Some(state) = &mut self.state {
                                let enabled = !state.settings().vsync;
                                state.set_vsync(enabled);
                                log::info!(
                                    "VSync {}",
                                    if enabled { "enabled" } else { "disabled" }
                                );
                            }
                            self.update_title();
                        } else if self.title_screen {
                            if key == KeyCode::Enter || key == KeyCode::Space {
                                self.start_game_from_title();
                            } else if key == KeyCode::Escape {
                                event_loop.exit();
                            }
                        } else if key == KeyCode::Escape {
                            // Toggle pause
                            self.paused = !self.paused;
                            if self.paused {
                                self.release_cursor();
                                if let Some(state) = &mut self.state {
                                    state.set_overlay_mode(OverlayMode::Pause {
                                        god_mode: self.god_mode,
                                    });
                                }
                            } else {
                                self.grab_cursor();
                                if let Some(state) = &mut self.state {
                                    state.set_overlay_mode(OverlayMode::None);
                                }
                            }
                            self.update_title();
                        } else if key == KeyCode::Slash && self.cursor_grabbed && !self.paused {
                            // Enter command mode
                            self.typing_command = true;
                            self.command_buffer.clear();
                            self.update_title();
                        } else if !self.paused {
                            self.input.key_down(key);
                        }
                    }
                    ElementState::Released => {
                        self.input.key_up(key);
                    }
                }
            }

            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: MouseButton::Left,
                ..
            } => {
                if self.title_screen {
                    self.start_game_from_title();
                } else if self.paused {
                    // Check if click is on the god mode button
                    let mut should_resume = false;
                    if let Some(st) = &mut self.state {
                        let w = st.size.width as f64;
                        let h = st.size.height as f64;
                        if w > 0.0 && h > 0.0 {
                            let ndc_x = (self.mouse_pos.0 / w) * 2.0 - 1.0;
                            let ndc_y = 1.0 - (self.mouse_pos.1 / h) * 2.0;

                            if ndc_x >= BTN_LEFT as f64
                                && ndc_x <= BTN_RIGHT as f64
                                && ndc_y >= BTN_BOTTOM as f64
                                && ndc_y <= BTN_TOP as f64
                            {
                                // Toggle god mode
                                self.god_mode = !self.god_mode;
                                st.camera.speed = if self.god_mode { 90.0 } else { 30.0 };
                                st.set_overlay_mode(OverlayMode::Pause {
                                    god_mode: self.god_mode,
                                });
                                self.update_title();
                            } else {
                                // Click outside button — resume
                                self.paused = false;
                                st.set_overlay_mode(OverlayMode::None);
                                should_resume = true;
                            }
                        }
                    }
                    if should_resume {
                        self.grab_cursor();
                        self.update_title();
                    }
                } else if !self.cursor_grabbed {
                    self.grab_cursor();
                }
            }

            WindowEvent::RedrawRequested => {
                let now = Instant::now();
                let dt = (now - self.last_frame).as_secs_f32();
                self.last_frame = now;
                self.fps_accum_seconds += dt;
                self.fps_accum_frames += 1;
                if self.fps_accum_seconds >= 1.0 {
                    let fps =
                        self.fps_accum_frames as f32 / self.fps_accum_seconds.max(f32::EPSILON);
                    log::info!("FPS: {:.1}", fps);
                    self.fps_display = fps;
                    self.update_title();
                    self.fps_accum_seconds = 0.0;
                    self.fps_accum_frames = 0;
                }

                if let Some(state) = &mut self.state {
                    // Only move when playing
                    if !self.title_screen && !self.paused && !self.typing_command {
                        state.camera.fly_move(
                            dt,
                            MoveIntent {
                                forward: self.input.forward(),
                                back: self.input.back(),
                                left: self.input.left(),
                                right: self.input.right(),
                                up: self.input.up(),
                                down: self.input.down(),
                            },
                        );
                    }

                    state.update(dt);

                    match state.render(self.paused || self.title_screen) {
                        Ok(_) => {}
                        Err(wgpu::SurfaceError::Lost) => state.resize(state.size),
                        Err(wgpu::SurfaceError::OutOfMemory) => event_loop.exit(),
                        Err(e) => log::error!("Render error: {:?}", e),
                    }
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
            if self.cursor_grabbed && !self.title_screen && !self.paused && !self.typing_command {
                if let Some(state) = &mut self.state {
                    state.camera.rotate(dx, dy);
                }
            }
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }
}

fn main() {
    env_logger::init();

    let event_loop = EventLoop::new().unwrap();
    let mut app = App::new();
    event_loop.run_app(&mut app).unwrap();
}
