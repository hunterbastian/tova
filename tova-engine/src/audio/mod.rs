use rodio::{OutputStream, Sink, Source};
use std::time::Duration;

const SAMPLE_RATE: u32 = 44_100;

pub struct AmbientAudio {
    _stream: OutputStream,
    _sink: Sink,
}

impl AmbientAudio {
    pub fn start() -> Result<Self, String> {
        let (stream, handle) = OutputStream::try_default()
            .map_err(|error| format!("failed to open audio output stream: {error}"))?;

        let sink = Sink::try_new(&handle)
            .map_err(|error| format!("failed to create ambient sink: {error}"))?;

        // Subtle, low-passed noise that feels like distant wind.
        sink.set_volume(0.22);
        sink.append(WindSource::new());
        sink.play();

        Ok(Self {
            _stream: stream,
            _sink: sink,
        })
    }
}

struct WindSource {
    seed: u32,
    smoothed_noise: f32,
    frame_sample: f32,
    channel_index: u16,
    time_seconds: f32,
}

impl WindSource {
    fn new() -> Self {
        Self {
            seed: 0xA11C_E5E9,
            smoothed_noise: 0.0,
            frame_sample: 0.0,
            channel_index: 0,
            time_seconds: 0.0,
        }
    }

    fn next_unit_noise(&mut self) -> f32 {
        // Small deterministic pseudo-random generator for procedural ambient audio.
        self.seed = self.seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        let unit = ((self.seed >> 8) as f32) / 16_777_216.0;
        unit * 2.0 - 1.0
    }
}

impl Iterator for WindSource {
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        if self.channel_index == 0 {
            let noise = self.next_unit_noise();

            // Low-pass filtering turns harsh white noise into a soft whoosh.
            self.smoothed_noise += (noise - self.smoothed_noise) * 0.012;

            // Very slow amplitude movement to avoid static, repetitive ambience.
            let lfo_a = (self.time_seconds * 0.11).sin() * 0.5 + 0.5;
            let lfo_b = (self.time_seconds * 0.019).sin() * 0.5 + 0.5;
            let gain = 0.08 + lfo_a * 0.08 + lfo_b * 0.05;

            self.frame_sample = (self.smoothed_noise * gain).clamp(-0.25, 0.25);
            self.time_seconds += 1.0 / SAMPLE_RATE as f32;
        }

        let out = self.frame_sample;
        self.channel_index = (self.channel_index + 1) % 2;
        Some(out)
    }
}

impl Source for WindSource {
    fn current_frame_len(&self) -> Option<usize> {
        None
    }

    fn channels(&self) -> u16 {
        2
    }

    fn sample_rate(&self) -> u32 {
        SAMPLE_RATE
    }

    fn total_duration(&self) -> Option<Duration> {
        None
    }
}
