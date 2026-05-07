use std::pin::Pin;
use std::time::Duration;

use async_trait::async_trait;
use futures::{Stream, StreamExt};
use serde_json::Value;

use super::{
    ApiChatMessage, ChatMessage, ChatRequest, ChatResponse, ChatStream, LlmProvider, StreamChunk,
    ThinkingConfig, Usage,
};

const DEEPSEEK_API_BASE: &str = "https://api.deepseek.com/";
const DEFAULT_MODEL: &str = "deepseek-v4-flash";
const DEFAULT_TIMEOUT_SECS: u64 = 120;

pub struct DeepSeekProvider {
    client: reqwest::Client,
    api_key: String,
    base_url: String,
    default_model: String,
}

impl DeepSeekProvider {
    pub fn new(api_key: impl Into<String>) -> anyhow::Result<Self> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(DEFAULT_TIMEOUT_SECS))
            .build()?;

        Ok(Self {
            client,
            api_key: api_key.into(),
            base_url: DEEPSEEK_API_BASE.to_string(),
            default_model: DEFAULT_MODEL.to_string(),
        })
    }

    pub fn with_timeout(mut self, timeout_secs: u64) -> anyhow::Result<Self> {
        self.client = reqwest::Client::builder()
            .timeout(Duration::from_secs(timeout_secs))
            .build()?;
        Ok(self)
    }

    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.default_model = model.into();
        self
    }

    pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = base_url.into();
        self
    }

    fn build_request_body(&self, request: &ChatRequest) -> Value {
        let messages: Vec<ApiChatMessage> = request.messages.iter().map(ApiChatMessage::from).collect();

        let mut body = serde_json::json!({
            "model": request.model,
            "messages": messages,
            "stream": false,
        });

        if let Some(temp) = request.temperature {
            body["temperature"] = Value::from(temp);
        }
        if let Some(max_tok) = request.max_tokens {
            body["max_tokens"] = Value::from(max_tok);
        }
        if let Some(top_p) = request.top_p {
            body["top_p"] = Value::from(top_p);
        }

        if let Some(ref thinking) = request.thinking {
            self.apply_thinking_config(&mut body, thinking);
        } else {
            body["thinking"] = serde_json::json!({"type": "enabled"});
            body["reasoning_effort"] = serde_json::json!("high");
        }

        body
    }

    fn apply_thinking_config(&self, body: &mut Value, thinking: &ThinkingConfig) {
        let thinking_type = if thinking.enabled { "enabled" } else { "disabled" };
        body["thinking"] = serde_json::json!({"type": thinking_type});

        if thinking.enabled {
            if let Some(effort) = &thinking.effort {
                body["reasoning_effort"] = serde_json::json!(effort.as_high_or_max());
            } else {
                body["reasoning_effort"] = serde_json::json!("high");
            }
        }
    }

    fn build_stream_body(&self, request: &ChatRequest) -> Value {
        let mut body = self.build_request_body(request);
        body["stream"] = Value::from(true);
        body
    }

    async fn parse_non_stream_response(&self, response: reqwest::Response) -> anyhow::Result<ChatResponse> {
        let json: Value = response.json().await?;

        let content = json["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("")
            .to_string();

        let model = json["model"].as_str().unwrap_or("unknown").to_string();

        let usage = if let Some(usage_obj) = json.get("usage") {
            Some(Usage {
                prompt_tokens: usage_obj["prompt_tokens"].as_u64().unwrap_or(0) as u32,
                completion_tokens: usage_obj["completion_tokens"].as_u64().unwrap_or(0) as u32,
                total_tokens: usage_obj["total_tokens"].as_u64().unwrap_or(0) as u32,
            })
        } else {
            None
        };

        Ok(ChatResponse {
            content,
            model,
            usage,
        })
    }

    fn process_sse_line(&self, line: &str) -> Option<StreamChunk> {
        let data = line.strip_prefix("data: ")?;

        if data == "[DONE]" {
            return Some(StreamChunk {
                content: String::new(),
                done: true,
            });
        }

        let json: Value = serde_json::from_str(data).ok()?;
        let delta = &json["choices"][0]["delta"];

        let content = delta["content"].as_str().unwrap_or("").to_string();

        if content.is_empty() {
            return None;
        }

        Some(StreamChunk {
            content,
            done: false,
        })
    }
}

#[async_trait]
impl LlmProvider for DeepSeekProvider {
    async fn chat(&self, request: ChatRequest) -> anyhow::Result<ChatResponse> {
        let messages = self.build_messages_with_files(request.messages, &request.files)?;

        let mut req = ChatRequest {
            messages,
            ..request
        };

        if req.model.is_empty() {
            req.model = self.default_model.clone();
        }

        let body = self.build_request_body(&req);
        let url = format!("{}/chat/completions", self.base_url);

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!(
                "DeepSeek API 返回错误 ({}): {}",
                status,
                error_text
            ));
        }

        self.parse_non_stream_response(response).await
    }

    async fn chat_stream(&self, request: ChatRequest) -> anyhow::Result<ChatStream> {
        let messages = self.build_messages_with_files(request.messages, &request.files)?;

        let mut req = ChatRequest {
            messages,
            ..request
        };

        if req.model.is_empty() {
            req.model = self.default_model.clone();
        }

        let body = self.build_stream_body(&req);
        let url = format!("{}/chat/completions", self.base_url);

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!(
                "DeepSeek API 返回错误 ({}): {}",
                status,
                error_text
            ));
        }

        let byte_stream = response.bytes_stream();
        let stream = byte_stream.filter_map(move |chunk_result| {
            let chunk = match chunk_result {
                Ok(c) => c,
                Err(e) => {
                    return futures::future::ready(Some(Err(anyhow::anyhow!(
                        "流读取错误: {}",
                        e
                    ))));
                }
            };

            let text = String::from_utf8_lossy(&chunk).to_string();
            let mut results: Vec<StreamChunk> = Vec::new();

            for line in text.lines() {
                let line = line.trim().to_string();
                if line.is_empty() {
                    continue;
                }
                if let Some(chunk_data) = self.process_sse_line(&line) {
                    results.push(chunk_data);
                }
            }

            if results.is_empty() {
                futures::future::ready(None)
            } else {
                let stream_items: Vec<super::StreamResult> =
                    results.into_iter().map(Ok).collect();
                futures::future::ready(Some(futures::stream::iter(stream_items)))
            }
        })
        .flatten();

        Ok(Box::pin(stream))
    }

    fn default_model(&self) -> &str {
        &self.default_model
    }

    fn provider_name(&self) -> &str {
        "DeepSeek"
    }
}
