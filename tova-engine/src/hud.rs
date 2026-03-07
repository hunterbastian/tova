use crate::geometry::Vertex;
use crate::voxel::block::{Block, BLOCK_COLORS};

const WHITE: [f32; 3] = [0.88, 0.86, 0.82];
const MUTED: [f32; 3] = [0.56, 0.57, 0.54];
const ACCENT: [f32; 3] = [0.74, 0.66, 0.47];
const PANEL: [f32; 3] = [0.05, 0.06, 0.07];
const PANEL_EDGE: [f32; 3] = [0.15, 0.15, 0.14];
const HEALTH: [f32; 3] = [0.45, 0.17, 0.16];
const MAGICKA: [f32; 3] = [0.20, 0.28, 0.36];
const FATIGUE: [f32; 3] = [0.47, 0.43, 0.24];

pub struct HudView<'a> {
    pub hotbar: &'a [Block],
    pub selected_slot: usize,
    pub status_message: Option<&'a str>,
    pub cursor_captured: bool,
    pub health: f32,
    pub magicka: f32,
    pub fatigue: f32,
}

pub fn build_mesh(view: HudView<'_>) -> (Vec<Vertex>, Vec<u32>) {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();

    build_vitals(&mut vertices, &mut indices, &view);
    build_hotbar(&mut vertices, &mut indices, &view);
    build_crosshair(&mut vertices, &mut indices, view.cursor_captured);
    build_status_banner(&mut vertices, &mut indices, &view);

    (vertices, indices)
}

fn build_vitals(vertices: &mut Vec<Vertex>, indices: &mut Vec<u32>, view: &HudView<'_>) {
    let left = -0.95;
    let top = -0.78;
    let width = 0.21;
    let height = 0.022;
    let gap = 0.067;

    build_vital_bar(
        vertices,
        indices,
        "HEALTH",
        left,
        top,
        width,
        height,
        view.health,
        HEALTH,
    );
    build_vital_bar(
        vertices,
        indices,
        "MAGICKA",
        left,
        top - gap,
        width,
        height,
        view.magicka,
        MAGICKA,
    );
    build_vital_bar(
        vertices,
        indices,
        "FATIGUE",
        left,
        top - gap * 2.0,
        width,
        height,
        view.fatigue,
        FATIGUE,
    );
}

fn build_hotbar(vertices: &mut Vec<Vertex>, indices: &mut Vec<u32>, view: &HudView<'_>) {
    let slot_width = 0.10;
    let slot_height = 0.085;
    let gap = 0.017;
    let total_width = slot_width * view.hotbar.len() as f32 + gap * (view.hotbar.len() as f32 - 1.0);
    let start_x = -total_width * 0.5;
    let bottom = -0.955;
    let top = bottom + slot_height;

    add_rect(
        vertices,
        indices,
        start_x - 0.035,
        start_x + total_width + 0.035,
        bottom - 0.02,
        top + 0.028,
        PANEL,
        0.42,
    );

    for (slot, block) in view.hotbar.iter().copied().enumerate() {
        let left = start_x + slot as f32 * (slot_width + gap);
        let right = left + slot_width;
        let selected = slot == view.selected_slot;

        add_rect(
            vertices,
            indices,
            left,
            right,
            bottom,
            top,
            if selected { ACCENT } else { PANEL_EDGE },
            if selected { 0.94 } else { 0.76 },
        );
        add_rect(
            vertices,
            indices,
            left + 0.008,
            right - 0.008,
            bottom + 0.008,
            top - 0.008,
            PANEL,
            0.88,
        );
        add_rect(
            vertices,
            indices,
            left + 0.018,
            right - 0.018,
            bottom + 0.022,
            top - 0.018,
            BLOCK_COLORS[block as usize],
            0.98,
        );
        draw_text(vertices, indices, &(slot + 1).to_string(), left + 0.016, top - 0.016, 0.009, MUTED, 0.92);
    }

    let block_name = view.hotbar[view.selected_slot]
        .display_name()
        .to_ascii_uppercase();
    let text_width = text_width(&block_name, 0.0105);
    draw_text(
        vertices,
        indices,
        &block_name,
        -text_width * 0.5,
        -0.81,
        0.0105,
        MUTED,
        0.94,
    );
}

