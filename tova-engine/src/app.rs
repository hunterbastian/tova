#[cfg(target_arch = "wasm32")]
use std::cell::RefCell;
#[cfg(target_arch = "wasm32")]
use std::rc::Rc;
use std::sync::Arc;

use glam::IVec3;
use web_time::Instant;
use winit::application::ApplicationHandler;
use winit::event::{
    DeviceEvent, ElementState, KeyEvent, MouseButton, MouseScrollDelta, WindowEvent,
};
use winit::event_loop::ActiveEventLoop;
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{CursorGrabMode, Window, WindowAttributes, WindowId};

use crate::camera::Camera;
use crate::graphics::GraphicsState;
use crate::hud::HudView;
use crate::input::InputState;
use crate::voxel::block::Block;
use crate::voxel::{VoxelWorld, WorldEdit, DEFAULT_WORLD_RADIUS};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsCast;
#[cfg(target_arch = "wasm32")]
use winit::platform::web::WindowAttributesExtWebSys;

const BLOCK_REACH: f32 = 8.0;
const RAY_STEP: f32 = 0.1;
const HOTBAR: [Block; 5] = [
    Block::Dirt,
    Block::Stone,
    Block::Grass,
    Block::Sand,
    Block::Cobble,
];

#[derive(Clone, Copy, Debug)]
struct BlockHit {
    block: Block,
    block_pos: IVec3,
    place_pos: IVec3,
}

pub struct TovaApp {
    window: Option<Arc<Window>>,
    graphics: Option<GraphicsState>,
    #[cfg(target_arch = "wasm32")]
    pending_graphics: Option<Rc<RefCell<Option<GraphicsState>>>>,
    world: Option<VoxelWorld>,
    camera: Camera,
    input: InputState,
    cursor_captured: bool,
    selected_slot: usize,
    last_frame: Instant,
    fps_accum_seconds: f32,
    fps_accum_frames: u32,
    fps_display: f32,
    status_message: Option<String>,
    status_seconds: f32,
}

impl Default for TovaApp {
    fn default() -> Self {
        Self {
            window: None,
            graphics: None,
            #[cfg(target_arch = "wasm32")]
            pending_graphics: None,
            world: None,
            camera: Camera::new(16.0 / 9.0),
            input: InputState::default(),
            cursor_captured: false,
            selected_slot: 0,
            last_frame: Instant::now(),
            fps_accum_seconds: 0.0,
            fps_accum_frames: 0,
            fps_display: 0.0,
            status_message: None,
            status_seconds: 0.0,
        }
    }
}

impl TovaApp {
    fn selected_block(&self) -> Block {
        HOTBAR[self.selected_slot]
    }

    fn update_title(&self) {
        let Some(window) = &self.window else {
            return;
        };

        let title = format!(
            "Tova Rebuilt v0.1 | {} | updated {}",
            rustc_version_label(),
            rust_updated_at_label(),
        );
        window.set_title(&title);
    }

    fn set_status<S>(&mut self, message: S)
    where
        S: Into<String>,
    {
        self.status_message = Some(message.into());
        self.status_seconds = 2.5;
        self.update_title();
    }

    fn update_status_message(&mut self, dt: f32) {
        if self.status_seconds <= 0.0 {
            return;
        }

        self.status_seconds = (self.status_seconds - dt).max(0.0);
        if self.status_seconds <= 0.0 {
            self.status_message = None;
            self.update_title();
        }
    }

    fn update_fps(&mut self, dt: f32) {
        self.fps_accum_seconds += dt;
        self.fps_accum_frames += 1;

        if self.fps_accum_seconds >= 1.0 {
            self.fps_display = self.fps_accum_frames as f32 / self.fps_accum_seconds.max(0.001);
            self.fps_accum_seconds = 0.0;
            self.fps_accum_frames = 0;
            self.update_title();
        }
    }

    fn capture_cursor(&mut self) {
        let Some(window) = &self.window else {
            return;
        };

        let _ = window
            .set_cursor_grab(CursorGrabMode::Locked)
            .or_else(|_| window.set_cursor_grab(CursorGrabMode::Confined));
        window.set_cursor_visible(false);
        self.cursor_captured = true;
        self.update_title();
    }

    fn release_cursor(&mut self) {
        let Some(window) = &self.window else {
            return;
        };

        let _ = window.set_cursor_grab(CursorGrabMode::None);
        window.set_cursor_visible(true);
        self.cursor_captured = false;
        self.input.clear();
        self.update_title();
    }

