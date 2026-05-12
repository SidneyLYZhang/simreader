use crate::config::{ConfigManager, ReasoningEffort};

pub fn config_show() -> anyhow::Result<()> {
    let mgr = ConfigManager::new()?;
    let cfg = mgr.config();
    println!("=== SimReader 配置 ===");
    println!("配置路径: {}", mgr.config_path().display());
    println!();
    println!("[llm]");
    println!("  provider      = \"{}\"", cfg.llm.provider);
    println!("  model         = \"{}\"", cfg.llm.model);
    println!("  base_url      = \"{}\"", cfg.llm.base_url);
    println!("  api_key       = **** (存储在系统密钥环中)");
    println!();
    println!("[llm.thinking]");
    println!("  enabled       = {}", cfg.llm.thinking.enabled);
    if let Some(ref effort) = cfg.llm.thinking.effort {
        println!("  effort        = {:?}", effort);
    }
    if let Some(max_tokens) = cfg.llm.thinking.max_tokens {
        println!("  max_tokens    = {}", max_tokens);
    }
    println!("  exclude       = {}", cfg.llm.thinking.exclude);
    println!();
    println!("[display]");
    println!("  line_width      = {}", cfg.display.line_width);
    println!("  output_language = \"{}\"", cfg.display.output_language);
    Ok(())
}

pub fn config_set_provider(provider: &str) -> anyhow::Result<()> {
    let mut mgr = ConfigManager::new()?;
    mgr.set_llm_provider(provider);
    mgr.save()?;
    println!("LLM 供应商已设置为: {}", provider);
    Ok(())
}

pub fn config_set_model(model: &str) -> anyhow::Result<()> {
    let mut mgr = ConfigManager::new()?;
    mgr.set_llm_model(model);
    mgr.save()?;
    println!("模型已设置为: {}", model);
    Ok(())
}

pub fn config_set_base_url(base_url: &str) -> anyhow::Result<()> {
    let mut mgr = ConfigManager::new()?;
    mgr.set_llm_base_url(base_url);
    mgr.save()?;
    println!("Base URL 已设置为: {}", base_url);
    Ok(())
}

pub fn config_set_api_key(api_key: &str) -> anyhow::Result<()> {
    let mgr = ConfigManager::new()?;
    mgr.set_api_key_for_current_provider(api_key)?;
    println!("API Key 已保存至系统密钥环");
    Ok(())
}

pub fn config_set_think(enabled: bool) -> anyhow::Result<()> {
    let mut mgr = ConfigManager::new()?;
    mgr.set_thinking_enabled(enabled);
    mgr.save()?;
    println!("思考模式已{}", if enabled { "开启" } else { "关闭" });
    Ok(())
}

pub fn config_set_think_intensity(intensity: &str) -> anyhow::Result<()> {
    let effort = match intensity.to_lowercase().as_str() {
        "low" => ReasoningEffort::Low,
        "medium" => ReasoningEffort::Medium,
        "high" => ReasoningEffort::High,
        "xhigh" => ReasoningEffort::XHigh,
        "max" => ReasoningEffort::Max,
        _ => anyhow::bail!("无效的思考强度: {}，可选值: low, medium, high, max", intensity),
    };
    let mut mgr = ConfigManager::new()?;
    mgr.set_thinking_effort(effort);
    mgr.save()?;
    println!("思考强度已设置为: {}", intensity.to_lowercase());
    Ok(())
}

pub fn config_set_line_width(width: usize) -> anyhow::Result<()> {
    if width == 0 {
        anyhow::bail!("行宽必须大于 0");
    }
    let mut mgr = ConfigManager::new()?;
    mgr.set_line_width(width);
    mgr.save()?;
    println!("行宽已设置为: {}", width);
    Ok(())
}

pub fn config_set_language(lang: &str) -> anyhow::Result<()> {
    let mut mgr = ConfigManager::new()?;
    mgr.set_output_language(lang);
    mgr.save()?;
    println!("LLM 输出语言已设置为: {}", lang);
    Ok(())
}
