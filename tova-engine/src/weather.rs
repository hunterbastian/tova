#[derive(Clone, Copy, PartialEq)]
pub enum WeatherType {
    Clear,
    Rain,
    Snow,
}

#[derive(Clone, Copy, PartialEq)]
pub enum WindMode {
    Calm,      // gentle ambient sway, barely noticeable
    Breeze,    // light directional wind, leaves rustle
    Gusty,     // irregular bursts with lulls between
    Steady,    // constant strong wind, everything leans one way
    Storm,     // violent, shifting direction, maximum intensity
}

pub struct Weather {
    pub current: WeatherType,
    pub intensity: f32,
    target_intensity: f32,
    pub time: f32,
    // Wind system
    pub wind_mode: WindMode,
    pub wind_angle: f32,         // current direction (radians)
    pub wind_strength: f32,      // current strength 0..1
    pub wind_gust_intensity: f32, // how gusty (0 = smooth, 1 = extreme bursts)
    pub wind_turbulence: f32,     // high-frequency variation 0..1
    target_wind_angle: f32,       // direction we're rotating toward
    angle_velocity: f32,          // how fast direction is changing
    gust_phase: f32,              // phase for gust oscillation
    gust_timer: f32,              // countdown to next gust burst
    gust_burst: f32,              // current burst strength (decays)
    lull_timer: f32,              // countdown to next lull
    in_lull: bool,                // currently in a calm lull
}

const TRANSITION_SPEED: f32 = 0.3;

impl Weather {
    pub fn new() -> Self {
        Self {
            current: WeatherType::Clear,
            intensity: 0.0,
            target_intensity: 0.0,
            time: 0.0,
            wind_mode: WindMode::Breeze,
            wind_angle: 0.8,
            wind_strength: 0.0,
            wind_gust_intensity: 0.0,
            wind_turbulence: 0.0,
            target_wind_angle: 0.8,
            angle_velocity: 0.0,
            gust_phase: 0.0,
            gust_timer: 3.0,
            gust_burst: 0.0,
            lull_timer: 8.0,
            in_lull: false,
        }
    }

    /// Toggle a weather type on/off. If already active, fades to clear.
    pub fn toggle(&mut self, weather: WeatherType) {
        if weather == self.current && self.target_intensity > 0.0 {
            self.target_intensity = 0.0;
        } else {
            self.current = weather;
            self.target_intensity = 1.0;
        }
    }

    /// Set wind mode directly (e.g., from /wind command).
    pub fn set_wind_mode(&mut self, mode: WindMode) {
        self.wind_mode = mode;
    }

    pub fn update(&mut self, dt: f32) {
        self.time += dt;

        // Weather intensity transition
        let diff = self.target_intensity - self.intensity;
        self.intensity += diff * (TRANSITION_SPEED * dt * 60.0).min(1.0);
        if (self.intensity - self.target_intensity).abs() < 0.001 {
            self.intensity = self.target_intensity;
        }

        // Auto-select wind mode based on weather
        self.wind_mode = match self.current {
            WeatherType::Clear => {
                // Cycle between calm and breeze naturally
                if (self.time * 0.02).sin() > 0.3 { WindMode::Breeze } else { WindMode::Calm }
            }
            WeatherType::Rain => {
                if self.intensity > 0.7 { WindMode::Storm }
                else if self.intensity > 0.3 { WindMode::Gusty }
                else { WindMode::Breeze }
            }
            WeatherType::Snow => {
                if self.intensity > 0.7 { WindMode::Steady }
                else { WindMode::Breeze }
            }
        };

        self.update_wind(dt);
    }