fn build_crosshair(vertices: &mut Vec<Vertex>, indices: &mut Vec<u32>, cursor_captured: bool) {
    if !cursor_captured {
        return;
    }

    add_rect(vertices, indices, -0.018, 0.018, -0.0022, 0.0022, WHITE, 0.88);
    add_rect(vertices, indices, -0.0022, 0.0022, -0.018, 0.018, WHITE, 0.88);
}

fn build_status_banner(
    vertices: &mut Vec<Vertex>,
    indices: &mut Vec<u32>,
    view: &HudView<'_>,
) {
    let message = view
        .status_message
        .or_else(|| (!view.cursor_captured).then_some("CLICK OR ENTER TO WALK"));
    let Some(message) = message else {
        return;
    };

    let width = text_width(message, 0.0105);
    let left = -width * 0.5 - 0.035;
    let right = width * 0.5 + 0.035;
    let text_color = if view.status_message.is_some() { ACCENT } else { WHITE };

    add_rect(vertices, indices, left, right, 0.88, 0.97, PANEL, 0.62);
    draw_text(
        vertices,
        indices,
        message,
        -width * 0.5,
        0.935,
        0.0105,
        text_color,
        0.96,
    );
}

#[allow(clippy::too_many_arguments)]
fn build_vital_bar(
    vertices: &mut Vec<Vertex>,
    indices: &mut Vec<u32>,
    label: &str,
    left: f32,
    top: f32,
    width: f32,
    height: f32,
    value: f32,
    color: [f32; 3],
) {
    draw_text(vertices, indices, label, left, top, 0.0085, MUTED, 0.92);
    let bottom = top - 0.048;
    let right = left + width;
    add_rect(vertices, indices, left, right, bottom, bottom + height, PANEL, 0.88);
    add_rect(
        vertices,
        indices,
        left + 0.0035,
        left + 0.0035 + (width - 0.007) * value.clamp(0.0, 1.0),
        bottom + 0.0035,
        bottom + height - 0.0035,
        color,
        0.96,
    );
}

#[allow(clippy::too_many_arguments)]
fn add_rect(
    vertices: &mut Vec<Vertex>,
    indices: &mut Vec<u32>,
    left: f32,
    right: f32,
    bottom: f32,
    top: f32,
    color: [f32; 3],
    alpha: f32,
) {
    let base = vertices.len() as u32;
    let meta = [alpha, 0.0, 0.0];
    vertices.push(Vertex {
        position: [left, bottom, 0.0],
        color,
        normal: meta,
    });
    vertices.push(Vertex {
        position: [right, bottom, 0.0],
        color,
        normal: meta,
    });
    vertices.push(Vertex {
        position: [right, top, 0.0],
        color,
        normal: meta,
    });
    vertices.push(Vertex {
        position: [left, top, 0.0],
        color,
        normal: meta,
    });
    indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
}

#[allow(clippy::too_many_arguments)]
fn draw_text(
    vertices: &mut Vec<Vertex>,
    indices: &mut Vec<u32>,
    text: &str,
    x: f32,
    y: f32,
    pixel: f32,
    color: [f32; 3],
    alpha: f32,
) {
    let mut cursor_x = x;
    for ch in text.chars() {
        let uppercase = ch.to_ascii_uppercase();
        if uppercase == ' ' {
            cursor_x += pixel * 6.0;
            continue;
        }

        for (row, bits) in glyph_rows(uppercase).iter().enumerate() {
            for col in 0..5 {
                if bits & (1 << (4 - col)) == 0 {
                    continue;
                }
                let left = cursor_x + col as f32 * pixel;
                let right = left + pixel;
                let top = y - row as f32 * pixel;
                let bottom = top - pixel;
                add_rect(vertices, indices, left, right, bottom, top, color, alpha);
            }
        }

        cursor_x += pixel * 6.0;
    }
}

fn text_width(text: &str, pixel: f32) -> f32 {
    text.chars().count() as f32 * pixel * 6.0
}