    fn rebuild_world_meshes(&mut self) {
        if let (Some(world), Some(graphics)) = (&self.world, &mut self.graphics) {
            graphics.rebuild_world(world);
        }
    }

    #[cfg(target_arch = "wasm32")]
    fn maybe_finish_graphics_init(&mut self) {
        if self.graphics.is_some() {
            return;
        }

        let Some(slot) = self.pending_graphics.as_ref() else {
            return;
        };
        let Some(graphics) = slot.borrow_mut().take() else {
            return;
        };

        self.graphics = Some(graphics);
        self.pending_graphics = None;
        set_browser_loading_state("ready");
        self.set_status("Browser world ready");
    }

    fn rebuild_dirty_chunks(&mut self, dirty_chunks: &[(i32, i32)]) {
        if let (Some(world), Some(graphics)) = (&self.world, &mut self.graphics) {
            graphics.rebuild_chunks(world, dirty_chunks);
        }
    }

    fn apply_world_edit(&mut self, edit: WorldEdit) {
        self.rebuild_dirty_chunks(&edit.dirty_chunks);
    }

    fn reset_world(&mut self) {
        let world = VoxelWorld::generate(DEFAULT_WORLD_RADIUS);
        self.world = Some(world);
        self.camera = Camera::new(self.camera.aspect());
        self.rebuild_world_meshes();
        self.set_status("Generated new frontier");
    }

    fn raycast_target(&self) -> Option<BlockHit> {
        let world = self.world.as_ref()?;
        let origin = self.camera.position + self.camera.forward() * 0.35;
        let direction = self.camera.forward();
        let steps = (BLOCK_REACH / RAY_STEP).ceil() as i32;
        let mut last_replaceable = VoxelWorld::block_coords(origin);
        let mut last_cell = None;

        for step in 0..=steps {
            let sample_pos = origin + direction * (step as f32 * RAY_STEP);
            let block_pos = VoxelWorld::block_coords(sample_pos);

            if last_cell == Some(block_pos) {
                continue;
            }
            last_cell = Some(block_pos);

            let block = world.sample_block(block_pos);
            if block.is_replaceable() {
                last_replaceable = block_pos;
                continue;
            }

            return Some(BlockHit {
                block,
                block_pos,
                place_pos: last_replaceable,
            });
        }

        None
    }

    fn mine_targeted_block(&mut self) {
        let Some(hit) = self.raycast_target() else {
            return;
        };
        if !hit.block.is_collectible() {
            return;
        }

        let edit = self
            .world
            .as_mut()
            .and_then(|world| world.set_block(hit.block_pos, Block::Air));

        if let Some(edit) = edit {
            self.apply_world_edit(edit);
            self.set_status(format!("Mined {}", hit.block.display_name()));
        }
    }

    fn place_selected_block(&mut self) {
        let Some(hit) = self.raycast_target() else {
            self.set_status("No placement surface in reach");
            return;
        };

        let block = self.selected_block();
        if self.camera.occupies_block(hit.place_pos) {
            self.set_status("Cannot place inside player");
            return;
        }

        let replaceable = self
            .world
            .as_ref()
            .is_some_and(|world| world.sample_block(hit.place_pos).is_replaceable());
        if !replaceable {
            self.set_status("Space occupied");
            return;
        }

        let edit = self
            .world
            .as_mut()
            .and_then(|world| world.set_block(hit.place_pos, block));
        if let Some(edit) = edit {
            self.apply_world_edit(edit);
            self.set_status(format!("Placed {}", block.display_name()));
        }
    }

    fn select_slot(&mut self, slot: usize) {
        if slot >= HOTBAR.len() {
            return;
        }

        self.selected_slot = slot;
        self.set_status(format!("Selected {}", self.selected_block().display_name()));
    }

    fn cycle_slot(&mut self, direction: i32) {
        let len = HOTBAR.len() as i32;
        let next = (self.selected_slot as i32 + direction).rem_euclid(len);
        self.selected_slot = next as usize;
        self.set_status(format!("Selected {}", self.selected_block().display_name()));
    }

