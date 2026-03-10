/// Procedural footstep audio.
///
/// WASM: Web Audio API with generated noise buffers.
/// Native: silent stub (audio not yet wired for desktop).

pub struct AudioSystem {
    #[cfg(target_arch = "wasm32")]
    inner: WasmAudio,
}

impl AudioSystem {
    pub fn new() -> Self {
        Self {
            #[cfg(target_arch = "wasm32")]
            inner: WasmAudio::new(),
        }
    }

    #[allow(unused_variables)]
    pub fn play_footstep(&mut self, variation: u32) {
        #[cfg(target_arch = "wasm32")]
        self.inner.play_footstep(variation);
    }
}

// ─── WASM implementation ───────────────────────────────────

#[cfg(target_arch = "wasm32")]
struct WasmAudio {
    ctx: Option<web_sys::AudioContext>,
    buffers: Vec<web_sys::AudioBuffer>,
}

#[cfg(target_arch = "wasm32")]
impl WasmAudio {
    fn new() -> Self {
        Self {
            ctx: None,
            buffers: Vec::new(),
        }
    }

    fn ensure_context(&mut self) {
        if self.ctx.is_some() {
            return;
        }

        let ctx = match web_sys::AudioContext::new() {
            Ok(c) => c,
            Err(_) => return,
        };

        // Resume if suspended (browsers require user gesture)
        if ctx.state() == web_sys::AudioContextState::Suspended {
            let _ = ctx.resume();
        }

        let sample_rate = ctx.sample_rate();

        // Generate 4 footstep variations
        for seed in 0..4u32 {
            if let Ok(buf) = Self::generate_footstep(&ctx, sample_rate, seed) {
                self.buffers.push(buf);
            }
        }

        self.ctx = Some(ctx);
    }

    fn generate_footstep(
        ctx: &web_sys::AudioContext,
        sample_rate: f32,
        seed: u32,
    ) -> Result<web_sys::AudioBuffer, wasm_bindgen::JsValue> {
        let duration = 0.09 + (seed as f32 * 0.01); // 90-120ms
        let num_samples = (sample_rate * duration) as u32;

        let buffer = ctx.create_buffer(1, num_samples, sample_rate)?;

        let mut samples = vec![0.0f32; num_samples as usize];
        let mut rng: u32 = 0xDEAD_BEEF ^ (seed.wrapping_mul(2654435761));
        let mut prev = 0.0f32;
        let cutoff = 0.12 + (seed as f32 * 0.02); // vary filter

        for (i, sample) in samples.iter_mut().enumerate() {
            // LCG noise
            rng = rng.wrapping_mul(1664525).wrapping_add(1013904223);
            let noise = (rng as f32 / u32::MAX as f32) * 2.0 - 1.0;

            // Decay envelope — quick attack, fast falloff
            let t = i as f32 / num_samples as f32;
            let envelope = (1.0 - t).powi(4) * 0.25;

            // One-pole low-pass for muffled thud
            let filtered = prev + cutoff * (noise - prev);
            prev = filtered;

            *sample = filtered * envelope;
        }

        buffer.copy_to_channel(&samples, 0)?;
        Ok(buffer)
    }

    fn play_footstep(&mut self, variation: u32) {
        self.ensure_context();

        let ctx = match &self.ctx {
            Some(c) => c,
            None => return,
        };

        // Resume if suspended
        if ctx.state() == web_sys::AudioContextState::Suspended {
            let _ = ctx.resume();
        }

        if self.buffers.is_empty() {
            return;
        }

        let buf = &self.buffers[variation as usize % self.buffers.len()];

        let source = match ctx.create_buffer_source() {
            Ok(s) => s,
            Err(_) => return,
        };

        source.set_buffer(Some(buf));

        // Slight pitch variation for naturalness
        let pitch = 0.9 + ((variation % 7) as f32 * 0.04);
        source.playback_rate().set_value(pitch);

        let _ = source.connect_with_audio_node(&ctx.destination());
        let _ = source.start();
    }
}
