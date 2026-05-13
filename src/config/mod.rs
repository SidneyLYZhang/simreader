use std::path::PathBuf;

use anyhow::Context;
use serde::{Deserialize, Serialize};

const APP_NAME: &str = "simreader";
const CONFIG_FILE: &str = "config.toml";
const KEYRING_USER_PREFIX: &str = "llm";

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
pub struct ThinkingSettings {
    #[serde(default = "default_thinking_enabled")]
    pub enabled: bool,
    #[serde(default = "default_thinking_effort", skip_serializing_if = "Option::is_none")]
    pub effort: Option<ReasoningEffort>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    #[serde(default)]
    pub exclude: bool,
}

fn default_thinking_enabled() -> bool {
    true
}

fn default_thinking_effort() -> Option<ReasoningEffort> {
    Some(ReasoningEffort::Max)
}

impl Default for ThinkingSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            effort: Some(ReasoningEffort::Max),
            max_tokens: None,
            exclude: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmSettings {
    #[serde(default = "default_provider")]
    pub provider: String,
    #[serde(default = "default_model")]
    pub model: String,
    #[serde(default = "default_base_url")]
    pub base_url: String,
    #[serde(default)]
    pub thinking: ThinkingSettings,
}

fn default_provider() -> String {
    "deepseek".into()
}

fn default_model() -> String {
    "deepseek-v3-flash".into()
}

fn default_base_url() -> String {
    "https://api.deepseek.com/".into()
}

