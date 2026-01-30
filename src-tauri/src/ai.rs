use reqwest::{Client, header};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AIProvider {
    OpenAI,
    Anthropic,
}

impl Default for AIProvider {
    fn default() -> Self {
        AIProvider::OpenAI
    }
}

pub struct AIClient {
    client: Client,
    api_key: Arc<Mutex<Option<String>>>,
    provider: Arc<Mutex<AIProvider>>,
}

impl AIClient {
    pub fn new() -> anyhow::Result<Self> {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(120))
            .build()?;
        
        Ok(AIClient {
            client,
            api_key: Arc::new(Mutex::new(None)),
            provider: Arc::new(Mutex::new(AIProvider::default())),
        })
    }
    
    pub async fn set_api_key(&self, api_key: &str, provider: &str) -> anyhow::Result<()> {
        let mut key = self.api_key.lock().await;
        *key = Some(api_key.to_string());

        let mut prov = self.provider.lock().await;
        *prov = match provider.to_lowercase().as_str() {
            "anthropic" | "claude" => AIProvider::Anthropic,
            _ => AIProvider::OpenAI,
        };

        println!("API key set for provider: {:?}", *prov);
        Ok(())
    }
    
    async fn get_api_key(&self) -> anyhow::Result<String> {
        let key = self.api_key.lock().await;
        key.clone().ok_or_else(|| anyhow::anyhow!("API key not set"))
    }
    
    async fn get_provider(&self) -> AIProvider {
        let provider = self.provider.lock().await;
        provider.clone()
    }
    
    pub async fn generate_summary(&self, transcript: &str) -> anyhow::Result<String> {
        let provider = self.get_provider().await;
        
        match provider {
            AIProvider::OpenAI => self.summarize_with_openai(transcript).await,
            AIProvider::Anthropic => self.summarize_with_anthropic(transcript).await,
        }
    }
    
    pub async fn chat(&self, transcript: &str, question: &str) -> anyhow::Result<String> {
        let provider = self.get_provider().await;
        
        match provider {
            AIProvider::OpenAI => self.chat_with_openai(transcript, question).await,
            AIProvider::Anthropic => self.chat_with_anthropic(transcript, question).await,
        }
    }
    
    async fn summarize_with_openai(&self, transcript: &str) -> anyhow::Result<String> {
        let api_key = self.get_api_key().await?;
        
        let prompt = format!(
            "Please provide a concise summary of the following meeting transcript. \
            Include: 1) Key discussion points, 2) Decisions made, 3) Action items, \
            4) Any important deadlines or follow-ups.\n\nTranscript:\n{}",
            transcript
        );
        
        #[derive(Serialize)]
        struct Message {
            role: String,
            content: String,
        }
        
        #[derive(Serialize)]
        struct Request {
            model: String,
            messages: Vec<Message>,
            temperature: f32,
            max_tokens: i32,
        }
        
        #[derive(Deserialize)]
        struct Choice {
            message: MessageResponse,
        }
        
        #[derive(Deserialize)]
        struct MessageResponse {
            content: String,
        }
        
        #[derive(Deserialize)]
        struct Response {
            choices: Vec<Choice>,
        }
        
        let request = Request {
            model: "gpt-4".to_string(),
            messages: vec![
                Message {
                    role: "system".to_string(),
                    content: "You are a helpful assistant that summarizes meeting transcripts.".to_string(),
                },
                Message {
                    role: "user".to_string(),
                    content: prompt,
                },
            ],
            temperature: 0.3,
            max_tokens: 2000,
        };
        
        let response = self.client
            .post("https://api.openai.com/v1/chat/completions")
            .header(header::AUTHORIZATION, format!("Bearer {}", api_key))
            .header(header::CONTENT_TYPE, "application/json")
            .json(&request)
            .send()
            .await?;
        
        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(anyhow::anyhow!("OpenAI API error: {}", error_text));
        }
        
        let result: Response = response.json().await?;
        
        result.choices
            .into_iter()
            .next()
            .map(|c| c.message.content)
            .ok_or_else(|| anyhow::anyhow!("No response from OpenAI"))
    }
    
    async fn summarize_with_anthropic(&self, transcript: &str) -> anyhow::Result<String> {
        let api_key = self.get_api_key().await?;
        
        let prompt = format!(
            "Please provide a concise summary of the following meeting transcript. \
            Include: 1) Key discussion points, 2) Decisions made, 3) Action items, \
            4) Any important deadlines or follow-ups.\n\nTranscript:\n{}",
            transcript
        );
        
        #[derive(Serialize)]
        struct Message {
            role: String,
            content: String,
        }
        
        #[derive(Serialize)]
        struct Request {
            model: String,
            max_tokens: i32,
            messages: Vec<Message>,
        }
        
        #[derive(Deserialize)]
        struct Content {
            text: String,
        }
        
        #[derive(Deserialize)]
        struct Response {
            content: Vec<Content>,
        }
        
        let request = Request {
            model: "claude-3-opus-20240229".to_string(),
            max_tokens: 2000,
            messages: vec![
                Message {
                    role: "user".to_string(),
                    content: prompt,
                },
            ],
        };
        
        let response = self.client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", api_key)
            .header("anthropic-version", "2023-06-01")
            .header(header::CONTENT_TYPE, "application/json")
            .json(&request)
            .send()
            .await?;
        
        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(anyhow::anyhow!("Anthropic API error: {}", error_text));
        }
        
        let result: Response = response.json().await?;
        
        result.content
            .into_iter()
            .next()
            .map(|c| c.text)
            .ok_or_else(|| anyhow::anyhow!("No response from Anthropic"))
    }
    
    async fn chat_with_openai(&self, transcript: &str, question: &str) -> anyhow::Result<String> {
        let api_key = self.get_api_key().await?;
        
        let prompt = format!(
            "Based on the following meeting transcript, please answer this question: {}\n\nTranscript:\n{}",
            question, transcript
        );
        
        #[derive(Serialize)]
        struct Message {
            role: String,
            content: String,
        }
        
        #[derive(Serialize)]
        struct Request {
            model: String,
            messages: Vec<Message>,
            temperature: f32,
            max_tokens: i32,
        }
        
        #[derive(Deserialize)]
        struct Choice {
            message: MessageResponse,
        }
        
        #[derive(Deserialize)]
        struct MessageResponse {
            content: String,
        }
        
        #[derive(Deserialize)]
        struct Response {
            choices: Vec<Choice>,
        }
        
        let request = Request {
            model: "gpt-4".to_string(),
            messages: vec![
                Message {
                    role: "system".to_string(),
                    content: "You are a helpful assistant that answers questions about meeting transcripts. Be concise and reference specific parts of the transcript when possible.".to_string(),
                },
                Message {
                    role: "user".to_string(),
                    content: prompt,
                },
            ],
            temperature: 0.3,
            max_tokens: 1500,
        };
        
        let response = self.client
            .post("https://api.openai.com/v1/chat/completions")
            .header(header::AUTHORIZATION, format!("Bearer {}", api_key))
            .header(header::CONTENT_TYPE, "application/json")
            .json(&request)
            .send()
            .await?;
        
        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(anyhow::anyhow!("OpenAI API error: {}", error_text));
        }
        
        let result: Response = response.json().await?;
        
        result.choices
            .into_iter()
            .next()
            .map(|c| c.message.content)
            .ok_or_else(|| anyhow::anyhow!("No response from OpenAI"))
    }
    
    async fn chat_with_anthropic(&self, transcript: &str, question: &str) -> anyhow::Result<String> {
        let api_key = self.get_api_key().await?;
        
        let prompt = format!(
            "Based on the following meeting transcript, please answer this question: {}\n\nTranscript:\n{}",
            question, transcript
        );
        
        #[derive(Serialize)]
        struct Message {
            role: String,
            content: String,
        }
        
        #[derive(Serialize)]
        struct Request {
            model: String,
            max_tokens: i32,
            messages: Vec<Message>,
        }
        
        #[derive(Deserialize)]
        struct Content {
            text: String,
        }
        
        #[derive(Deserialize)]
        struct Response {
            content: Vec<Content>,
        }
        
        let request = Request {
            model: "claude-3-opus-20240229".to_string(),
            max_tokens: 1500,
            messages: vec![
                Message {
                    role: "user".to_string(),
                    content: prompt,
                },
            ],
        };
        
        let response = self.client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", api_key)
            .header("anthropic-version", "2023-06-01")
            .header(header::CONTENT_TYPE, "application/json")
            .json(&request)
            .send()
            .await?;
        
        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(anyhow::anyhow!("Anthropic API error: {}", error_text));
        }
        
        let result: Response = response.json().await?;
        
        result.content
            .into_iter()
            .next()
            .map(|c| c.text)
            .ok_or_else(|| anyhow::anyhow!("No response from Anthropic"))
    }
    
    pub async fn extract_action_items(&self, transcript: &str) -> anyhow::Result<Vec<String>> {
        let provider = self.get_provider().await;
        
        let prompt = format!(
            "Extract all action items from the following meeting transcript. \
            Return them as a simple JSON array of strings. Each action item should include \
            the task and who is responsible (if mentioned).\n\nTranscript:\n{}",
            transcript
        );
        
        let response = match provider {
            AIProvider::OpenAI => {
                self.send_openai_request(&prompt, "gpt-3.5-turbo", 1000).await?
            }
            AIProvider::Anthropic => {
                self.send_anthropic_request(&prompt, "claude-3-haiku-20240307", 1000).await?
            }
        };
        
        // Try to parse as JSON array
        match serde_json::from_str::<Vec<String>>(&response) {
            Ok(items) => Ok(items),
            Err(_) => {
                // Fallback: split by newlines and clean up
                let items: Vec<String> = response
                    .lines()
                    .map(|line| line.trim().trim_start_matches('-').trim().to_string())
                    .filter(|line| !line.is_empty())
                    .collect();
                Ok(items)
            }
        }
    }
    
    async fn send_openai_request(&self, prompt: &str, model: &str, max_tokens: i32) -> anyhow::Result<String> {
        let api_key = self.get_api_key().await?;
        
        #[derive(Serialize)]
        struct Message {
            role: String,
            content: String,
        }
        
        #[derive(Serialize)]
        struct Request {
            model: String,
            messages: Vec<Message>,
            temperature: f32,
            max_tokens: i32,
        }
        
        #[derive(Deserialize)]
        struct Choice {
            message: MessageResponse,
        }
        
        #[derive(Deserialize)]
        struct MessageResponse {
            content: String,
        }
        
        #[derive(Deserialize)]
        struct Response {
            choices: Vec<Choice>,
        }
        
        let request = Request {
            model: model.to_string(),
            messages: vec![
                Message {
                    role: "user".to_string(),
                    content: prompt.to_string(),
                },
            ],
            temperature: 0.2,
            max_tokens,
        };
        
        let response = self.client
            .post("https://api.openai.com/v1/chat/completions")
            .header(header::AUTHORIZATION, format!("Bearer {}", api_key))
            .header(header::CONTENT_TYPE, "application/json")
            .json(&request)
            .send()
            .await?;
        
        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(anyhow::anyhow!("OpenAI API error: {}", error_text));
        }
        
        let result: Response = response.json().await?;
        
        result.choices
            .into_iter()
            .next()
            .map(|c| c.message.content)
            .ok_or_else(|| anyhow::anyhow!("No response from OpenAI"))
    }
    
    async fn send_anthropic_request(&self, prompt: &str, model: &str, max_tokens: i32) -> anyhow::Result<String> {
        let api_key = self.get_api_key().await?;
        
        #[derive(Serialize)]
        struct Message {
            role: String,
            content: String,
        }
        
        #[derive(Serialize)]
        struct Request {
            model: String,
            max_tokens: i32,
            messages: Vec<Message>,
        }
        
        #[derive(Deserialize)]
        struct Content {
            text: String,
        }
        
        #[derive(Deserialize)]
        struct Response {
            content: Vec<Content>,
        }
        
        let request = Request {
            model: model.to_string(),
            max_tokens,
            messages: vec![
                Message {
                    role: "user".to_string(),
                    content: prompt.to_string(),
                },
            ],
        };
        
        let response = self.client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", api_key)
            .header("anthropic-version", "2023-06-01")
            .header(header::CONTENT_TYPE, "application/json")
            .json(&request)
            .send()
            .await?;
        
        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(anyhow::anyhow!("Anthropic API error: {}", error_text));
        }
        
        let result: Response = response.json().await?;
        
        result.content
            .into_iter()
            .next()
            .map(|c| c.text)
            .ok_or_else(|| anyhow::anyhow!("No response from Anthropic"))
    }
}
