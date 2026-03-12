mod audio;
mod command;
mod hud;
mod inventory;
mod props;
mod renderer;
mod player;
mod time;
mod ui;
mod voxel;
mod weather;
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
use hud::FpsCounter;
use renderer::RenderState;
use player::{Input, Player};
use time::GameTime;
use inventory::Inventory;
use weather::{Weather, WeatherType};

/// Which page of the pause menu is shown.
#[derive(Clone, Copy, PartialEq)]
enum MenuPage { Main, Settings }

struct App {
    state: Option<RenderState>,
    input: Input,
    player: Player,
    audio: AudioSystem,
    weather: Weather,
    inventory: Inventory,
    game_time: GameTime,
    fps_counter: FpsCounter,
    last_frame: Instant,
    start_time: Instant,
    cursor_grabbed: bool,
    paused: bool,
    menu_page: MenuPage,
    fov_setting: u32,          // index into FOV_OPTIONS
    render_dist_setting: u32,  // index into DIST_OPTIONS
    command_input: Option<String>, // None = closed, Some = open with text
    mouse_pos: (f32, f32), // normalized 0..1
    window: Option<Arc<Window>>,
    screenshot_requested: bool,
    auto_screenshot_frames: i32, // countdown to auto-screenshot
    #[cfg(target_arch = "wasm32")]
    pending_state: Rc<RefCell<Option<RenderState>>>,
}

const FOV_OPTIONS: [u32; 4] = [60, 70, 80, 90];
const DIST_OPTIONS: [i32; 4] = [8, 10, 14, 18];
const SENS_OPTIONS: [f32; 4] = [0.0015, 0.003, 0.005, 0.008];
const SENS_LABELS: [&str; 4] = ["LOW", "MEDIUM", "HIGH", "VERY HIGH"];

