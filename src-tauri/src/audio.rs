use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use hound::{WavSpec, WavWriter};
use std::io::BufWriter;
use std::fs::File;
use serde_json::json;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RecordingState {
    Idle,
    Recording,
    Paused,
}

pub struct AudioRecorder {
    audio_dir: PathBuf,
    state: RecordingState,
    current_meeting_id: Option<String>,
    sample_rate: u32,
    is_recording: Arc<AtomicBool>,
}

impl AudioRecorder {
    pub fn new(audio_dir: PathBuf) -> Self {
        AudioRecorder {
            audio_dir,
            state: RecordingState::Idle,
            current_meeting_id: None,
            sample_rate: 16000,
            is_recording: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn get_state(&self) -> serde_json::Value {
        json!({
            "state": match self.state {
                RecordingState::Idle => "idle",
                RecordingState::Recording => "recording",
                RecordingState::Paused => "paused",
            },
            "meeting_id": self.current_meeting_id,
        })
    }

    pub async fn start_recording(&mut self, meeting_id: &str) -> anyhow::Result<String> {
        if self.state == RecordingState::Recording {
            return Err(anyhow::anyhow!("Already recording"));
        }

        self.current_meeting_id = Some(meeting_id.to_string());
        self.state = RecordingState::Recording;
        self.is_recording.store(true, Ordering::SeqCst);

        let audio_path = self.audio_dir.join(format!("{}.wav", meeting_id));
        let target_sample_rate = self.sample_rate;
        let is_recording = Arc::clone(&self.is_recording);

        let audio_path_clone = audio_path.clone();

        // Spawn recording in a separate thread (cpal needs to run on a real thread, not tokio)
        thread::spawn(move || {
            if let Err(e) = record_from_microphone(audio_path_clone, target_sample_rate, is_recording) {
                eprintln!("Audio recording error: {}", e);
            }
        });

        Ok(audio_path.to_string_lossy().to_string())
    }

    pub async fn stop_recording(&mut self) -> anyhow::Result<(String, u64)> {
        if self.state == RecordingState::Idle {
            return Err(anyhow::anyhow!("Not recording"));
        }

        self.is_recording.store(false, Ordering::SeqCst);

        // Give the recording thread time to finish and finalize the file
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

        let audio_path = self.audio_dir.join(format!("{}.wav", self.current_meeting_id.as_ref().unwrap()));

        // Check if file exists and has content
        let duration = match get_audio_duration(audio_path.to_string_lossy().as_ref()) {
            Ok(d) => d,
            Err(e) => {
                eprintln!("Warning: Could not get audio duration: {}", e);
                0.0
            }
        };

        self.state = RecordingState::Idle;
        let meeting_id = self.current_meeting_id.take().unwrap();

        Ok((meeting_id, duration as u64))
    }

    pub async fn pause_recording(&mut self) -> anyhow::Result<()> {
        if self.state != RecordingState::Recording {
            return Err(anyhow::anyhow!("Not recording"));
        }

        self.state = RecordingState::Paused;
        Ok(())
    }

    pub async fn resume_recording(&mut self) -> anyhow::Result<()> {
        if self.state != RecordingState::Paused {
            return Err(anyhow::anyhow!("Not paused"));
        }

        self.state = RecordingState::Recording;
        Ok(())
    }
}

fn record_from_microphone(
    output_path: PathBuf,
    target_sample_rate: u32,
    is_recording: Arc<AtomicBool>,
) -> anyhow::Result<()> {
    let host = cpal::default_host();
    let device = host.default_input_device()
        .ok_or_else(|| anyhow::anyhow!("No input device available"))?;

    let config = device.default_input_config()?;
    let input_sample_rate = config.sample_rate().0;
    let channels = config.channels();

    println!("Recording from: {:?}", device.name()?);
    println!("Input sample rate: {}, Channels: {}, Target: {}", input_sample_rate, channels, target_sample_rate);

    // Create WAV file with target sample rate
    let spec = WavSpec {
        channels: 1,
        sample_rate: target_sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };

    let file = File::create(&output_path)?;
    let buf_writer = BufWriter::new(file);
    let wav_writer = Arc::new(std::sync::Mutex::new(Some(WavWriter::new(buf_writer, spec)?)));

    let wav_writer_clone = Arc::clone(&wav_writer);
    let is_recording_clone = Arc::clone(&is_recording);

    // Buffer for resampling
    let resample_ratio = target_sample_rate as f64 / input_sample_rate as f64;

    let err_fn = |err| eprintln!("Audio stream error: {}", err);

    let stream = match config.sample_format() {
        cpal::SampleFormat::F32 => {
            device.build_input_stream(
                &config.into(),
                move |data: &[f32], _: &cpal::InputCallbackInfo| {
                    if !is_recording_clone.load(Ordering::SeqCst) {
                        return;
                    }

                    // Convert to mono
                    let mono: Vec<f32> = if channels == 2 {
                        data.chunks_exact(2).map(|c| (c[0] + c[1]) / 2.0).collect()
                    } else {
                        data.to_vec()
                    };

                    // Simple resampling (linear interpolation)
                    let resampled = if (resample_ratio - 1.0).abs() > 0.01 {
                        let output_len = (mono.len() as f64 * resample_ratio) as usize;
                        let mut output = Vec::with_capacity(output_len);
                        for i in 0..output_len {
                            let src_idx = i as f64 / resample_ratio;
                            let idx0 = src_idx.floor() as usize;
                            let idx1 = (idx0 + 1).min(mono.len().saturating_sub(1));
                            let frac = src_idx - idx0 as f64;
                            let sample = mono.get(idx0).copied().unwrap_or(0.0) * (1.0 - frac as f32)
                                + mono.get(idx1).copied().unwrap_or(0.0) * frac as f32;
                            output.push(sample);
                        }
                        output
                    } else {
                        mono
                    };

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
                err_fn,
                None,
            )?
        }
        cpal::SampleFormat::I16 => {
            device.build_input_stream(
                &config.into(),
                move |data: &[i16], _: &cpal::InputCallbackInfo| {
                    if !is_recording_clone.load(Ordering::SeqCst) {
                        return;
                    }

                    // Convert to f32 mono
                    let mono: Vec<f32> = if channels == 2 {
                        data.chunks_exact(2)
                            .map(|c| ((c[0] as f32 + c[1] as f32) / 2.0) / 32768.0)
                            .collect()
                    } else {
                        data.iter().map(|&s| s as f32 / 32768.0).collect()
                    };

                    // Simple resampling
                    let resampled = if (resample_ratio - 1.0).abs() > 0.01 {
                        let output_len = (mono.len() as f64 * resample_ratio) as usize;
                        let mut output = Vec::with_capacity(output_len);
                        for i in 0..output_len {
                            let src_idx = i as f64 / resample_ratio;
                            let idx0 = src_idx.floor() as usize;
                            let idx1 = (idx0 + 1).min(mono.len().saturating_sub(1));
                            let frac = src_idx - idx0 as f64;
                            let sample = mono.get(idx0).copied().unwrap_or(0.0) * (1.0 - frac as f32)
                                + mono.get(idx1).copied().unwrap_or(0.0) * frac as f32;
                            output.push(sample);
                        }
                        output
                    } else {
                        mono
                    };

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
                err_fn,
                None,
            )?
        }
        format => return Err(anyhow::anyhow!("Unsupported sample format: {:?}", format)),
    };

    stream.play()?;
    println!("Recording started...");

    // Keep the stream alive while recording
    while is_recording.load(Ordering::SeqCst) {
        thread::sleep(std::time::Duration::from_millis(100));
    }

    // Stop and finalize
    drop(stream);

    // Finalize the WAV file
    if let Ok(mut guard) = wav_writer.lock() {
        if let Some(writer) = guard.take() {
            writer.finalize()?;
            println!("Recording saved to: {:?}", output_path);
        }
    }

    Ok(())
}

pub fn get_audio_duration(audio_path: &str) -> anyhow::Result<f64> {
    let reader = hound::WavReader::open(audio_path)?;
    let spec = reader.spec();
    let duration_samples = reader.len();
    let duration_seconds = duration_samples as f64 / spec.sample_rate as f64;

    Ok(duration_seconds)
}

pub fn resample_audio(input: &[f32], input_rate: u32, output_rate: u32) -> anyhow::Result<Vec<f32>> {
    use rubato::{SincInterpolationParameters, SincInterpolationType, Resampler, SincFixedIn};

    if input_rate == output_rate {
        return Ok(input.to_vec());
    }

    let params = SincInterpolationParameters {
        sinc_len: 256,
        f_cutoff: 0.95,
        interpolation: SincInterpolationType::Linear,
        oversampling_factor: 256,
        window: rubato::WindowFunction::BlackmanHarris2,
    };

    let mut resampler = SincFixedIn::<f32>::new(
        output_rate as f64 / input_rate as f64,
        2.0,
        params,
        input.len(),
        1,
    )?;

    let waves_in = vec![input.to_vec()];
    let waves_out = resampler.process(&waves_in, None)?;

    Ok(waves_out.into_iter().next().unwrap_or_default())
}