    fn update_wind(&mut self, dt: f32) {
        self.gust_phase += dt;

        // ─── Mode-specific parameters ────────────────────────
        let (base_strength, gust_amount, turb, dir_change_rate, dir_change_range) = match self.wind_mode {
            WindMode::Calm => (0.08, 0.05, 0.02, 0.01, 0.2),
            WindMode::Breeze => (0.25, 0.15, 0.08, 0.04, 0.5),
            WindMode::Gusty => (0.20, 0.55, 0.20, 0.08, 1.0),
            WindMode::Steady => (0.60, 0.10, 0.05, 0.02, 0.3),
            WindMode::Storm => (0.55, 0.45, 0.35, 0.15, 2.0),
        };

        // ─── Direction ───────────────────────────────────────
        // Periodically pick a new target direction
        self.lull_timer -= dt;
        if self.lull_timer <= 0.0 {
            // New target direction
            self.target_wind_angle += pseudo_random(self.gust_phase) * dir_change_range;
            self.lull_timer = 4.0 + pseudo_random(self.gust_phase * 1.7) * 6.0;

            // Gusty mode: toggle lull periods
            if self.wind_mode == WindMode::Gusty {
                self.in_lull = !self.in_lull;
                if self.in_lull {
                    self.lull_timer = 1.5 + pseudo_random(self.gust_phase * 2.1) * 2.0;
                }
            } else {
                self.in_lull = false;
            }
        }

        // Smooth direction interpolation with momentum
        let angle_diff = angle_wrap(self.target_wind_angle - self.wind_angle);
        self.angle_velocity += angle_diff * dir_change_rate * 2.0 * dt;
        self.angle_velocity *= 1.0 - dt * 2.0; // damping
        self.wind_angle += self.angle_velocity;

        // ─── Gust bursts ─────────────────────────────────────
        self.gust_timer -= dt;
        if self.gust_timer <= 0.0 {
            // Trigger a burst
            self.gust_burst = 0.5 + pseudo_random(self.gust_phase * 3.3) * 0.5;
            self.gust_timer = match self.wind_mode {
                WindMode::Calm => 6.0 + pseudo_random(self.gust_phase * 0.7) * 10.0,
                WindMode::Breeze => 3.0 + pseudo_random(self.gust_phase * 1.1) * 5.0,
                WindMode::Gusty => 0.8 + pseudo_random(self.gust_phase * 1.9) * 2.5,
                WindMode::Steady => 5.0 + pseudo_random(self.gust_phase * 0.5) * 8.0,
                WindMode::Storm => 0.3 + pseudo_random(self.gust_phase * 2.7) * 1.5,
            };
        }
        // Burst decays
        self.gust_burst *= (-3.0_f32 * dt).exp();

        // ─── Compose final strength ──────────────────────────
        // Multi-frequency oscillation for organic feel
        let osc1 = (self.gust_phase * 0.7).sin() * 0.5 + 0.5;
        let osc2 = (self.gust_phase * 1.9).sin() * 0.3;
        let osc3 = (self.gust_phase * 4.3).sin() * 0.1;  // high frequency flutter
        let gust_wave = (osc1 + osc2 + osc3).max(0.0) * gust_amount;

        let lull_mult = if self.in_lull { 0.15 } else { 1.0 };
        let target_strength = (base_strength + gust_wave + self.gust_burst * gust_amount) * lull_mult;

        // Smooth interpolation to target (faster in storms)
        let lerp_speed = match self.wind_mode {
            WindMode::Storm => 6.0,
            WindMode::Gusty => 4.0,
            _ => 2.5,
        };
        self.wind_strength += (target_strength - self.wind_strength) * (lerp_speed * dt).min(1.0);
        self.wind_strength = self.wind_strength.clamp(0.0, 1.0);

        // Turbulence — high-frequency jitter
        self.wind_turbulence = turb * (1.0 + self.gust_burst * 0.5);

        // Gust intensity for shader use
        self.wind_gust_intensity = gust_amount + self.gust_burst * 0.3;
    }

    /// Wind direction as (x, z) unit vector.
    pub fn wind_dir(&self) -> (f32, f32) {
        (self.wind_angle.cos(), self.wind_angle.sin())
    }

    /// Wind mode name for HUD display.
    pub fn wind_mode_name(&self) -> &'static str {
        match self.wind_mode {
            WindMode::Calm => "CALM",
            WindMode::Breeze => "BREEZE",
            WindMode::Gusty => "GUSTY",
            WindMode::Steady => "STEADY",
            WindMode::Storm => "STORM",
        }
    }

    pub fn fog_multiplier(&self) -> f32 {
        match self.current {
            WeatherType::Clear => 1.0,
            WeatherType::Rain => 1.0 + self.intensity * 1.5,
            WeatherType::Snow => 1.0 + self.intensity * 0.8,
        }
    }

    pub fn sky_darken(&self) -> f32 {
        match self.current {
            WeatherType::Clear => 0.0,
            WeatherType::Rain => self.intensity * 0.25,
            WeatherType::Snow => self.intensity * 0.05,
        }
    }

    pub fn type_f32(&self) -> f32 {
        match self.current {
            WeatherType::Clear => 0.0,
            WeatherType::Rain => 1.0,
            WeatherType::Snow => 2.0,
        }
    }
}

/// Wrap angle difference to [-PI, PI].
fn angle_wrap(mut a: f32) -> f32 {
    while a > std::f32::consts::PI { a -= std::f32::consts::TAU; }
    while a < -std::f32::consts::PI { a += std::f32::consts::TAU; }
    a
}

/// Deterministic pseudo-random from a float seed, returns -1..1.
fn pseudo_random(x: f32) -> f32 {
    let n = (x * 127.1 + 311.7).sin() * 43758.5453;
    n.fract() * 2.0 - 1.0
}