impl App {
    fn new() -> Self {
        Self {
            state: None,
            input: Input::new(),
            player: Player::new(Vec3::new(64.0, 100.0, -55.0)),
            audio: AudioSystem::new(),
            weather: Weather::new(),
            inventory: Inventory::new(),
            game_time: GameTime::new(),
            fps_counter: FpsCounter::new(),
            last_frame: Instant::now(),
            start_time: Instant::now(),
            cursor_grabbed: false,
            paused: false,
            menu_page: MenuPage::Main,
            fov_setting: 1,         // 70 (default)
            render_dist_setting: 2, // 14 (default)
            command_input: None,
            mouse_pos: (0.5, 0.5),
            window: None,
            screenshot_requested: false,
            auto_screenshot_frames: 60, // auto-screenshot after 60 frames (~1 sec)
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

    fn execute_command(&mut self, input: &str) {
        use command::CommandResult;
        match command::parse_command(input) {
            CommandResult::ToggleGodMode => {
                self.player.god_mode = !self.player.god_mode;
                log::info!("God mode: {}", if self.player.god_mode { "ON" } else { "OFF" });
            }
            CommandResult::ToggleRain => {
                self.weather.toggle(WeatherType::Rain);
                log::info!("Rain toggled");
            }
            CommandResult::ToggleSnow => {
                self.weather.toggle(WeatherType::Snow);
                log::info!("Snow toggled");
            }
            CommandResult::Teleport(x, y, z) => {
                self.player.position = Vec3::new(x, y, z);
                log::info!("Teleported to ({}, {}, {})", x, y, z);
            }
            CommandResult::SetSpeed(s) => {
                self.player.walk_speed_override = Some(s);
                log::info!("Walk speed set to {}", s);
            }
            CommandResult::PrintPos => {
                let p = self.player.position;
                log::info!("Position: ({:.1}, {:.1}, {:.1})", p.x, p.y, p.z);
            }
            CommandResult::SetTime(t) => {
                self.game_time.set_time(t);
                log::info!("Time set to {:.1}", t);
            }
            CommandResult::Unknown(msg) => {
                if !msg.is_empty() {
                    log::warn!("{}", msg);
                }
            }
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
                        ref text,
                        ..
                    },
                ..
            } => match key_state {
                ElementState::Pressed => {
                    // ─── Command palette input mode ───────────────
                    if let Some(ref mut cmd) = self.command_input {
                        match key {
                            KeyCode::Escape => {
                                // Close command palette without executing
                                self.command_input = None;
                                self.grab_cursor();
                            }
                            KeyCode::Enter => {
                                // Execute command and close
                                let input = cmd.clone();
                                self.command_input = None;
                                self.grab_cursor();
                                self.execute_command(&input);
                            }
                            KeyCode::Backspace => {
                                cmd.pop();
                            }
                            _ => {
                                // Append typed text
                                if let Some(txt) = text {
                                    for ch in txt.chars() {
                                        if !ch.is_control() {
                                            cmd.push(ch);
                                        }
                                    }
                                }
                            }
                        }
                    }
                    // ─── Normal game input ────────────────────────
                    else {
                        if key == KeyCode::Escape {
                            if self.cursor_grabbed {
                                self.release_cursor();
                                self.paused = true;
                                self.menu_page = MenuPage::Main;
                            } else if self.paused {
                                if self.menu_page == MenuPage::Settings {
                                    self.menu_page = MenuPage::Main;
                                } else {
                                    self.paused = false;
                                    self.menu_page = MenuPage::Main;
                                    self.grab_cursor();
                                }
                            }
                        }
                        // Open command palette with `/` or `T`
                        if (key == KeyCode::Slash || key == KeyCode::KeyT) && self.cursor_grabbed && !self.paused {
                            self.command_input = Some(if key == KeyCode::Slash { "/".to_string() } else { String::new() });
                            self.release_cursor();
                        }
                        else if key == KeyCode::KeyR && self.cursor_grabbed {
                            self.weather.toggle(WeatherType::Rain);
                        }
                        else if key == KeyCode::KeyN && self.cursor_grabbed {
                            self.weather.toggle(WeatherType::Snow);
                        }
                        // Hotbar slot selection (1-9)
                        if self.cursor_grabbed && self.command_input.is_none() {
                            match key {
                                KeyCode::Digit1 => self.inventory.select(0),
                                KeyCode::Digit2 => self.inventory.select(1),
                                KeyCode::Digit3 => self.inventory.select(2),
                                KeyCode::Digit4 => self.inventory.select(3),
                                KeyCode::Digit5 => self.inventory.select(4),
                                KeyCode::Digit6 => self.inventory.select(5),
                                KeyCode::Digit7 => self.inventory.select(6),
                                KeyCode::Digit8 => self.inventory.select(7),
                                KeyCode::Digit9 => self.inventory.select(8),
                                _ => {}
                            }
                        }
                        if key == KeyCode::F12 {
                            self.screenshot_requested = true;
                        }
                        if !self.paused && self.command_input.is_none() {
                            self.input.key_down(key);
                        }
                    }
                }
                ElementState::Released => {
                    self.input.key_up(key);
                }
            },

            WindowEvent::MouseWheel { delta, .. } => {
                if self.cursor_grabbed && !self.paused && self.command_input.is_none() {
                    let scroll = match delta {
                        winit::event::MouseScrollDelta::LineDelta(_, y) => y as i32,
                        winit::event::MouseScrollDelta::PixelDelta(pos) => {
                            if pos.y > 0.0 { 1 } else if pos.y < 0.0 { -1 } else { 0 }
                        }
                    };
                    if scroll != 0 {
                        self.inventory.scroll(scroll);
                    }
                }
            }

            WindowEvent::CursorMoved { position, .. } => {
                if self.paused {
                    if let Some(window) = &self.window {
                        let size = window.inner_size();
                        self.mouse_pos = (
                            position.x as f32 / size.width as f32,
                            position.y as f32 / size.height as f32,
                        );
                    }
                }
            }

            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                ..
            } => {
                if self.paused {
                    let (mx, my) = self.mouse_pos;
                    let in_btn = |y0: f32, y1: f32| -> bool {
                        let uv_top = (1.0 - y1) / 2.0;
                        let uv_bot = (1.0 - y0) / 2.0;
                        mx > 0.36 && mx < 0.64 && my > uv_top && my < uv_bot
                    };
                    match self.menu_page {
                        MenuPage::Main => {
                            if in_btn(0.06, 0.14) {
                                self.player.god_mode = !self.player.god_mode;
                            } else if in_btn(-0.04, 0.04) {
                                self.menu_page = MenuPage::Settings;
                            } else if in_btn(-0.14, -0.06) {
                                self.paused = false;
                                self.menu_page = MenuPage::Main;
                                self.grab_cursor();
                            }
                        }
                        MenuPage::Settings => {
                            if in_btn(0.08, 0.14) {
                                self.fov_setting = (self.fov_setting + 1) % FOV_OPTIONS.len() as u32;
                            } else if in_btn(0.00, 0.06) {
                                self.render_dist_setting = (self.render_dist_setting + 1) % DIST_OPTIONS.len() as u32;
                                if let Some(state) = &mut self.state {
                                    state.set_render_distance(DIST_OPTIONS[self.render_dist_setting as usize]);
                                }
                            } else if in_btn(-0.08, -0.02) {
                                let idx = SENS_OPTIONS.iter().position(|&s| (s - self.player.sensitivity).abs() < 0.0001).unwrap_or(1);
                                self.player.sensitivity = SENS_OPTIONS[(idx + 1) % SENS_OPTIONS.len()];
                            } else if in_btn(-0.18, -0.10) {
                                self.menu_page = MenuPage::Main;
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
                self.fps_counter.record(dt);

                if let Some(state) = &mut self.state {
                    // Sync UI state to renderer
                    state.paused = self.paused;
                    state.god_mode = self.player.god_mode;
                    state.command_text = self.command_input.clone();
                    state.menu_page = match self.menu_page { MenuPage::Main => 0, MenuPage::Settings => 1 };
                    state.fov_setting = self.fov_setting;
                    state.render_dist_setting = self.render_dist_setting;
                    let sens_idx = SENS_OPTIONS.iter().position(|&s| (s - self.player.sensitivity).abs() < 0.0001).unwrap_or(1);
                    state.sensitivity_label = SENS_LABELS[sens_idx].into();
                    state.mouse_uv = [self.mouse_pos.0, self.mouse_pos.1];

                    // Advance game time
                    self.game_time.update(dt);

                    // Update sun from game time
                    let sun_dir = self.game_time.sun_direction();
                    let sun_color = self.game_time.sun_color();
                    let ambient = self.game_time.ambient_level();
                    state.update_sun(sun_dir, sun_color, ambient);

                    // Update sky zenith color from time
                    let zenith = self.game_time.sky_zenith();
                    state.set_sky_zenith(zenith);

                    // Player physics — reads chunk data for collision
                    if !self.paused {
                        self.player.update(dt, &self.input, &state.chunk_manager);
                    }

                    // Footstep audio
                    if let Some(step_id) = self.player.take_step() {
                        self.audio.play_footstep(step_id);
                    }

                    // Stream chunks around player
                    let (pcx, pcz) = self.player.chunk_pos();
                    state.update_chunks(pcx, pcz);

                    // Sync camera to player eye position + head bob + shake
                    let cam_pos = self.player.eye_position()
                        + self.player.head_bob_offset();

                    // Landing camera shake (pitch kick)
                    let shake = self.player.landing_shake();

                    state.camera.position = cam_pos;
                    state.camera.yaw = self.player.yaw;
                    state.camera.pitch = self.player.pitch - shake;

                    // Sprint FOV zoom (smooth lerp)
                    let base_fov = (FOV_OPTIONS[self.fov_setting as usize] as f32).to_radians();
                    let target_fov = if self.player.sprinting {
                        base_fov + 10.0_f32.to_radians()
                    } else {
                        base_fov
                    };
                    state.camera.fov_y += (target_fov - state.camera.fov_y)
                        * (8.0 * dt).min(1.0);

                    state.update_camera();

                    // Weather system
                    self.weather.update(dt);
                    let time_fog = self.game_time.fog_multiplier();
                    let (wind_x, wind_z) = self.weather.wind_dir();
                    state.update_weather(
                        self.weather.type_f32(),
                        self.weather.intensity,
                        self.weather.time,
                        self.weather.fog_multiplier() * time_fog,
                        self.weather.sky_darken(),
                        wind_x,
                        wind_z,
                        self.weather.wind_strength,
                        self.weather.wind_gust_intensity,
                        self.weather.wind_turbulence,
                        self.game_time.sky_zenith(),
                        self.game_time.sky_horizon(),
                        self.game_time.sky_horizon_sun(),
                        self.game_time.sky_nadir(),
                    );

                    // Build HUD overlay
                    let hud_state = hud::HudState {
                        fps: self.fps_counter.fps(),
                        pos_x: self.player.position.x,
                        pos_y: self.player.position.y,
                        pos_z: self.player.position.z,
                        yaw: self.player.yaw,
                        god_mode: self.player.god_mode,
                        time_str: self.game_time.format_time(),
                        period: self.game_time.period_name(),
                        aspect: state.camera.aspect,
                    };
                    let mut hud_verts = hud::build_hud(&hud_state);
                    // Append hotbar to HUD geometry
                    hud_verts.extend(inventory::build_hotbar(
                        self.inventory.selected,
                        state.camera.aspect,
                    ));
                    state.hud_vertices = Some(hud_verts);

                    // Update existing shader HUD uniform
                    state.update_hud(
                        self.player.yaw,
                        self.player.stamina,
                        self.player.god_mode,
                        self.start_time.elapsed().as_secs_f32(),
                    );

                    // Reposition ocean plane to follow camera
                    state.update_ocean(
                        self.player.position.x,
                        self.player.position.z,
                    );

                    match state.render() {
                        Ok(_) => {
                            // Auto-screenshot countdown
                            if self.auto_screenshot_frames > 0 {
                                self.auto_screenshot_frames -= 1;
                                if self.auto_screenshot_frames == 0 {
                                    state.screenshot("/tmp/tova_auto.png");
                                }
                            }
                            // Manual screenshot
                            if self.screenshot_requested {
                                state.screenshot("/tmp/tova_screenshot.png");
                                self.screenshot_requested = false;
                            }
                        }
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
