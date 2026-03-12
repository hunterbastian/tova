// ═══════════════════════════════════════════════════════════════
//  Inventory & Hotbar — block selection toolbar
//  Renders at the bottom center of the screen
// ═══════════════════════════════════════════════════════════════

use crate::ui::{self, UiVertex};
use crate::voxel::block::{Block, BLOCK_COLORS};

/// Number of hotbar slots.
pub const HOTBAR_SIZE: usize = 9;

/// The placeable block types in order.
const HOTBAR_BLOCKS: [Block; HOTBAR_SIZE] = [
    Block::Grass,
    Block::Dirt,
    Block::Stone,
    Block::Sand,
    Block::Cobble,
    Block::Wood,
    Block::Leaves,
    Block::Gravel,
    Block::Water,
];

/// Block names for display.
const BLOCK_NAMES: [&str; HOTBAR_SIZE] = [
    "GRASS", "DIRT", "STONE", "SAND", "COBBLE",
    "WOOD", "LEAVES", "GRAVEL", "WATER",
];

pub struct Inventory {
    pub selected: usize,
}

impl Inventory {
    pub fn new() -> Self {
        Self { selected: 0 }
    }

    /// Select slot by index (0-8).
    pub fn select(&mut self, slot: usize) {
        if slot < HOTBAR_SIZE {
            self.selected = slot;
        }
    }

    /// Scroll selection (positive = right, negative = left).
    pub fn scroll(&mut self, delta: i32) {
        let new = (self.selected as i32 + delta).rem_euclid(HOTBAR_SIZE as i32);
        self.selected = new as usize;
    }

    /// Get the currently selected block.
    pub fn selected_block(&self) -> Block {
        HOTBAR_BLOCKS[self.selected]
    }
}

/// Build hotbar overlay geometry.
/// Returns UiVertex triangles to draw with the menu pipeline.
pub fn build_hotbar(selected: usize, aspect: f32) -> Vec<UiVertex> {
    let mut verts = Vec::new();
    let ax = 1.0 / aspect;

    // ─── Layout constants ────────────────────────────────────
    let slot_size = 0.055;          // NDC width per slot
    let slot_gap = 0.006;           // gap between slots
    let total_w = (HOTBAR_SIZE as f32) * slot_size + (HOTBAR_SIZE as f32 - 1.0) * slot_gap;
    let bar_x0 = -total_w * 0.5;   // centered horizontally
    let bar_y = -0.88;              // near bottom
    let slot_h = slot_size * aspect; // corrected for aspect ratio

    // ─── Background bar ──────────────────────────────────────
    let bg_pad = 0.01;
    let bg_color = [0.10, 0.10, 0.09, 0.55];
    ui::push_quad(
        &mut verts,
        bar_x0 - bg_pad,
        bar_y - bg_pad,
        bar_x0 + total_w + bg_pad,
        bar_y + slot_h + bg_pad,
        bg_color,
    );

    // ─── Slots ───────────────────────────────────────────────
    let slot_bg = [0.16, 0.16, 0.14, 0.50];
    let sel_border = [0.72, 0.70, 0.62, 0.80];
    let border_w = 0.003;

    for i in 0..HOTBAR_SIZE {
        let x0 = bar_x0 + i as f32 * (slot_size + slot_gap);
        let x1 = x0 + slot_size;
        let y0 = bar_y;
        let y1 = bar_y + slot_h;

        // Selection highlight border
        if i == selected {
            ui::push_quad(&mut verts, x0 - border_w, y0 - border_w, x1 + border_w, y1 + border_w, sel_border);
        }

        // Slot background
        ui::push_quad(&mut verts, x0, y0, x1, y1, slot_bg);

        // Block color swatch (inner square with padding)
        let pad = 0.008;
        let block_color = BLOCK_COLORS[HOTBAR_BLOCKS[i] as usize];
        let brightness = if i == selected { 1.0 } else { 0.7 };
        let bc = [
            block_color[0] * brightness,
            block_color[1] * brightness,
            block_color[2] * brightness,
            0.90,
        ];
        ui::push_quad(&mut verts, x0 + pad, y0 + pad, x1 - pad, y1 - pad, bc);

        // Slot number (1-9) above the slot
        let num_text = format!("{}", i + 1);
        let text_scale = 0.55;
        let num_w = ui::text_width(&num_text, text_scale, ax);
        let num_x = x0 + (slot_size - num_w) * 0.5;
        let dim = if i == selected {
            [0.85, 0.82, 0.76, 0.85]
        } else {
            [0.50, 0.48, 0.44, 0.50]
        };
        ui::render_text(&mut verts, &num_text, num_x, y1 + 0.005, text_scale, ax, dim);
    }

    // ─── Selected block name ─────────────────────────────────
    let name = BLOCK_NAMES[selected];
    let name_scale = 0.7;
    let name_w = ui::text_width(name, name_scale, ax);
    let name_color = [0.70, 0.68, 0.62, 0.65];
    ui::render_text(&mut verts, name, -name_w * 0.5, bar_y - 0.035, name_scale, ax, name_color);

    verts
}
