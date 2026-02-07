mod renderer;
mod player;
mod voxel;

use std::sync::Arc;
use std::time::Instant;
use winit::application::ApplicationHandler;
use winit::event::{DeviceEvent, ElementState, KeyEvent, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{CursorGrabMode, Window, WindowAttributes, WindowId};

use renderer::state::{BTN_BOTTOM, BTN_LEFT, BTN_RIGHT, BTN_TOP};
use renderer::RenderState;
use player::Input;

struct App {
    state: Option<RenderState>,
    input: Input,
    last_frame: Instant,
    cursor_grabbed: bool,
    window: Option<Arc<Window>>,
    // Game state
    paused: bool,
    god_mode: bool,
    fog_enabled: bool,
    typing_command: bool,
    command_buffer: String,
    mouse_pos: (f64, f64),
}

impl App {
    fn new() -> Self {
        Self {
            state: None,
            input: Input::new(),
            last_frame: Instant::now(),
            cursor_grabbed: false,
            window: None,
            paused: false,
            god_mode: false,
            fog_enabled: true,
            typing_command: false,
            command_buffer: String::new(),
            mouse_pos: (0.0, 0.0),
        }
    }

    fn grab_cursor(&mut self) {
        if let Some(window) = &self.window {
            let _ = window.set_cursor_grab(CursorGrabMode::Locked)
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
            let title = if self.typing_command {
                format!("Tova — /{}", self.command_buffer)
            } else if self.paused {
                let god = if self.god_mode { "ON" } else { "OFF" };
                format!("Tova — PAUSED | God Mode: {}", god)
            } else {
                let mut flags = Vec::new();
                if self.god_mode { flags.push("GOD"); }
                if !self.fog_enabled { flags.push("NO FOG"); }
                if flags.is_empty() {
                    "Tova".to_string()
                } else {
                    format!("Tova — {}", flags.join(" | "))
                }
            };
            window.set_title(&title);
        }
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
        self.last_frame = Instant::now();
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
                        } else if key == KeyCode::Escape {
                            // Toggle pause
                            self.paused = !self.paused;
                            if self.paused {
                                self.release_cursor();
                                if let Some(state) = &mut self.state {
                                    state.update_overlay(self.god_mode);
                                }
                            } else {
                                self.grab_cursor();
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
                ..
            } => {
                if self.paused {
                    // Check if click is on the god mode button
                    if let Some(st) = &mut self.state {
                        let w = st.size.width as f64;
                        let h = st.size.height as f64;
                        if w > 0.0 && h > 0.0 {
                            let ndc_x = (self.mouse_pos.0 / w) * 2.0 - 1.0;
                            let ndc_y = 1.0 - (self.mouse_pos.1 / h) * 2.0;

                            if ndc_x >= BTN_LEFT as f64 && ndc_x <= BTN_RIGHT as f64
                                && ndc_y >= BTN_BOTTOM as f64 && ndc_y <= BTN_TOP as f64
                            {
                                // Toggle god mode
                                self.god_mode = !self.god_mode;
                                st.camera.speed = if self.god_mode { 60.0 } else { 20.0 };
                                st.update_overlay(self.god_mode);
                                self.update_title();
                            } else {
                                // Click outside button — resume
                                self.paused = false;
                                self.grab_cursor();
                                self.update_title();
                            }
                        }
                    }
                } else if !self.cursor_grabbed {
                    self.grab_cursor();
                }
            }

            WindowEvent::RedrawRequested => {
                let now = Instant::now();
                let dt = (now - self.last_frame).as_secs_f32();
                self.last_frame = now;

                if let Some(state) = &mut self.state {
                    // Only move when playing
                    if !self.paused && !self.typing_command {
                        state.camera.fly_move(
                            dt,
                            self.input.forward(),
                            self.input.back(),
                            self.input.left(),
                            self.input.right(),
                            self.input.up(),
                            self.input.down(),
                        );
                    }

                    state.update_camera();

                    match state.render(self.paused) {
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
            if self.cursor_grabbed && !self.paused {
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