    fn handle_key_pressed(&mut self, event_loop: &ActiveEventLoop, key: KeyCode) {
        match key {
            KeyCode::Escape => {
                if self.cursor_captured {
                    self.release_cursor();
                } else if cfg!(target_arch = "wasm32") {
                    self.set_status("Browser build stays open");
                } else {
                    event_loop.exit();
                }
            }
            KeyCode::Enter => self.capture_cursor(),
            KeyCode::KeyR => self.reset_world(),
            KeyCode::Digit1 => self.select_slot(0),
            KeyCode::Digit2 => self.select_slot(1),
            KeyCode::Digit3 => self.select_slot(2),
            KeyCode::Digit4 => self.select_slot(3),
            KeyCode::Digit5 => self.select_slot(4),
            _ if self.cursor_captured => self.input.key_down(key),
            _ => {}
        }
    }

    fn handle_redraw_requested(&mut self, event_loop: &ActiveEventLoop) {
        #[cfg(target_arch = "wasm32")]
        self.maybe_finish_graphics_init();

        let now = Instant::now();
        let dt = (now - self.last_frame).as_secs_f32();
        self.last_frame = now;

        self.update_status_message(dt);
        self.update_fps(dt);

        if self.cursor_captured {
            if let Some(world) = &self.world {
                self.camera.update(dt, self.input.move_intent(), world);
            }
        }

        let hud = HudView {
            hotbar: &HOTBAR,
            selected_slot: self.selected_slot,
            status_message: self.status_message.as_deref(),
            cursor_captured: self.cursor_captured,
            health: 1.0,
            magicka: 0.92,
            fatigue: self.camera.fatigue(),
        };
        let Some(graphics) = &mut self.graphics else {
            return;
        };

        match graphics.render(&self.camera, hud) {
            Ok(()) => {}
            Err(wgpu::SurfaceError::Lost) => graphics.resize(graphics.size),
            Err(wgpu::SurfaceError::OutOfMemory) => event_loop.exit(),
            Err(error) => log::error!("render error: {error:?}"),
        }
    }
}

impl ApplicationHandler for TovaApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        let attrs = browser_window_attributes(
            WindowAttributes::default()
            .with_title("Tova Rebuilt")
            .with_inner_size(winit::dpi::LogicalSize::new(1280, 720)),
        );
        let window = Arc::new(
            event_loop
                .create_window(attrs)
                .expect("failed to create window"),
        );
        prepare_window_for_platform(&window);
        #[cfg(target_arch = "wasm32")]
        set_browser_loading_state("loading");

        let aspect = window.inner_size().width as f32 / window.inner_size().height.max(1) as f32;
        let world = VoxelWorld::generate(DEFAULT_WORLD_RADIUS);
        self.window = Some(window);
        self.world = Some(world);
        self.camera = Camera::new(aspect);
        self.last_frame = Instant::now();
        #[cfg(not(target_arch = "wasm32"))]
        {
            let graphics = pollster::block_on(GraphicsState::new(
                self.window.as_ref().expect("window missing").clone(),
                self.world.as_ref().expect("world missing"),
            ));
            self.graphics = Some(graphics);
            self.set_status("Spawn clearing ready");
        }
        #[cfg(target_arch = "wasm32")]
        {
            let slot = Rc::new(RefCell::new(None));
            let world_for_gpu = self.world.as_ref().expect("world missing").clone();
            let window_for_gpu = self.window.as_ref().expect("window missing").clone();
            let slot_for_future = Rc::clone(&slot);
            wasm_bindgen_futures::spawn_local(async move {
                let graphics = GraphicsState::new(window_for_gpu, &world_for_gpu).await;
                *slot_for_future.borrow_mut() = Some(graphics);
            });
            self.pending_graphics = Some(slot);
            self.set_status("Loading browser renderer");
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Focused(false) => self.release_cursor(),
            WindowEvent::Resized(size) => {
                if let Some(graphics) = &mut self.graphics {
                    graphics.resize(size);
                }
                if size.width > 0 && size.height > 0 {
                    self.camera
                        .set_aspect(size.width as f32 / size.height.max(1) as f32);
                }
            }
            WindowEvent::RedrawRequested => self.handle_redraw_requested(event_loop),
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: PhysicalKey::Code(key),
                        state,
                        ..
                    },
                ..
            } => match state {
                ElementState::Pressed => self.handle_key_pressed(event_loop, key),
                ElementState::Released => self.input.key_up(key),
            },
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button,
                ..
            } => match button {
                MouseButton::Left => {
                    if self.cursor_captured {
                        self.mine_targeted_block();
                    } else {
                        self.capture_cursor();
                    }
                }
                MouseButton::Right if self.cursor_captured => self.place_selected_block(),
                _ => {}
            },
            WindowEvent::MouseWheel { delta, .. } if self.cursor_captured => {
                let direction = match delta {
                    MouseScrollDelta::LineDelta(_, y) => y.signum() as i32,
                    MouseScrollDelta::PixelDelta(position) => position.y.signum() as i32,
                };

                if direction != 0 {
                    self.cycle_slot(-direction);
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
            if self.cursor_captured {
                self.camera.rotate(dx, dy);
            }
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(window) = self.window.as_ref().cloned() {
            #[cfg(target_arch = "wasm32")]
            {
                self.maybe_finish_graphics_init();
                if let Some(size) = sync_browser_canvas(&window) {
                    if let Some(graphics) = &mut self.graphics {
                        if graphics.size != size {
                            graphics.resize(size);
                        }
                    }
                    if size.width > 0 && size.height > 0 {
                        self.camera
                            .set_aspect(size.width as f32 / size.height.max(1) as f32);
                    }
                }
            }
            window.request_redraw();
        }
    }
}

