// ═══════════════════════════════════════════════════════════════
//  Game time — drives sun position, sky color, lighting
//  24-hour cycle with configurable speed
// ═══════════════════════════════════════════════════════════════

use glam::Vec3;

/// Default starting time: late morning
const DEFAULT_TIME: f32 = 10.0;
/// Real seconds per in-game hour (120 = 1 full day in 48 minutes)
const DEFAULT_SECONDS_PER_HOUR: f32 = 120.0;

pub struct GameTime {
    /// Current time of day (0.0–24.0). 0=midnight, 6=sunrise, 12=noon, 18=sunset
    pub time: f32,
    /// How many real seconds pass per in-game hour
    pub seconds_per_hour: f32,
    /// Total in-game days elapsed
    pub day: u32,
    /// Whether time is advancing
    pub paused: bool,
}

impl GameTime {
    pub fn new() -> Self {
        Self {
            time: DEFAULT_TIME,
            seconds_per_hour: DEFAULT_SECONDS_PER_HOUR,
            day: 0,
            paused: false,
        }
    }

    /// Advance time by `dt` real seconds.
    pub fn update(&mut self, dt: f32) {
        if self.paused {
            return;
        }
        let hours = dt / self.seconds_per_hour;
        self.time += hours;
        if self.time >= 24.0 {
            self.time -= 24.0;
            self.day += 1;
        }
    }

    /// Set time directly (0.0–24.0).
    pub fn set_time(&mut self, t: f32) {
        self.time = t.rem_euclid(24.0);
    }

    /// Sun direction (normalized). Sun rises in east (+X), sets in west (-X),
    /// passes through south (-Z) at noon. Y is altitude.
    pub fn sun_direction(&self) -> Vec3 {
        // Convert time to sun angle: 6:00 = sunrise (0°), 12:00 = noon (90°), 18:00 = sunset (180°)
        let sun_angle = (self.time - 6.0) / 12.0 * std::f32::consts::PI;

        // Sun altitude: sin curve from sunrise to sunset
        let altitude = sun_angle.sin();
        // Sun horizontal position: moves east→south→west
        let horizontal = sun_angle.cos();

        // At night, sun goes below horizon (negative Y)
        Vec3::new(-horizontal, altitude.max(-0.15), -0.3)
            .normalize()
    }

    /// Sun color — cold filtered light, muted at all times.
    pub fn sun_color(&self) -> [f32; 3] {
        let altitude = self.sun_altitude();

        if altitude > 0.3 {
            // Daytime: dim, cool filtered light through heavy overcast
            [0.52, 0.50, 0.55]
        } else if altitude > 0.0 {
            // Dusk/dawn: bruised purple-amber, not warm gold
            let t = altitude / 0.3;
            let r = 0.55 + (1.0 - t) * 0.10;
            let g = 0.38 + t * 0.12;
            let b = 0.40 + t * 0.15;
            [r, g, b]
        } else {
            // Night: deep cold blue moonlight
            let t = (altitude / -0.15).min(1.0);
            let r = 0.55 - t * 0.40;
            let g = 0.38 - t * 0.22;
            let b = 0.40 - t * 0.08;
            [r, g, b]
        }
    }

    /// Ambient light level — always dim, even during "day".
    pub fn ambient_level(&self) -> f32 {
        let alt = self.sun_altitude();
        if alt > 0.1 {
            0.28  // darker day — perpetual gloom
        } else if alt > -0.05 {
            let t = (alt + 0.05) / 0.15;
            0.10 + t * 0.18
        } else {
            // Night: very dark
            0.10
        }
    }

    /// Sky zenith — dark grey-purple, oppressive ceiling.
    pub fn sky_zenith(&self) -> [f32; 3] {
        let alt = self.sun_altitude();
        if alt > 0.2 {
            // Day: heavy dark overcast with purple undertone
            [0.28, 0.26, 0.32]
        } else if alt > 0.0 {
            // Dusk/dawn: bruised purple
            let t = alt / 0.2;
            [0.22 + t * 0.06, 0.18 + t * 0.08, 0.26 + t * 0.06]
        } else {
            // Night: near-black with deep blue
            let t = (alt / -0.15).min(1.0);
            [0.22 - t * 0.14, 0.18 - t * 0.12, 0.26 - t * 0.10]
        }
    }

    /// Sky horizon — cold muted, dark at all times.
    pub fn sky_horizon(&self) -> [f32; 3] {
        let alt = self.sun_altitude();
        if alt > 0.2 {
            [0.34, 0.32, 0.38]
        } else if alt > 0.0 {
            let t = alt / 0.2;
            [0.30 + (1.0 - t) * 0.08, 0.24 + t * 0.08, 0.28 + t * 0.10]
        } else {
            let t = (alt / -0.15).min(1.0);
            [0.38 - t * 0.25, 0.24 - t * 0.16, 0.28 - t * 0.12]
        }
    }

    /// Sky horizon-sun — dim pale glow, not warm gold.
    pub fn sky_horizon_sun(&self) -> [f32; 3] {
        let alt = self.sun_altitude();
        if alt > 0.2 {
            [0.40, 0.38, 0.42]
        } else if alt > 0.0 {
            // Dusk: muted bruised amber-purple, not orange
            let t = alt / 0.2;
            [0.48 + (1.0 - t) * 0.10, 0.32 + t * 0.06, 0.30 + t * 0.12]
        } else {
            let t = (alt / -0.15).min(1.0);
            [0.48 - t * 0.32, 0.32 - t * 0.20, 0.30 - t * 0.12]
        }
    }

    /// Sky nadir — very dark ground hemisphere.
    pub fn sky_nadir(&self) -> [f32; 3] {
        let alt = self.sun_altitude();
        if alt > 0.2 {
            [0.22, 0.20, 0.25]
        } else if alt > 0.0 {
            let t = alt / 0.2;
            [0.18 + t * 0.04, 0.15 + t * 0.05, 0.20 + t * 0.05]
        } else {
            let t = (alt / -0.15).min(1.0);
            [0.18 - t * 0.12, 0.15 - t * 0.10, 0.20 - t * 0.10]
        }
    }

    /// Fog density multiplier — always thick, oppressive.
    pub fn fog_multiplier(&self) -> f32 {
        let alt = self.sun_altitude();
        if alt > 0.2 {
            1.3  // even daytime is hazy
        } else if alt > 0.0 {
            1.3 + (1.0 - alt / 0.2) * 0.5
        } else {
            // Night: very thick — world closes in
            1.8
        }
    }

    /// Normalized sun altitude (sin of elevation angle). Positive = above horizon.
    fn sun_altitude(&self) -> f32 {
        let sun_angle = (self.time - 6.0) / 12.0 * std::f32::consts::PI;
        sun_angle.sin()
    }

    /// Format time as HH:MM string.
    pub fn format_time(&self) -> String {
        let h = self.time.floor() as u32 % 24;
        let m = ((self.time.fract()) * 60.0).floor() as u32;
        format!("{:02}:{:02}", h, m)
    }

    /// Human-readable period name.
    pub fn period_name(&self) -> &'static str {
        match self.time as u32 {
            5..=6 => "DAWN",
            7..=11 => "MORNING",
            12 => "NOON",
            13..=16 => "AFTERNOON",
            17..=18 => "DUSK",
            19..=20 => "EVENING",
            _ => "NIGHT",
        }
    }
}
