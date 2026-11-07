// audio/transcription/hosted_openai_provider.rs
//
// Hosted OpenAI / LiteLLM transcription provider implementation.
// Encodes raw 16kHz mono audio into WAV and sends it to an OpenAI-compatible
// audio transcription endpoint (e.g. LiteLLM proxy with indic-transcribe).

use super::provider::{TranscriptionError, TranscriptionProvider, TranscriptResult};
use async_trait::async_trait;
use log::{debug, error, info};
use reqwest::multipart::{Form, Part};
use serde::Deserialize;
use std::time::Duration;

pub struct HostedOpenAIProvider {
    client: reqwest::Client,
    endpoint: String,
    api_key: String,
    model: String,
}

impl HostedOpenAIProvider {
    pub fn new(endpoint: String, api_key: String, model: String) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(60))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());

        Self {
            client,
            endpoint,
            api_key,
            model,
        }
    }
}

/// Encode raw 16kHz mono f32 samples to standard 16-bit PCM WAV in memory
fn encode_wav_pcm16(samples: &[f32], sample_rate: u32) -> Vec<u8> {
    let num_samples = samples.len() as u32;
    let data_size = num_samples * 2;
    let file_size = 36 + data_size;
    let mut buf = Vec::with_capacity((44 + data_size) as usize);

    // RIFF header
    buf.extend_from_slice(b"RIFF");
    buf.extend_from_slice(&file_size.to_le_bytes());
    buf.extend_from_slice(b"WAVE");

    // "fmt " sub-chunk
    buf.extend_from_slice(b"fmt ");
    buf.extend_from_slice(&16u32.to_le_bytes()); // subchunk size (16 for PCM)
    buf.extend_from_slice(&1u16.to_le_bytes());  // PCM format
    buf.extend_from_slice(&1u16.to_le_bytes());  // 1 channel (mono)
    buf.extend_from_slice(&sample_rate.to_le_bytes());
    let byte_rate = sample_rate * 2;
    buf.extend_from_slice(&byte_rate.to_le_bytes());
    buf.extend_from_slice(&2u16.to_le_bytes());  // block align
    buf.extend_from_slice(&16u16.to_le_bytes()); // bits per sample

    // "data" sub-chunk
    buf.extend_from_slice(b"data");
    buf.extend_from_slice(&data_size.to_le_bytes());

    for &sample in samples {
        let clamped = sample.max(-1.0).min(1.0);
        let sample_i16 = (clamped * 32767.0) as i16;
        buf.extend_from_slice(&sample_i16.to_le_bytes());
    }

    buf
}

#[derive(Deserialize)]
struct WhisperResponse {
    text: String,
}

#[async_trait]
impl TranscriptionProvider for HostedOpenAIProvider {
    async fn transcribe(
        &self,
        audio: Vec<f32>,
        language: Option<String>,
    ) -> std::result::Result<TranscriptResult, TranscriptionError> {
        if audio.is_empty() {
            return Err(TranscriptionError::AudioTooShort {
                samples: 0,
                minimum: 1600,
            });
        }

        let wav_bytes = encode_wav_pcm16(&audio, 16000);
        let part = Part::bytes(wav_bytes)
            .file_name("audio.wav")
            .mime_str("audio/wav")
            .map_err(|e| TranscriptionError::EngineFailed(e.to_string()))?;

        let mut form = Form::new()
            .part("file", part)
            .text("model", self.model.clone());

        if let Some(ref lang) = language {
            if !lang.is_empty() && lang != "auto" {
                form = form.text("language", lang.clone());
            }
        }

        let base = self.endpoint.trim_end_matches('/');
        let url = if base.ends_with("/audio/transcriptions") {
            base.to_string()
        } else if base.ends_with("/v1") {
            format!("{}/audio/transcriptions", base)
        } else {
            format!("{}/v1/audio/transcriptions", base)
        };

        let mut request = self
            .client
            .post(&url)
            .header("bypass-tunnel-reminder", "true")
            .header("User-Agent", "Meetily/1.0")
            .multipart(form);

        if !self.api_key.is_empty() {
            request = request.header("Authorization", format!("Bearer {}", self.api_key));
        }

        debug!("Sending audio chunk to hosted ASR endpoint: {}", url);

        let response = request
            .send()
            .await
            .map_err(|e| TranscriptionError::EngineFailed(format!("Network error: {}", e)))?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            error!("Hosted ASR server returned HTTP {}: {}", status, error_text);
            return Err(TranscriptionError::EngineFailed(format!(
                "HTTP {}: {}",
                status, error_text
            )));
        }

        let result: WhisperResponse = response
            .json()
            .await
            .map_err(|e| TranscriptionError::EngineFailed(format!("Failed to parse response: {}", e)))?;

        let text = result.text.trim().to_string();
        info!("✅ Hosted ASR transcribed ({} chars): '{}'", text.len(), text);

        Ok(TranscriptResult {
            text,
            confidence: Some(1.0),
            is_partial: false,
        })
    }

    async fn is_model_loaded(&self) -> bool {
        true
    }

    async fn get_current_model(&self) -> Option<String> {
        Some(self.model.clone())
    }

    fn provider_name(&self) -> &'static str {
        "Hosted OpenAI / Indic-Transcribe"
    }
}