fn rustc_version_label() -> &'static str {
    option_env!("TOVA_RUSTC_VERSION").unwrap_or("rustc unknown")
}

fn rust_updated_at_label() -> &'static str {
    option_env!("TOVA_RUST_UPDATED_AT").unwrap_or("unknown")
}

#[cfg(target_arch = "wasm32")]
fn browser_window_attributes(attrs: WindowAttributes) -> WindowAttributes {
    let Some(browser_window) = web_sys::window() else {
        return attrs.with_append(true).with_prevent_default(true);
    };
    let Some(document) = browser_window.document() else {
        return attrs.with_append(true).with_prevent_default(true);
    };

    let maybe_canvas = document
        .get_element_by_id("tova-canvas")
        .and_then(|element| element.dyn_into::<web_sys::HtmlCanvasElement>().ok());

    attrs
        .with_canvas(maybe_canvas)
        .with_append(true)
        .with_prevent_default(true)
        .with_focusable(true)
}

#[cfg(not(target_arch = "wasm32"))]
fn browser_window_attributes(attrs: WindowAttributes) -> WindowAttributes {
    attrs
}

#[cfg(target_arch = "wasm32")]
fn prepare_window_for_platform(window: &Window) {
    use winit::platform::web::WindowExtWebSys;

    window.set_prevent_default(true);
    let _ = sync_browser_canvas(window);

    if let Some(canvas) = window.canvas() {
        let _ = canvas.set_attribute("id", "tova-canvas");
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn prepare_window_for_platform(_window: &Window) {}

#[cfg(target_arch = "wasm32")]
fn sync_browser_canvas(window: &Window) -> Option<winit::dpi::PhysicalSize<u32>> {
    use winit::platform::web::WindowExtWebSys;

    let Some(browser_window) = web_sys::window() else {
        return None;
    };
    let Some(width) = browser_window.inner_width().ok().and_then(|v| v.as_f64()) else {
        return None;
    };
    let Some(height) = browser_window.inner_height().ok().and_then(|v| v.as_f64()) else {
        return None;
    };
    let scale = browser_window.device_pixel_ratio().max(1.0);
    let physical_size = winit::dpi::PhysicalSize::new(
        (width * scale).round().max(1.0) as u32,
        (height * scale).round().max(1.0) as u32,
    );

    if let Some(canvas) = window.canvas() {
        canvas.set_width(physical_size.width);
        canvas.set_height(physical_size.height);
        let _ = canvas.set_attribute(
            "style",
            &format!(
                "width:{width}px;height:{height}px;display:block;outline:none;touch-action:none;"
            ),
        );
        let _ = canvas.set_attribute("tabindex", "0");
    }

    let _ = window.request_inner_size(winit::dpi::LogicalSize::new(width, height));
    Some(physical_size)
}

#[cfg(target_arch = "wasm32")]
fn set_browser_loading_state(state: &str) {
    let Some(browser_window) = web_sys::window() else {
        return;
    };
    let Some(document) = browser_window.document() else {
        return;
    };
    let Some(shell) = document.get_element_by_id("tova-loading") else {
        return;
    };

    let _ = shell.set_attribute("data-state", state);
}

#[cfg(test)]
mod tests {
    use super::HOTBAR;

    #[test]
    fn hotbar_has_five_slots() {
        assert_eq!(HOTBAR.len(), 5);
    }
}
