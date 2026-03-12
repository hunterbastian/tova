// ═══════════════════════════════════════════════════════════════
//  Command palette — in-game text command input
// ═══════════════════════════════════════════════════════════════

use crate::ui::{self, UiVertex};

/// Result of executing a command.
pub enum CommandResult {
    ToggleGodMode,
    ToggleRain,
    ToggleSnow,
    Teleport(f32, f32, f32),
    SetSpeed(f32),
    SetTime(f32),
    PrintPos,
    Unknown(String),
}

/// Parse a command string.
pub fn parse_command(input: &str) -> CommandResult {
    let input = input.trim();
    let input = input.strip_prefix('/').unwrap_or(input);
    let parts: Vec<&str> = input.split_whitespace().collect();

    if parts.is_empty() {
        return CommandResult::Unknown(String::new());
    }

    match parts[0].to_lowercase().as_str() {
        "god" | "fly" => CommandResult::ToggleGodMode,
        "rain" => CommandResult::ToggleRain,
        "snow" => CommandResult::ToggleSnow,
        "pos" | "position" => CommandResult::PrintPos,
        "tp" | "teleport" => {
            if parts.len() >= 4 {
                if let (Ok(x), Ok(y), Ok(z)) = (
                    parts[1].parse::<f32>(),
                    parts[2].parse::<f32>(),
                    parts[3].parse::<f32>(),
                ) {
                    return CommandResult::Teleport(x, y, z);
                }
            }
            CommandResult::Unknown("Usage: /tp <x> <y> <z>".into())
        }
        "speed" => {
            if parts.len() >= 2 {
                if let Ok(s) = parts[1].parse::<f32>() {
                    return CommandResult::SetSpeed(s);
                }
            }
            CommandResult::Unknown("Usage: /speed <value>".into())
        }
        "time" => {
            if parts.len() >= 2 {
                if let Ok(t) = parts[1].parse::<f32>() {
                    return CommandResult::SetTime(t.clamp(0.0, 24.0));
                }
            }
            CommandResult::Unknown("Usage: /time <0-24>".into())
        }
        other => CommandResult::Unknown(format!("Unknown command: {}", other)),
    }
}

/// Build the command palette overlay geometry.
pub fn build_command_bar(text: &str, time: f32, aspect: f32) -> Vec<UiVertex> {
    let mut verts = Vec::new();
    let ax = 1.0 / aspect;

    // Dark bar across the bottom of screen
    let bar_h = 0.07;
    let bar_y0 = -1.0;
    let bar_y1 = bar_y0 + bar_h;
    ui::push_quad(&mut verts, -1.0, bar_y0, 1.0, bar_y1, [0.06, 0.06, 0.05, 0.85]);

    // Text
    let char_h = ui::GLYPH_H as f32 * ui::PIXEL_SIZE;
    let start_x = -0.95;
    let start_y = bar_y0 + (bar_h - char_h) * 0.5;
    ui::render_text(&mut verts, text, start_x, start_y, 1.0, ax, [0.82, 0.80, 0.74, 0.9]);

    // Blinking cursor
    if ((time * 2.5) as i32) % 2 == 0 {
        let cursor_x = start_x + ui::text_width(text, 1.0, ax);
        ui::push_quad(
            &mut verts,
            cursor_x, start_y,
            cursor_x + ui::PIXEL_SIZE * 0.8 * ax, start_y + char_h,
            [0.75, 0.72, 0.65, 0.8],
        );
    }

    verts
}
