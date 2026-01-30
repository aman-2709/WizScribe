use whisper_rs::{WhisperContext, WhisperContextParameters, FullParams, SamplingStrategy};
use std::path::Path;
use std::sync::Arc;
use tokio::sync::Mutex;
use hound::WavReader;

pub struct WhisperTranscriber {
    context: Arc<Mutex<Option<WhisperContext>>>,
    model_path: String,
}

impl WhisperTranscriber {
    pub fn new() -> anyhow::Result<Self> {
        let model_path = Self::get_default_model_path()?;
        
        Ok(WhisperTranscriber {
            context: Arc::new(Mutex::new(None)),
            model_path,
        })
    }
    
    pub fn new_with_model(model_path: &str) -> anyhow::Result<Self> {
        Ok(WhisperTranscriber {
            context: Arc::new(Mutex::new(None)),
            model_path: model_path.to_string(),
        })
    }
    
    fn get_default_model_path() -> anyhow::Result<String> {
        // Check common model locations
        let possible_paths = vec![
            dirs::home_dir().map(|h| h.join(".local/share/wizscribe/models/ggml-base.en.bin")),
            dirs::home_dir().map(|h| h.join(".wizscribe/models/ggml-base.en.bin")),
            Some(std::path::PathBuf::from("/usr/local/share/wizscribe/models/ggml-base.en.bin")),
            Some(std::path::PathBuf::from("./models/ggml-base.en.bin")),
        ];
        
        for path in possible_paths.into_iter().flatten() {
            if path.exists() {
                return Ok(path.to_string_lossy().to_string());
            }
        }
        
        // Return a default path even if it doesn't exist yet
        Ok(dirs::home_dir()
            .map(|h| h.join(".local/share/wizscribe/models/ggml-base.en.bin"))
            .unwrap_or_default()
            .to_string_lossy()
            .to_string())
    }
    
    async fn ensure_context_loaded(&self) -> anyhow::Result<()> {
        let mut ctx = self.context.lock().await;
        
        if ctx.is_none() {
            if !Path::new(&self.model_path).exists() {
                return Err(anyhow::anyhow!(
                    "Whisper model not found at {}. Please download a model file.",
                    self.model_path
                ));
            }
            
            let ctx_params = WhisperContextParameters::default();
            let whisper_ctx = WhisperContext::new_with_params(&self.model_path, ctx_params)
                .map_err(|e| anyhow::anyhow!("Failed to load Whisper model: {:?}", e))?;
            
            *ctx = Some(whisper_ctx);
        }
        
        Ok(())
    }
    
    pub async fn transcribe(&self, audio_path: &str) -> anyhow::Result<String> {
        self.ensure_context_loaded().await?;
        
        // Read and preprocess audio
        let audio_data = self.load_audio(audio_path)?;
        
        let ctx = self.context.lock().await;
        let whisper_ctx = ctx.as_ref().ok_or_else(|| anyhow::anyhow!("Whisper context not loaded"))?;

        // Create a state for this transcription
        let mut state = whisper_ctx.create_state()
            .map_err(|e| anyhow::anyhow!("Failed to create whisper state: {:?}", e))?;

        // Set up transcription parameters
        let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
        params.set_n_threads(4);
        params.set_translate(false);
        params.set_language(Some("en"));
        params.set_print_special(false);
        params.set_print_progress(false);
        params.set_print_realtime(false);
        params.set_print_timestamps(true);

        // Run transcription
        state.full(params, &audio_data)
            .map_err(|e| anyhow::anyhow!("Transcription failed: {:?}", e))?;

        // Extract results
        let num_segments = state.full_n_segments()
            .map_err(|e| anyhow::anyhow!("Failed to get segment count: {:?}", e))?;

        let mut transcript = String::new();

        for i in 0..num_segments {
            let segment = state.full_get_segment_text(i)
                .map_err(|e| anyhow::anyhow!("Failed to get segment text: {:?}", e))?;

            let start = state.full_get_segment_t0(i)
                .map_err(|e| anyhow::anyhow!("Failed to get segment start: {:?}", e))?;

            let end = state.full_get_segment_t1(i)
                .map_err(|e| anyhow::anyhow!("Failed to get segment end: {:?}", e))?;
            
            // Format timestamp as [MM:SS.mmm]
            let start_secs = start as f64 / 100.0;
            let end_secs = end as f64 / 100.0;
            
            let start_mins = (start_secs / 60.0) as i32;
            let start_secs_rem = start_secs % 60.0;
            let start_ms = ((start_secs_rem - start_secs_rem.floor()) * 1000.0) as i32;
            
            transcript.push_str(&format!(
                "[{:02}:{:02}.{:03}] - [{:02}:{:02}.{:03}] {}\n",
                start_mins,
                start_secs_rem as i32,
                start_ms,
                (end_secs / 60.0) as i32,
                (end_secs % 60.0) as i32,
                ((end_secs % 1.0) * 1000.0) as i32,
                segment.trim()
            ));
        }
        
        Ok(transcript.trim().to_string())
    }
    
