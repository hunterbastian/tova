// ═══════════════════════════════════════════════════════════════
//  UI primitives — pixel font and quad helpers
//  Shared by command palette, HUD, and any future overlays
// ═══════════════════════════════════════════════════════════════

/// Vertex matching MenuVertex layout: position [f32; 2] + color [f32; 4].
#[repr(C)]
#[derive(Copy, Clone)]
pub struct UiVertex {
    pub position: [f32; 2],
    pub color: [f32; 4],
}

unsafe impl bytemuck::Pod for UiVertex {}
unsafe impl bytemuck::Zeroable for UiVertex {}

/// Push a quad (2 triangles) into a vertex buffer.
pub fn push_quad(verts: &mut Vec<UiVertex>, x0: f32, y0: f32, x1: f32, y1: f32, color: [f32; 4]) {
    verts.push(UiVertex { position: [x0, y0], color });
    verts.push(UiVertex { position: [x1, y0], color });
    verts.push(UiVertex { position: [x1, y1], color });
    verts.push(UiVertex { position: [x0, y0], color });
    verts.push(UiVertex { position: [x1, y1], color });
    verts.push(UiVertex { position: [x0, y1], color });
}

// ═══════════════════════════════════════════════════════════════
//  Pixel font — 5x7 bitmaps for A-Z, 0-9, symbols
//  Each glyph is 7 rows of 5-bit masks (MSB = leftmost pixel)
// ═══════════════════════════════════════════════════════════════

pub const GLYPH_W: usize = 5;
pub const GLYPH_H: usize = 7;
pub const PIXEL_SIZE: f32 = 0.0028;

/// Returns 7-row bitmap for a character, or None for unsupported chars.
pub fn glyph_bitmap(ch: char) -> Option<[u8; 7]> {
    match ch.to_ascii_uppercase() {
        'A' => Some([0b01110, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001]),
        'B' => Some([0b11110, 0b10001, 0b10001, 0b11110, 0b10001, 0b10001, 0b11110]),
        'C' => Some([0b01110, 0b10001, 0b10000, 0b10000, 0b10000, 0b10001, 0b01110]),
        'D' => Some([0b11110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b11110]),
        'E' => Some([0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b11111]),
        'F' => Some([0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b10000]),
        'G' => Some([0b01110, 0b10001, 0b10000, 0b10111, 0b10001, 0b10001, 0b01110]),
        'H' => Some([0b10001, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001]),
        'I' => Some([0b01110, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110]),
        'J' => Some([0b00111, 0b00010, 0b00010, 0b00010, 0b00010, 0b10010, 0b01100]),
        'K' => Some([0b10001, 0b10010, 0b10100, 0b11000, 0b10100, 0b10010, 0b10001]),
        'L' => Some([0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b11111]),
        'M' => Some([0b10001, 0b11011, 0b10101, 0b10101, 0b10001, 0b10001, 0b10001]),
        'N' => Some([0b10001, 0b11001, 0b10101, 0b10011, 0b10001, 0b10001, 0b10001]),
        'O' => Some([0b01110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110]),
        'P' => Some([0b11110, 0b10001, 0b10001, 0b11110, 0b10000, 0b10000, 0b10000]),
        'Q' => Some([0b01110, 0b10001, 0b10001, 0b10001, 0b10101, 0b01110, 0b00001]),
        'R' => Some([0b11110, 0b10001, 0b10001, 0b11110, 0b10100, 0b10010, 0b10001]),
        'S' => Some([0b01110, 0b10001, 0b10000, 0b01110, 0b00001, 0b10001, 0b01110]),
        'T' => Some([0b11111, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100]),
        'U' => Some([0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110]),
        'V' => Some([0b10001, 0b10001, 0b10001, 0b10001, 0b01010, 0b01010, 0b00100]),
        'W' => Some([0b10001, 0b10001, 0b10001, 0b10101, 0b10101, 0b11011, 0b10001]),
        'X' => Some([0b10001, 0b10001, 0b01010, 0b00100, 0b01010, 0b10001, 0b10001]),
        'Y' => Some([0b10001, 0b10001, 0b01010, 0b00100, 0b00100, 0b00100, 0b00100]),
        'Z' => Some([0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b10000, 0b11111]),
        '0' => Some([0b01110, 0b10001, 0b10011, 0b10101, 0b11001, 0b10001, 0b01110]),
        '1' => Some([0b00100, 0b01100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110]),
        '2' => Some([0b01110, 0b10001, 0b00001, 0b00110, 0b01000, 0b10000, 0b11111]),
        '3' => Some([0b01110, 0b10001, 0b00001, 0b00110, 0b00001, 0b10001, 0b01110]),
        '4' => Some([0b00010, 0b00110, 0b01010, 0b10010, 0b11111, 0b00010, 0b00010]),
        '5' => Some([0b11111, 0b10000, 0b11110, 0b00001, 0b00001, 0b10001, 0b01110]),
        '6' => Some([0b01110, 0b10000, 0b10000, 0b11110, 0b10001, 0b10001, 0b01110]),
        '7' => Some([0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b01000, 0b01000]),
        '8' => Some([0b01110, 0b10001, 0b10001, 0b01110, 0b10001, 0b10001, 0b01110]),
        '9' => Some([0b01110, 0b10001, 0b10001, 0b01111, 0b00001, 0b00001, 0b01110]),
        '/' => Some([0b00001, 0b00010, 0b00010, 0b00100, 0b01000, 0b01000, 0b10000]),
        ' ' => Some([0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b00000]),
        '.' => Some([0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b00100]),
        '-' => Some([0b00000, 0b00000, 0b00000, 0b11111, 0b00000, 0b00000, 0b00000]),
        ':' => Some([0b00000, 0b00100, 0b00000, 0b00000, 0b00000, 0b00100, 0b00000]),
        '(' => Some([0b00010, 0b00100, 0b01000, 0b01000, 0b01000, 0b00100, 0b00010]),
        ')' => Some([0b01000, 0b00100, 0b00010, 0b00010, 0b00010, 0b00100, 0b01000]),
        ',' => Some([0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b00100, 0b01000]),
        _ => None,
    }
}