impl Default for LlmSettings {
    fn default() -> Self {
        Self {
            provider: "deepseek".into(),
            model: "deepseek-v4-flash".into(),
            base_url: "https://api.deepseek.com/".into(),
            thinking: ThinkingSettings::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisplaySettings {
    #[serde(default = "default_line_width")]
    pub line_width: usize,
    #[serde(default = "default_output_language")]
    pub output_language: String,
}

fn default_line_width() -> usize {
    80
}

fn default_output_language() -> String {
    "中文".into()
}

impl Default for DisplaySettings {
    fn default() -> Self {
        Self {
            line_width: 80,
            output_language: "中文".into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    #[serde(default)]
    pub llm: LlmSettings,
    #[serde(default)]
    pub display: DisplaySettings,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            llm: LlmSettings::default(),
            display: DisplaySettings::default(),
        }
    }
}

pub struct ConfigManager {
    config_path: PathBuf,
    config: AppConfig,
}

impl ConfigManager {
    pub fn new() -> anyhow::Result<Self> {
        let config_path = Self::default_config_path()?;
        let config = if config_path.exists() {
            let content = std::fs::read_to_string(&config_path)
                .with_context(|| format!("无法读取配置文件: {}", config_path.display()))?;
            toml::from_str(&content).with_context(|| "配置文件格式错误")?
        } else {
            let default_config = AppConfig::default();
            let content = toml::to_string_pretty(&default_config)
                .context("默认配置序列化失败")?;
            if let Some(parent) = config_path.parent() {
                std::fs::create_dir_all(parent)
                    .with_context(|| format!("无法创建配置目录: {}", parent.display()))?;
            }
            std::fs::write(&config_path, &content)
                .with_context(|| format!("无法写入配置文件: {}", config_path.display()))?;
            default_config
        };

        Ok(Self {
            config_path,
            config,
        })
    }

    pub fn with_config_path(path: impl Into<PathBuf>) -> anyhow::Result<Self> {
        let config_path = path.into();
        let config = if config_path.exists() {
            let content = std::fs::read_to_string(&config_path)
                .with_context(|| format!("无法读取配置文件: {}", config_path.display()))?;
            toml::from_str(&content).with_context(|| "配置文件格式错误")?
        } else {
            AppConfig::default()
        };

        Ok(Self {
            config_path,
            config,
        })
    }

    pub fn config(&self) -> &AppConfig {
        &self.config
    }

    pub fn llm(&self) -> &LlmSettings {
        &self.config.llm
    }

    pub fn config_path(&self) -> &PathBuf {
        &self.config_path
    }

    fn default_config_path() -> anyhow::Result<PathBuf> {
        let base = dirs::config_dir()
            .context("无法确定系统配置目录")?
            .join(APP_NAME);
        Ok(base.join(CONFIG_FILE))
    }

    pub fn save(&self) -> anyhow::Result<()> {
        if let Some(parent) = self.config_path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("无法创建配置目录: {}", parent.display()))?;
        }
        let content =
            toml::to_string_pretty(&self.config).context("配置文件序列化失败")?;
        std::fs::write(&self.config_path, &content)
            .with_context(|| format!("无法写入配置文件: {}", self.config_path.display()))?;
        Ok(())
    }

    pub fn save_to(&self, path: impl Into<PathBuf>) -> anyhow::Result<()> {
        let path = path.into();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content =
            toml::to_string_pretty(&self.config).context("配置文件序列化失败")?;
        std::fs::write(&path, &content)
            .with_context(|| format!("无法写入配置文件: {}", path.display()))?;
        Ok(())
    }

    pub fn set_llm_provider(&mut self, provider: impl Into<String>) -> &mut Self {
        self.config.llm.provider = provider.into();
        self
    }

    pub fn set_llm_model(&mut self, model: impl Into<String>) -> &mut Self {
        self.config.llm.model = model.into();
        self
    }

    pub fn set_llm_base_url(&mut self, base_url: impl Into<String>) -> &mut Self {
        self.config.llm.base_url = base_url.into();
        self
    }

    pub fn set_thinking_enabled(&mut self, enabled: bool) -> &mut Self {
        self.config.llm.thinking.enabled = enabled;
        self
    }

    pub fn set_thinking_effort(&mut self, effort: ReasoningEffort) -> &mut Self {
        self.config.llm.thinking.effort = Some(effort);
        self
    }

    pub fn set_thinking_max_tokens(&mut self, max_tokens: Option<u32>) -> &mut Self {
        self.config.llm.thinking.max_tokens = max_tokens;
        self
    }

    fn keyring_entry(provider: &str) -> anyhow::Result<keyring::Entry> {
        keyring::Entry::new(APP_NAME, &format!("{}-{}", KEYRING_USER_PREFIX, provider))
            .context("无法创建密钥环条目")
    }

    pub fn set_api_key(&self, provider: &str, api_key: &str) -> anyhow::Result<()> {
        let entry = Self::keyring_entry(provider)?;
        entry
            .set_password(api_key)
            .with_context(|| format!("无法保存 {} 的 API Key 到系统密钥环", provider))
    }

    pub fn get_api_key(&self, provider: &str) -> anyhow::Result<String> {
        let entry = Self::keyring_entry(provider)?;
        entry
            .get_password()
            .with_context(|| format!("未找到 {} 的 API Key，请先通过 'simreader config set-key' 配置", provider))
    }

    pub fn delete_api_key(&self, provider: &str) -> anyhow::Result<()> {
        let entry = Self::keyring_entry(provider)?;
        entry
            .delete_credential()
            .with_context(|| format!("无法删除 {} 的 API Key", provider))
    }

    pub fn set_api_key_for_current_provider(&self, api_key: &str) -> anyhow::Result<()> {
        self.set_api_key(&self.config.llm.provider, api_key)
    }

    pub fn get_api_key_for_current_provider(&self) -> anyhow::Result<String> {
        self.get_api_key(&self.config.llm.provider)
    }

    pub fn delete_api_key_for_current_provider(&self) -> anyhow::Result<()> {
        self.delete_api_key(&self.config.llm.provider)
    }

    pub fn line_width(&self) -> usize {
        self.config.display.line_width
    }

    pub fn set_line_width(&mut self, width: usize) -> &mut Self {
        self.config.display.line_width = width;
        self
    }

    pub fn output_language(&self) -> &str {
        &self.config.display.output_language
    }

    pub fn set_output_language(&mut self, lang: impl Into<String>) -> &mut Self {
        self.config.display.output_language = lang.into();
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = AppConfig::default();
        assert_eq!(config.llm.provider, "deepseek");
        assert_eq!(config.llm.model, "deepseek-v3-flash");
        assert_eq!(config.llm.base_url, "https://api.deepseek.com/");
        assert!(config.llm.thinking.enabled);
        assert_eq!(config.llm.thinking.effort, Some(ReasoningEffort::Max));
    }

    #[test]
    fn test_reasoning_effort_serialization() {
        let effort = ReasoningEffort::High;
        let json = serde_json::to_string(&effort).unwrap();
        assert_eq!(json, r#""high""#);

        let effort = ReasoningEffort::XHigh;
        let json = serde_json::to_string(&effort).unwrap();
        assert_eq!(json, r#""xhigh""#);
    }

    #[test]
    fn test_reasoning_effort_as_high_or_max() {
        assert_eq!(ReasoningEffort::Low.as_high_or_max(), "high");
        assert_eq!(ReasoningEffort::Medium.as_high_or_max(), "high");
        assert_eq!(ReasoningEffort::High.as_high_or_max(), "high");
        assert_eq!(ReasoningEffort::XHigh.as_high_or_max(), "max");
        assert_eq!(ReasoningEffort::Max.as_high_or_max(), "max");
    }

    #[test]
    fn test_toml_roundtrip() {
        let mut config = AppConfig::default();
        config.llm.provider = "openrouter".into();
        config.llm.model = "moonshotai/kimi-k2.6".into();
        config.llm.base_url = "https://openrouter.ai/api/v1".into();
        config.llm.thinking.effort = Some(ReasoningEffort::Max);

        let toml_str = toml::to_string_pretty(&config).unwrap();
        let parsed: AppConfig = toml::from_str(&toml_str).unwrap();

        assert_eq!(parsed.llm.provider, config.llm.provider);
        assert_eq!(parsed.llm.model, config.llm.model);
        assert_eq!(parsed.llm.base_url, config.llm.base_url);
        assert_eq!(parsed.llm.thinking.effort, config.llm.thinking.effort);
    }

    #[test]
    fn test_parse_minimal_config() {
        let toml_str = r#"
[llm]
provider = "deepseek"
model = "deepseek-v4-flash"
"#;
        let config: AppConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.llm.provider, "deepseek");
        assert_eq!(config.llm.model, "deepseek-v4-flash");
        assert!(config.llm.thinking.enabled);
    }

    #[test]
    fn test_parse_full_config() {
        let toml_str = r#"
[llm]
provider = "openrouter"
model = "moonshotai/kimi-k2.6"
base_url = "https://openrouter.ai/api/v1"

[llm.thinking]
enabled = true
effort = "xhigh"
max_tokens = 4096
exclude = false
"#;
        let config: AppConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.llm.provider, "openrouter");
        assert_eq!(config.llm.model, "moonshotai/kimi-k2.6");
        assert_eq!(config.llm.base_url, "https://openrouter.ai/api/v1");
        assert!(config.llm.thinking.enabled);
        assert_eq!(config.llm.thinking.effort, Some(ReasoningEffort::XHigh));
        assert_eq!(config.llm.thinking.max_tokens, Some(4096));
        assert!(!config.llm.thinking.exclude);
    }

    #[test]
    fn test_thinking_settings_default() {
        let settings = ThinkingSettings::default();
        assert!(settings.enabled);
        assert_eq!(settings.effort, Some(ReasoningEffort::Max));
        assert_eq!(settings.max_tokens, None);
        assert!(!settings.exclude);
    }
}
