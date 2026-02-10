use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::collections::VecDeque;
use hound::{WavSpec, WavWriter};
use std::io::BufWriter;
use std::fs::File;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use crossbeam_channel::{bounded, Sender, Receiver};

use crate::audio::{list_audio_devices, get_audio_duration};

/// Represents a single transcribed segment with speaker attribution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpeakerSegment {
    pub speaker: String, // "Me" or "Them"
    pub text: String,
    pub start_ms: u64,
    pub end_ms: u64,
    pub is_overlapping: bool,
}

/// Complete transcript with speaker attribution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpeakerTranscript {
    pub version: u32,
    pub mic_device: String,
    pub system_device: String,
    pub has_dual_audio: bool,
    pub segments: Vec<SpeakerSegment>,
}

impl SpeakerTranscript {
    pub fn new(mic_device: String, system_device: String) -> Self {
        Self {
            version: 1,
            mic_device,
            system_device,
            has_dual_audio: true,
            segments: Vec::new(),
        }
    }

    /// Merge two transcripts (one per channel) with overlap detection
    pub fn merge(
        mut mic_segments: Vec<SpeakerSegment>,
        mut system_segments: Vec<SpeakerSegment>,
    ) -> Vec<SpeakerSegment> {
        // Ensure segments are labeled correctly
        for seg in &mut mic_segments {
            seg.speaker = "Me".to_string();
        }
        for seg in &mut system_segments {
            seg.speaker = "Them".to_string();
        }

        let mut all_segments: Vec<SpeakerSegment> = mic_segments
            .into_iter()
            .chain(system_segments)
            .collect();

        // Sort by start time
        all_segments.sort_by_key(|s| s.start_ms);

        // Detect overlaps
        for i in 1..all_segments.len() {
            let prev_end = all_segments[i - 1].end_ms;
            let curr_start = all_segments[i].start_ms;
            let speakers_differ = all_segments[i - 1].speaker != all_segments[i].speaker;

            if prev_end > curr_start && speakers_differ {
                all_segments[i].is_overlapping = true;
            }
        }

        all_segments
    }
}

/// Status returned when dual recording starts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DualRecordingStatus {
    pub audio_path: String,
    pub mic_active: bool,
    pub system_active: bool,
    pub mic_device: String,
    pub system_device: String,
}

/// Result returned when dual recording stops
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DualRecordingResult {
    pub meeting_id: String,
    pub duration_secs: u64,
    pub is_dual_audio: bool,
    pub mic_captured: bool,
    pub system_captured: bool,
}

/// Audio source error event payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioSourceError {
    pub source: String, // "mic" or "system"
    pub error: String,
    pub timestamp: u64,
    pub recording_continues: bool,
}


/// Writes interleaved stereo samples to a WAV file
pub struct StereoWavWriter {
    writer: WavWriter<BufWriter<File>>,
    left_buffer: VecDeque<f32>,
    right_buffer: VecDeque<f32>,
    samples_written: u64,
}

impl StereoWavWriter {
    pub fn new(output_path: &PathBuf) -> anyhow::Result<Self> {
        let spec = WavSpec {
            channels: 2,
            sample_rate: 16000,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };

        let file = File::create(output_path)?;
        let buf_writer = BufWriter::new(file);
        let writer = WavWriter::new(buf_writer, spec)?;

        Ok(Self {
            writer,
            left_buffer: VecDeque::new(),
            right_buffer: VecDeque::new(),
            samples_written: 0,
        })
    }

    /// Write a frame of interleaved samples (left, right)
    pub fn write_frame(&mut self, left: f32, right: f32) -> anyhow::Result<()> {
        let left_clamped = left.max(-1.0).min(1.0);
        let right_clamped = right.max(-1.0).min(1.0);

        let left_int = (left_clamped * 32767.0) as i16;
        let right_int = (right_clamped * 32767.0) as i16;

        self.writer.write_sample(left_int)?;
        self.writer.write_sample(right_int)?;
        self.samples_written += 1;

        Ok(())
    }

    /// Buffer samples from left channel (mic)
    pub fn buffer_left(&mut self, samples: &[f32]) {
        self.left_buffer.extend(samples);
    }

    /// Buffer samples from right channel (system)
    pub fn buffer_right(&mut self, samples: &[f32]) {
        self.right_buffer.extend(samples);
    }

    /// Flush buffered samples as interleaved frames
    /// Writes zeros for the channel that doesn't have data yet
    pub fn flush_buffers(&mut self) -> anyhow::Result<()> {
        while !self.left_buffer.is_empty() || !self.right_buffer.is_empty() {
            let left = self.left_buffer.pop_front().unwrap_or(0.0);
            let right = self.right_buffer.pop_front().unwrap_or(0.0);
            self.write_frame(left, right)?;
        }
        Ok(())
    }