    fn load_audio(&self, audio_path: &str) -> anyhow::Result<Vec<f32>> {
        let mut reader = WavReader::open(audio_path)?;
        let spec = reader.spec();
        
        // Convert to mono f32 at 16kHz
        let samples: Vec<f32> = match spec.sample_format {
            hound::SampleFormat::Int => {
                let max_val = (1 << (spec.bits_per_sample - 1)) as f32;
                reader.samples::<i32>()
                    .filter_map(|s| s.ok())
                    .map(|s| s as f32 / max_val)
                    .collect()
            }
            hound::SampleFormat::Float => {
                reader.samples::<f32>()
                    .filter_map(|s| s.ok())
                    .collect()
            }
        };
        
        // Convert to mono if stereo
        let mono_samples = if spec.channels == 2 {
            samples.chunks_exact(2)
                .map(|chunk| (chunk[0] + chunk[1]) / 2.0)
                .collect()
        } else {
            samples
        };
        
        // Resample to 16kHz if needed
        if spec.sample_rate != 16000 {
            crate::audio::resample_audio(&mono_samples, spec.sample_rate, 16000)
        } else {
            Ok(mono_samples)
        }
    }
    
    pub async fn transcribe_with_progress<F>(
        &self,
        audio_path: &str,
        _progress_callback: F,
    ) -> anyhow::Result<String>
    where
        F: Fn(f32) + Send + 'static,
    {
        // For now, just call the regular transcribe method
        // In the future, this could use a callback for progress updates
        self.transcribe(audio_path).await
    }
    
    pub fn is_model_available(&self) -> bool {
        Path::new(&self.model_path).exists()
    }
    
    pub fn get_model_path(&self) -> &str {
        &self.model_path
    }
    
    pub async fn set_model(&self, model_path: &str) -> anyhow::Result<()> {
        let mut ctx = self.context.lock().await;
        *ctx = None; // Force reload with new model
        
        if !Path::new(model_path).exists() {
            return Err(anyhow::anyhow!("Model file not found: {}", model_path));
        }
        
        let ctx_params = WhisperContextParameters::default();
        let whisper_ctx = WhisperContext::new_with_params(model_path, ctx_params)
            .map_err(|e| anyhow::anyhow!("Failed to load Whisper model: {:?}", e))?;
        
        *ctx = Some(whisper_ctx);
        
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct TranscriptionSegment {
    pub start_ms: i64,
    pub end_ms: i64,
    pub text: String,
}

pub fn parse_transcript_to_segments(transcript: &str) -> Vec<TranscriptionSegment> {
    use regex::Regex;
    
    let re = Regex::new(r"\[(\d{2}):(\d{2})\.(\d{3})\]\s+-\s+\[(\d{2}):(\d{2})\.(\d{3})\]\s+(.+)").unwrap();
    
    re.captures_iter(transcript)
        .filter_map(|cap| {
            let start_mins: i64 = cap.get(1)?.as_str().parse().ok()?;
            let start_secs: i64 = cap.get(2)?.as_str().parse().ok()?;
            let start_ms: i64 = cap.get(3)?.as_str().parse().ok()?;
            
            let end_mins: i64 = cap.get(4)?.as_str().parse().ok()?;
            let end_secs: i64 = cap.get(5)?.as_str().parse().ok()?;
            let end_ms: i64 = cap.get(6)?.as_str().parse().ok()?;
            
            let text = cap.get(7)?.as_str().to_string();
            
            Some(TranscriptionSegment {
                start_ms: (start_mins * 60 + start_secs) * 1000 + start_ms,
                end_ms: (end_mins * 60 + end_secs) * 1000 + end_ms,
                text,
            })
        })
        .collect()
}