/// Render a text string as pixel-font quads with a dark shadow behind it.
/// `x`, `y` — NDC position of the bottom-left of the first character.
/// `scale` — multiplier on PIXEL_SIZE (1.0 = default size).
/// `ax` — aspect correction (1.0 / aspect ratio).
/// `color` — RGBA color for the text.
pub fn render_text(
    verts: &mut Vec<UiVertex>,
    text: &str,
    x: f32,
    y: f32,
    scale: f32,
    ax: f32,
    color: [f32; 4],
) {
    let ps = PIXEL_SIZE * scale;
    let char_w = (GLYPH_W as f32 + 1.0) * ps;
    // Shadow offset (1 pixel down-right)
    let sx = ps * ax * 0.5;
    let sy = -ps * 0.5;
    let shadow_color = [0.0, 0.0, 0.0, color[3] * 0.4];

    for (ci, ch) in text.chars().enumerate() {
        if let Some(bitmap) = glyph_bitmap(ch) {
            let cx = x + ci as f32 * char_w * ax;
            for row in 0..GLYPH_H {
                let bits = bitmap[row];
                for col in 0..GLYPH_W {
                    if bits & (1 << (GLYPH_W - 1 - col)) != 0 {
                        let px = cx + col as f32 * ps * ax;
                        let py = y + (GLYPH_H - 1 - row) as f32 * ps;
                        // Shadow pixel
                        push_quad(verts, px + sx, py + sy, px + sx + ps * ax, py + sy + ps, shadow_color);
                        // Foreground pixel
                        push_quad(verts, px, py, px + ps * ax, py + ps, color);
                    }
                }
            }
        }
    }
}

/// Measure text width in NDC units.
pub fn text_width(text: &str, scale: f32, ax: f32) -> f32 {
    let ps = PIXEL_SIZE * scale;
    let char_w = (GLYPH_W as f32 + 1.0) * ps;
    text.len() as f32 * char_w * ax
}