    /// Finalize the WAV file
    pub fn finalize(self) -> anyhow::Result<()> {
        self.writer.finalize()?;
        Ok(())
    }
}

/// Auto-detect default microphone and system audio devices
pub fn get_default_devices() -> (Option<usize>, Option<usize>) {
    let devices = match list_audio_devices() {
        Ok(d) => d,
        Err(_) => return (None, None),
    };

    // Default mic: first non-monitor device
    let mic = devices.iter().position(|d| !d.is_monitor);

    // Default system: first monitor source
    let system = devices.iter().position(|d| d.is_monitor);

    (mic, system)
}

/// Get device by index from cpal
fn get_device_by_index(index: usize) -> anyhow::Result<cpal::Device> {
    let host = cpal::default_host();
    host.input_devices()?
        .nth(index)
        .ok_or_else(|| anyhow::anyhow!("Device with index {} not found", index))
}

/// Coordinates dual audio stream recording
pub struct DualAudioRecorder {
    audio_dir: PathBuf,
    current_meeting_id: Option<String>,
    is_recording: Arc<AtomicBool>,
    mic_active: Arc<AtomicBool>,
    system_active: Arc<AtomicBool>,
    mic_device_name: String,
    system_device_name: String,
    sample_rate: u32,
}

impl DualAudioRecorder {
    pub fn new(audio_dir: PathBuf) -> Self {
        Self {
            audio_dir,
            current_meeting_id: None,
            is_recording: Arc::new(AtomicBool::new(false)),
            mic_active: Arc::new(AtomicBool::new(false)),
            system_active: Arc::new(AtomicBool::new(false)),
            mic_device_name: String::new(),
            system_device_name: String::new(),
            sample_rate: 16000,
        }
    }

    /// Start recording from both microphone and system audio sources
    /// System audio is optional - if not available, records mic only
    pub fn start(
        &mut self,
        meeting_id: &str,
        mic_device_index: Option<usize>,
        system_device_index: Option<usize>,
    ) -> anyhow::Result<DualRecordingStatus> {
        if self.is_recording.load(Ordering::SeqCst) {
            return Err(anyhow::anyhow!("Already recording"));
        }

        // Get device indices (auto-detect if not specified)
        let (default_mic, default_system) = get_default_devices();
        let mic_idx = mic_device_index.or(default_mic)
            .ok_or_else(|| anyhow::anyhow!("No microphone device available"))?;

        // System audio is optional
        let system_idx = system_device_index.or(default_system);

        // Get mic device (required)
        let mic_device = get_device_by_index(mic_idx)?;
        self.mic_device_name = mic_device.name().unwrap_or_else(|_| "Unknown Mic".to_string());

        // Get system device (optional)
        let has_system_audio = if let Some(idx) = system_idx {
            match get_device_by_index(idx) {
                Ok(device) => {
                    self.system_device_name = device.name().unwrap_or_else(|_| "Unknown System".to_string());
                    true
                }
                Err(e) => {
                    eprintln!("System audio device not available: {}", e);
                    self.system_device_name = "Not available".to_string();
                    false
                }
            }
        } else {
            eprintln!("No system audio monitor source detected - recording mic only");
            self.system_device_name = "Not available".to_string();
            false
        };

        self.current_meeting_id = Some(meeting_id.to_string());
        self.is_recording.store(true, Ordering::SeqCst);
        self.mic_active.store(true, Ordering::SeqCst);
        self.system_active.store(has_system_audio, Ordering::SeqCst);

        let audio_path = self.audio_dir.join(format!("{}.wav", meeting_id));
        let audio_path_str = audio_path.to_string_lossy().to_string();

        let is_recording = Arc::clone(&self.is_recording);
        let mic_active = Arc::clone(&self.mic_active);
        let system_active = Arc::clone(&self.system_active);
        let target_sample_rate = self.sample_rate;

        let audio_path_clone = audio_path.clone();

        // Spawn recording in a separate thread
        thread::spawn(move || {
            let result = if has_system_audio {
                record_dual_streams(
                    audio_path_clone,
                    target_sample_rate,
                    is_recording,
                    mic_active,
                    system_active,
                    mic_idx,
                    system_idx.unwrap(),
                )
            } else {
                record_mono_stream(
                    audio_path_clone,
                    target_sample_rate,
                    is_recording,
                    mic_idx,
                )
            };

            if let Err(e) = result {
                eprintln!("Audio recording error: {}", e);
            }
        });

        Ok(DualRecordingStatus {
            audio_path: audio_path_str,
            mic_active: true,
            system_active: has_system_audio,
            mic_device: self.mic_device_name.clone(),
            system_device: self.system_device_name.clone(),
        })
    }

