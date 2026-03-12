// ═══════════════════════════════════════════════════════════════
//  HUD — heads-up display overlay
//  FPS, coordinates, compass, time, god-mode indicator
//  Rendered as pixel-font geometry (no font atlas)
// ═══════════════════════════════════════════════════════════════

use crate::ui::{self, UiVertex};

/// All the data the HUD needs to render a frame.
pub struct HudState {
    pub fps: u32,
    pub pos_x: f32,
    pub pos_y: f32,
    pub pos_z: f32,
    pub yaw: f32,
    pub god_mode: bool,
    pub time_str: String,
    pub period: &'static str,
    pub aspect: f32,
}

/// Build HUD overlay geometry.
pub fn build_hud(state: &HudState) -> Vec<UiVertex> {
    let mut verts = Vec::new();
    let ax = 1.0 / state.aspect;

    let dim = [0.65, 0.62, 0.56, 0.55f32]; // dim text color
    let bright = [0.82, 0.80, 0.74, 0.7];  // brighter text
    let accent = [0.55, 0.72, 0.55, 0.7];  // green accent

    // ─── Top-left: FPS ───────────────────────────────────────
    let fps_text = format!("{} FPS", state.fps);
    ui::render_text(&mut verts, &fps_text, -0.97, 0.93, 0.9, ax, dim);

    // ─── Top-left: Coordinates (below FPS) ───────────────────
    let coord_text = format!(
        "{:.0} {:.0} {:.0}",
        state.pos_x, state.pos_y, state.pos_z
    );
    ui::render_text(&mut verts, &coord_text, -0.97, 0.89, 0.9, ax, dim);

    // ─── Top-left: Time + period (below coords) ─────────────
    let time_text = format!("{} {}", state.time_str, state.period);
    ui::render_text(&mut verts, &time_text, -0.97, 0.85, 0.9, ax, dim);

    // ─── Top-right: God mode indicator ──────────────────────
    if state.god_mode {
        let god_text = "GOD";
        let w = ui::text_width(god_text, 0.9, ax);
        ui::render_text(&mut verts, god_text, 0.97 - w, 0.93, 0.9, ax, accent);
    }

    // ─── Bottom-center: Compass ─────────────────────────────
    let compass = build_compass_text(state.yaw);
    let w = ui::text_width(&compass, 0.8, ax);
    ui::render_text(&mut verts, &compass, -w * 0.5, -0.94, 0.8, ax, bright);

    verts
}

/// Build compass string from yaw. Shows cardinal + intercardinal directions.
fn build_compass_text(yaw: f32) -> String {
    // Yaw: 0 = -Z (north), PI/2 = -X (west), etc.
    // Normalize to 0-360 degrees
    let deg = (yaw.to_degrees()).rem_euclid(360.0);

    // Compass points every 45 degrees
    let points = ["N", "NE", "E", "SE", "S", "SW", "W", "NW"];
    let idx = ((deg + 22.5) / 45.0).floor() as usize % 8;
    let heading = points[idx];

    format!("-- {} {:.0} --", heading, deg)
}

/// Smoothed FPS counter (tracks rolling average).
pub struct FpsCounter {
    frame_times: [f32; 60],
    index: usize,
    count: usize,
}

impl FpsCounter {
    pub fn new() -> Self {
        Self {
            frame_times: [0.0; 60],
            index: 0,
            count: 0,
        }
    }

    /// Record a frame with the given delta time.
    pub fn record(&mut self, dt: f32) {
        self.frame_times[self.index] = dt;
        self.index = (self.index + 1) % 60;
        if self.count < 60 {
            self.count += 1;
        }
    }

    /// Get smoothed FPS.
    pub fn fps(&self) -> u32 {
        if self.count == 0 {
            return 0;
        }
        let sum: f32 = self.frame_times[..self.count].iter().sum();
        let avg = sum / self.count as f32;
        if avg > 0.0 {
            (1.0 / avg) as u32
        } else {
            0
        }
    }
}
