#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QualityPreset {
    Low,
    Medium,
    High,
    Ultra,
}

impl QualityPreset {
    pub fn next(self) -> Self {
        match self {
            Self::Low => Self::Medium,
            Self::Medium => Self::High,
            Self::High => Self::Ultra,
            Self::Ultra => Self::Low,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Low => "Low",
            Self::Medium => "Medium",
            Self::High => "High",
            Self::Ultra => "Ultra",
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct RenderSettings {
    pub preset: QualityPreset,
    pub vsync: bool,
    pub shadow_enabled: bool,
    pub bloom_enabled: bool,
    pub volumetric_enabled: bool,
    pub render_scale: f32,
    pub shadow_resolution: u32,
    pub shader_pack_enabled: bool,
    pub day_cycle_enabled: bool,
    pub pcf_radius: f32,
    pub bloom_quarter_enabled: bool,
}

impl RenderSettings {
    pub fn from_preset(preset: QualityPreset) -> Self {
        match preset {
            QualityPreset::Low => Self {
                preset,
                vsync: true,
                shadow_enabled: true,
                bloom_enabled: false,
                volumetric_enabled: false,
                render_scale: 0.85,
                shadow_resolution: 512,
                shader_pack_enabled: false,
                day_cycle_enabled: false,
                pcf_radius: 0.0,
                bloom_quarter_enabled: false,
            },
            QualityPreset::Medium => Self {
                preset,
                vsync: true,
                shadow_enabled: true,
                bloom_enabled: true,
                volumetric_enabled: false,
                render_scale: 1.0,
                shadow_resolution: 1024,
                shader_pack_enabled: true,
                day_cycle_enabled: true,
                pcf_radius: 1.0,
                bloom_quarter_enabled: false,
            },
            QualityPreset::High => Self {
                preset,
                vsync: true,
                shadow_enabled: true,
                bloom_enabled: true,
                volumetric_enabled: true,
                render_scale: 1.0,
                shadow_resolution: 2048,
                shader_pack_enabled: true,
                day_cycle_enabled: true,
                pcf_radius: 1.5,
                bloom_quarter_enabled: true,
            },
            QualityPreset::Ultra => Self {
                preset,
                vsync: true,
                shadow_enabled: true,
                bloom_enabled: true,
                volumetric_enabled: true,
                render_scale: 1.0,
                shadow_resolution: 4096,
                shader_pack_enabled: true,
                day_cycle_enabled: true,
                pcf_radius: 2.0,
                bloom_quarter_enabled: true,
            },
        }
    }

    pub fn shadow_bias(self) -> f32 {
        match self.preset {
            QualityPreset::Low => 0.0045,
            QualityPreset::Medium => 0.003,
            QualityPreset::High => 0.0022,
            QualityPreset::Ultra => 0.0018,
        }
    }

    pub fn bloom_threshold(self) -> f32 {
        match self.preset {
            QualityPreset::Low => 1.1,
            QualityPreset::Medium => 1.0,
            QualityPreset::High => 0.9,
            QualityPreset::Ultra => 0.85,
        }
    }

    pub fn bloom_intensity(self) -> f32 {
        match self.preset {
            QualityPreset::Low => 0.0,
            QualityPreset::Medium => 0.08,
            QualityPreset::High => 0.13,
            QualityPreset::Ultra => 0.16,
        }
    }

    pub fn volumetric_strength(self) -> f32 {
        if !self.volumetric_enabled {
            return 0.0;
        }
        match self.preset {
            QualityPreset::Low => 0.0,
            QualityPreset::Medium => 0.0,
            QualityPreset::High => 0.18,
            QualityPreset::Ultra => 0.28,
        }
    }
}

impl Default for RenderSettings {
    fn default() -> Self {
        Self::from_preset(QualityPreset::High)
    }
}
