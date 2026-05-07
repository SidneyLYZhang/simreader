pub mod deepseek;
pub mod openrouter;

use std::path::PathBuf;
use std::pin::Pin;

use async_trait::async_trait;
use futures::Stream;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

impl ChatMessage {
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: "system".into(),
            content: content.into(),
        }
    }

    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: "user".into(),
            content: content.into(),
        }
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: "assistant".into(),
            content: content.into(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ReasoningEffort {
    Low,
    Medium,
    High,
    XHigh,
    Max,
}

impl ReasoningEffort {
    pub fn as_high_or_max(&self) -> &str {
        match self {
            ReasoningEffort::Low | ReasoningEffort::Medium | ReasoningEffort::High => "high",
            ReasoningEffort::XHigh | ReasoningEffort::Max => "max",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThinkingConfig {
    pub enabled: bool,
    pub effort: Option<ReasoningEffort>,
    pub max_tokens: Option<u32>,
    pub exclude: bool,
}

impl Default for ThinkingConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            effort: Some(ReasoningEffort::High),
            max_tokens: None,
            exclude: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    #[serde(skip)]
    pub thinking: Option<ThinkingConfig>,
    #[serde(skip)]
    pub files: Vec<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Usage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatResponse {
    pub content: String,
    pub model: String,
    pub usage: Option<Usage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamChunk {
    pub content: String,
    pub done: bool,
}

pub type StreamResult = anyhow::Result<StreamChunk>;
pub type ChatStream = Pin<Box<dyn Stream<Item = StreamResult> + Send>>;

#[async_trait]
pub trait LlmProvider: Send + Sync {
    async fn chat(&self, request: ChatRequest) -> anyhow::Result<ChatResponse>;

    async fn chat_stream(&self, request: ChatRequest) -> anyhow::Result<ChatStream>;

    fn default_model(&self) -> &str;

    fn provider_name(&self) -> &str;

    fn build_messages_with_files(
        &self,
        messages: Vec<ChatMessage>,
        files: &[PathBuf],
    ) -> anyhow::Result<Vec<ChatMessage>> {
        if files.is_empty() {
            return Ok(messages);
        }

        let mut file_contents = Vec::new();
        for file_path in files {
            let content = std::fs::read_to_string(file_path)
                .map_err(|e| anyhow::anyhow!("无法读取文件 {:?}: {}", file_path, e))?;
            file_contents.push(format!(
                "[文件: {}]\n```\n{}\n```",
                file_path.display(),
                content
            ));
        }

        let file_context = file_contents.join("\n\n");
        let context_message = ChatMessage::system(format!(
            "以下是用户上传的文件内容，请参考这些内容回答用户问题：\n\n{}",
            file_context
        ));

        let mut all_messages = vec![context_message];
        all_messages.extend(messages);
        Ok(all_messages)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ApiChatMessage {
    role: String,
    content: String,
}

impl From<&ChatMessage> for ApiChatMessage {
    fn from(msg: &ChatMessage) -> Self {
        Self {
            role: msg.role.clone(),
            content: msg.content.clone(),
        }
    }
}