fn glyph_rows(ch: char) -> [u8; 7] {
    match ch {
        'A' => [
            0b01110, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001,
        ],
        'B' => [
            0b11110, 0b10001, 0b10001, 0b11110, 0b10001, 0b10001, 0b11110,
        ],
        'C' => [
            0b01110, 0b10001, 0b10000, 0b10000, 0b10000, 0b10001, 0b01110,
        ],
        'D' => [
            0b11110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b11110,
        ],
        'E' => [
            0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b11111,
        ],
        'F' => [
            0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b10000,
        ],
        'G' => [
            0b01110, 0b10001, 0b10000, 0b10111, 0b10001, 0b10001, 0b01110,
        ],
        'H' => [
            0b10001, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001,
        ],
        'I' => [
            0b11111, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b11111,
        ],
        'K' => [
            0b10001, 0b10010, 0b10100, 0b11000, 0b10100, 0b10010, 0b10001,
        ],
        'L' => [
            0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b11111,
        ],
        'M' => [
            0b10001, 0b11011, 0b10101, 0b10101, 0b10001, 0b10001, 0b10001,
        ],
        'N' => [
            0b10001, 0b11001, 0b10101, 0b10011, 0b10001, 0b10001, 0b10001,
        ],
        'O' => [
            0b01110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110,
        ],
        'P' => [
            0b11110, 0b10001, 0b10001, 0b11110, 0b10000, 0b10000, 0b10000,
        ],
        'R' => [
            0b11110, 0b10001, 0b10001, 0b11110, 0b10100, 0b10010, 0b10001,
        ],
        'S' => [
            0b01111, 0b10000, 0b10000, 0b01110, 0b00001, 0b00001, 0b11110,
        ],
        'T' => [
            0b11111, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100,
        ],
        'U' => [
            0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110,
        ],
        'V' => [
            0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01010, 0b00100,
        ],
        'W' => [
            0b10001, 0b10001, 0b10001, 0b10101, 0b10101, 0b11011, 0b10001,
        ],
        'X' => [
            0b10001, 0b10001, 0b01010, 0b00100, 0b01010, 0b10001, 0b10001,
        ],
        'Y' => [
            0b10001, 0b10001, 0b01010, 0b00100, 0b00100, 0b00100, 0b00100,
        ],
        '0' => [
            0b01110, 0b10001, 0b10011, 0b10101, 0b11001, 0b10001, 0b01110,
        ],
        '1' => [
            0b00100, 0b01100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110,
        ],
        '2' => [
            0b01110, 0b10001, 0b00001, 0b00010, 0b00100, 0b01000, 0b11111,
        ],
        '3' => [
            0b11110, 0b00001, 0b00001, 0b00110, 0b00001, 0b00001, 0b11110,
        ],
        '4' => [
            0b00010, 0b00110, 0b01010, 0b10010, 0b11111, 0b00010, 0b00010,
        ],
        '5' => [
            0b11111, 0b10000, 0b10000, 0b11110, 0b00001, 0b00001, 0b11110,
        ],
        '6' => [
            0b00110, 0b01000, 0b10000, 0b11110, 0b10001, 0b10001, 0b01110,
        ],
        '7' => [
            0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b01000, 0b01000,
        ],
        '8' => [
            0b01110, 0b10001, 0b10001, 0b01110, 0b10001, 0b10001, 0b01110,
        ],
        '9' => [
            0b01110, 0b10001, 0b10001, 0b01111, 0b00001, 0b00010, 0b11100,
        ],
        _ => [0; 7],
    }
}

#[cfg(test)]
mod tests {
    use crate::voxel::block::Block;

    use super::{build_mesh, HudView};

    #[test]
    fn hud_builds_geometry() {
        let (_, indices) = build_mesh(HudView {
            hotbar: &[Block::Dirt, Block::Stone, Block::Grass, Block::Sand, Block::Cobble],
            selected_slot: 1,
            status_message: Some("READY"),
            cursor_captured: true,
            health: 1.0,
            magicka: 0.92,
            fatigue: 0.76,
        });
        assert!(!indices.is_empty());
    }
}
