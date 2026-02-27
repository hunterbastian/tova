use rodio::{Decoder, OutputStream, Sink, Source};
use std::collections::BTreeSet;
use std::fs;
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::time::Duration;

const SAMPLE_RATE: u32 = 44_100;
const AMBIENT_VOLUME: f32 = 0.16;
const MUSIC_VOLUME: f32 = 0.42;
// Keep queue depth bounded to avoid excessive simultaneous open file handles.
const PLAYLIST_LOOP_COUNT: usize = 24;
const SUPPORTED_EXTENSIONS: &[&str] = &["ogg", "mp3", "wav", "flac"];

pub struct AmbientAudio {
    _stream: OutputStream,
    _ambient_sink: Sink,
    _music_sink: Option<Sink>,
}

impl AmbientAudio {
    pub fn start() -> Result<Self, String> {
        let (stream, handle) = OutputStream::try_default()
            .map_err(|error| format!("failed to open audio output stream: {error}"))?;

        let ambient_sink = Sink::try_new(&handle)
            .map_err(|error| format!("failed to create ambient sink: {error}"))?;
        ambient_sink.set_volume(AMBIENT_VOLUME);
        ambient_sink.append(WindSource::new());
        ambient_sink.play();

        let tracks = discover_music_tracks();
        let music_sink = if tracks.is_empty() {
            log::warn!(
                "No soundtrack files found in assets/music. Add licensed .ogg/.mp3/.wav/.flac files to enable music."
            );
            None
        } else {
            let sink = Sink::try_new(&handle)
                .map_err(|error| format!("failed to create music sink: {error}"))?;
            sink.set_volume(MUSIC_VOLUME);
            let queued = queue_playlist(&sink, &tracks, PLAYLIST_LOOP_COUNT);
            if queued == 0 {
                log::warn!(
                    "Found music files but could not decode any tracks. Ambient wind will continue without songs."
                );
                None
            } else {
                log::info!(
                    "Queued {queued} soundtrack entries from {} source tracks.",
                    tracks.len()
                );
                sink.play();
                Some(sink)
            }
        };

        Ok(Self {
            _stream: stream,
            _ambient_sink: ambient_sink,
            _music_sink: music_sink,
        })
    }
}

fn queue_playlist(sink: &Sink, tracks: &[PathBuf], loop_count: usize) -> usize {
    if tracks.is_empty() {
        return 0;
    }

    let mut queued = 0_usize;

    for cycle in 0..loop_count {
        for offset in 0..tracks.len() {
            let index = (offset + cycle) % tracks.len();
            let path = &tracks[index];
            match open_track(path) {
                Ok(source) => {
                    sink.append(source);
                    queued += 1;
                }
                Err(error) => {
                    log::warn!("Skipping track '{}': {error}", path.display());
                }
            }
        }
    }

    queued
}

fn open_track(path: &Path) -> Result<Decoder<BufReader<File>>, String> {
    let file = File::open(path)
        .map_err(|error| format!("failed to open track '{}': {error}", path.display()))?;
    Decoder::new(BufReader::new(file))
        .map_err(|error| format!("failed to decode track '{}': {error}", path.display()))
}

fn discover_music_tracks() -> Vec<PathBuf> {
    let mut tracks = BTreeSet::new();

    for dir in music_search_dirs() {
        if !dir.is_dir() {
            continue;
        }

        let entries = match fs::read_dir(&dir) {
            Ok(entries) => entries,
            Err(error) => {
                log::warn!(
                    "Could not read music directory '{}': {error}",
                    dir.display()
                );
                continue;
            }
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() && is_supported_audio_file(&path) {
                tracks.insert(path);
            }
        }
    }

    tracks.into_iter().collect()
}

fn music_search_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();

    if let Ok(custom_dir) = std::env::var("TOVA_MUSIC_DIR") {
        dirs.push(PathBuf::from(custom_dir));
    }

    dirs.push(PathBuf::from("assets/music"));

    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            dirs.push(exe_dir.join("assets/music"));
            dirs.push(exe_dir.join("../assets/music"));
            dirs.push(exe_dir.join("../../assets/music"));
        }
    }

    dirs
}

fn is_supported_audio_file(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| {
            let lower = ext.to_ascii_lowercase();
            SUPPORTED_EXTENSIONS
                .iter()
                .any(|supported| supported == &lower.as_str())
        })
        .unwrap_or(false)
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
        self.seed = self
            .seed
            .wrapping_mul(1_664_525)
            .wrapping_add(1_013_904_223);
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