    /// Stop the dual recording
    pub async fn stop(&mut self) -> anyhow::Result<DualRecordingResult> {
        if !self.is_recording.load(Ordering::SeqCst) {
            return Err(anyhow::anyhow!("Not recording"));
        }

        let mic_captured = self.mic_active.load(Ordering::SeqCst);
        let system_captured = self.system_active.load(Ordering::SeqCst);

        self.is_recording.store(false, Ordering::SeqCst);

        // Give the recording thread time to finish
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

        let meeting_id = self.current_meeting_id.take().unwrap_or_default();
        let audio_path = self.audio_dir.join(format!("{}.wav", meeting_id));

        let duration = match get_audio_duration(audio_path.to_string_lossy().as_ref()) {
            Ok(d) => d as u64,
            Err(e) => {
                eprintln!("Warning: Could not get audio duration: {}", e);
                0
            }
        };

        self.mic_active.store(false, Ordering::SeqCst);
        self.system_active.store(false, Ordering::SeqCst);

        Ok(DualRecordingResult {
            meeting_id,
            duration_secs: duration,
            is_dual_audio: mic_captured && system_captured,
            mic_captured,
            system_captured,
        })
    }

    pub fn get_status(&self) -> serde_json::Value {
        serde_json::json!({
            "is_recording": self.is_recording.load(Ordering::SeqCst),
            "mic_active": self.mic_active.load(Ordering::SeqCst),
            "system_active": self.system_active.load(Ordering::SeqCst),
            "mic_device": self.mic_device_name,
            "system_device": self.system_device_name,
            "meeting_id": self.current_meeting_id,
        })
    }
}

/// Record from a single microphone into a mono WAV file
fn record_mono_stream(
    output_path: PathBuf,
    target_sample_rate: u32,
    is_recording: Arc<AtomicBool>,
    mic_device_index: usize,
) -> anyhow::Result<()> {
    let mic_device = get_device_by_index(mic_device_index)?;

    println!("Recording from mic: {:?}", mic_device.name()?);

    let mic_config = mic_device.default_input_config()?;
    let mic_sample_rate = mic_config.sample_rate().0;
    let mic_channels = mic_config.channels();

    println!("Mic sample rate: {}, channels: {}", mic_sample_rate, mic_channels);

    // Create mono WAV file
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: target_sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };

    let file = File::create(&output_path)?;
    let buf_writer = BufWriter::new(file);
    let wav_writer = Arc::new(std::sync::Mutex::new(Some(hound::WavWriter::new(buf_writer, spec)?)));

    let wav_writer_clone = Arc::clone(&wav_writer);
    let is_recording_clone = Arc::clone(&is_recording);
    let resample_ratio = target_sample_rate as f64 / mic_sample_rate as f64;

    let stream = match mic_config.sample_format() {
        cpal::SampleFormat::F32 => {
            mic_device.build_input_stream(
                &mic_config.into(),
                move |data: &[f32], _: &cpal::InputCallbackInfo| {
                    if !is_recording_clone.load(Ordering::SeqCst) {
                        return;
                    }

                    // Convert to mono
                    let mono: Vec<f32> = if mic_channels == 2 {
                        data.chunks_exact(2).map(|c| (c[0] + c[1]) / 2.0).collect()
                    } else {
                        data.to_vec()
                    };

                    // Resample
                    let resampled = resample_linear(&mono, resample_ratio);

                    // Write to WAV
                    if let Ok(mut guard) = wav_writer_clone.lock() {
                        if let Some(ref mut writer) = *guard {
                            for sample in resampled {
                                let clamped = sample.max(-1.0).min(1.0);
                                let int_sample = (clamped * 32767.0) as i16;
                                let _ = writer.write_sample(int_sample);
                            }
                        }
                    }
                },
                |err| eprintln!("Mic stream error: {}", err),
                None,
            )?
        }
        cpal::SampleFormat::I16 => {
            mic_device.build_input_stream(
                &mic_config.into(),
                move |data: &[i16], _: &cpal::InputCallbackInfo| {
                    if !is_recording_clone.load(Ordering::SeqCst) {
                        return;
                    }

                    let mono: Vec<f32> = if mic_channels == 2 {
                        data.chunks_exact(2)
                            .map(|c| ((c[0] as f32 + c[1] as f32) / 2.0) / 32768.0)
                            .collect()
                    } else {
                        data.iter().map(|&s| s as f32 / 32768.0).collect()
                    };

                    let resampled = resample_linear(&mono, resample_ratio);

                    if let Ok(mut guard) = wav_writer_clone.lock() {
                        if let Some(ref mut writer) = *guard {
                            for sample in resampled {
                                let clamped = sample.max(-1.0).min(1.0);
                                let int_sample = (clamped * 32767.0) as i16;
                                let _ = writer.write_sample(int_sample);
                            }
                        }
                    }
                },
                |err| eprintln!("Mic stream error: {}", err),
                None,
            )?
        }
        format => return Err(anyhow::anyhow!("Unsupported sample format: {:?}", format)),
    };

    stream.play()?;
    println!("Mono recording started...");

    // Keep stream alive while recording
    while is_recording.load(Ordering::SeqCst) {
        thread::sleep(std::time::Duration::from_millis(100));
    }

    drop(stream);

    // Finalize WAV
    if let Ok(mut guard) = wav_writer.lock() {
        if let Some(writer) = guard.take() {
            writer.finalize()?;
            println!("Recording saved to: {:?}", output_path);
        }
    }

    Ok(())
}

/// Record from two audio streams simultaneously into a stereo WAV file
fn record_dual_streams(
    output_path: PathBuf,
    target_sample_rate: u32,
    is_recording: Arc<AtomicBool>,
    mic_active: Arc<AtomicBool>,
    system_active: Arc<AtomicBool>,
    mic_device_index: usize,
    system_device_index: usize,
) -> anyhow::Result<()> {
    // Get devices
    let mic_device = get_device_by_index(mic_device_index)?;
    let system_device = get_device_by_index(system_device_index)?;

    println!("Recording from mic: {:?}", mic_device.name()?);
    println!("Recording from system: {:?}", system_device.name()?);

    // Get configs
    let mic_config = mic_device.default_input_config()?;
    let system_config = system_device.default_input_config()?;

    let mic_sample_rate = mic_config.sample_rate().0;
    let system_sample_rate = system_config.sample_rate().0;
    let mic_channels = mic_config.channels();
    let system_channels = system_config.channels();

    // Create channels for sample passing
    let (mic_tx, mic_rx): (Sender<Vec<f32>>, Receiver<Vec<f32>>) = bounded(100);
    let (system_tx, system_rx): (Sender<Vec<f32>>, Receiver<Vec<f32>>) = bounded(100);

    // Spawn writer thread
    let is_recording_writer = Arc::clone(&is_recording);
    let writer_handle = thread::spawn(move || {
        let mut stereo_writer = match StereoWavWriter::new(&output_path) {
            Ok(w) => w,
            Err(e) => {
                eprintln!("Failed to create stereo writer: {}", e);
                return;
            }
        };

        while is_recording_writer.load(Ordering::SeqCst) {
            // Non-blocking receive from both channels
            let mic_samples = mic_rx.try_recv().ok();
            let system_samples = system_rx.try_recv().ok();

            if let Some(samples) = mic_samples {
                stereo_writer.buffer_left(&samples);
            }
            if let Some(samples) = system_samples {
                stereo_writer.buffer_right(&samples);
            }

            // Flush what we have
            if let Err(e) = stereo_writer.flush_buffers() {
                eprintln!("Error writing samples: {}", e);
            }

            // Small sleep to prevent busy-waiting
            thread::sleep(std::time::Duration::from_millis(10));
        }

        // Final flush
        let _ = stereo_writer.flush_buffers();
        if let Err(e) = stereo_writer.finalize() {
            eprintln!("Error finalizing WAV: {}", e);
        }
        println!("Recording saved to: {:?}", output_path);
    });

    // Build mic stream
    let is_recording_mic = Arc::clone(&is_recording);
    let mic_active_clone = Arc::clone(&mic_active);
    let mic_resample_ratio = target_sample_rate as f64 / mic_sample_rate as f64;

    let mic_stream = match mic_config.sample_format() {
        cpal::SampleFormat::F32 => {
            mic_device.build_input_stream(
                &mic_config.into(),
                move |data: &[f32], _: &cpal::InputCallbackInfo| {
                    if !is_recording_mic.load(Ordering::SeqCst) {
                        return;
                    }

                    // Convert to mono
                    let mono: Vec<f32> = if mic_channels == 2 {
                        data.chunks_exact(2).map(|c| (c[0] + c[1]) / 2.0).collect()
                    } else {
                        data.to_vec()
                    };

                    // Resample
                    let resampled = resample_linear(&mono, mic_resample_ratio);

                    let _ = mic_tx.try_send(resampled);
                },
                move |err| {
                    eprintln!("Mic stream error: {}", err);
                    mic_active_clone.store(false, Ordering::SeqCst);
                },
                None,
            )?
        }
        cpal::SampleFormat::I16 => {
            mic_device.build_input_stream(
                &mic_config.into(),
                move |data: &[i16], _: &cpal::InputCallbackInfo| {
                    if !is_recording_mic.load(Ordering::SeqCst) {
                        return;
                    }

                    let mono: Vec<f32> = if mic_channels == 2 {
                        data.chunks_exact(2)
                            .map(|c| ((c[0] as f32 + c[1] as f32) / 2.0) / 32768.0)
                            .collect()
                    } else {
                        data.iter().map(|&s| s as f32 / 32768.0).collect()
                    };

                    let resampled = resample_linear(&mono, mic_resample_ratio);
                    let _ = mic_tx.try_send(resampled);
                },
                move |err| {
                    eprintln!("Mic stream error: {}", err);
                    mic_active_clone.store(false, Ordering::SeqCst);
                },
                None,
            )?
        }
        format => return Err(anyhow::anyhow!("Unsupported mic sample format: {:?}", format)),
    };

    // Build system stream
    let is_recording_system = Arc::clone(&is_recording);
    let system_active_clone = Arc::clone(&system_active);
    let system_resample_ratio = target_sample_rate as f64 / system_sample_rate as f64;

    let system_stream = match system_config.sample_format() {
        cpal::SampleFormat::F32 => {
            system_device.build_input_stream(
                &system_config.into(),
                move |data: &[f32], _: &cpal::InputCallbackInfo| {
                    if !is_recording_system.load(Ordering::SeqCst) {
                        return;
                    }

                    let mono: Vec<f32> = if system_channels == 2 {
                        data.chunks_exact(2).map(|c| (c[0] + c[1]) / 2.0).collect()
                    } else {
                        data.to_vec()
                    };

                    let resampled = resample_linear(&mono, system_resample_ratio);
                    let _ = system_tx.try_send(resampled);
                },
                move |err| {
                    eprintln!("System stream error: {}", err);
                    system_active_clone.store(false, Ordering::SeqCst);
                },
                None,
            )?
        }
        cpal::SampleFormat::I16 => {
            system_device.build_input_stream(
                &system_config.into(),
                move |data: &[i16], _: &cpal::InputCallbackInfo| {
                    if !is_recording_system.load(Ordering::SeqCst) {
                        return;
                    }

                    let mono: Vec<f32> = if system_channels == 2 {
                        data.chunks_exact(2)
                            .map(|c| ((c[0] as f32 + c[1] as f32) / 2.0) / 32768.0)
                            .collect()
                    } else {
                        data.iter().map(|&s| s as f32 / 32768.0).collect()
                    };

                    let resampled = resample_linear(&mono, system_resample_ratio);
                    let _ = system_tx.try_send(resampled);
                },
                move |err| {
                    eprintln!("System stream error: {}", err);
                    system_active_clone.store(false, Ordering::SeqCst);
                },
                None,
            )?
        }
        format => return Err(anyhow::anyhow!("Unsupported system sample format: {:?}", format)),
    };

    // Start both streams
    mic_stream.play()?;
    system_stream.play()?;
    println!("Dual recording started...");

    // Keep streams alive while recording
    while is_recording.load(Ordering::SeqCst) {
        thread::sleep(std::time::Duration::from_millis(100));
    }

    // Stop streams
    drop(mic_stream);
    drop(system_stream);

    // Wait for writer to finish
    let _ = writer_handle.join();

    Ok(())
}

/// Simple linear interpolation resampling
fn resample_linear(samples: &[f32], ratio: f64) -> Vec<f32> {
    if (ratio - 1.0).abs() < 0.01 {
        return samples.to_vec();
    }

    let output_len = (samples.len() as f64 * ratio) as usize;
    let mut output = Vec::with_capacity(output_len);

    for i in 0..output_len {
        let src_idx = i as f64 / ratio;
        let idx0 = src_idx.floor() as usize;
        let idx1 = (idx0 + 1).min(samples.len().saturating_sub(1));
        let frac = src_idx - idx0 as f64;
        let sample = samples.get(idx0).copied().unwrap_or(0.0) * (1.0 - frac as f32)
            + samples.get(idx1).copied().unwrap_or(0.0) * frac as f32;
        output.push(sample);
    }

    output
}
