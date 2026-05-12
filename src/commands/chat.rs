use std::io::{self, Write};
use std::path::Path;

use crate::commands::summary::{create_llm_provider};
use crate::config::ConfigManager;

pub async fn chat_file(file_path: &str, question: Option<&str>) -> anyhow::Result<()> {
    let mgr = ConfigManager::new()?;
    let cfg = mgr.config();

    if cfg.llm.provider.is_empty() {
        println!("未配置 LLM。请先运行 'sr config' 进行配置。");
        return Ok(());
    }

    let api_key = match mgr.get_api_key_for_current_provider() {
        Ok(key) => key,
        Err(_) => {
            println!("未配置 API Key。请先运行 'sr config set-key <your-key>' 进行配置。");
            return Ok(());
        }
    };

    let provider = match create_llm_provider(&mgr, &api_key) {
        Ok(p) => p,
        Err(e) => {
            anyhow::bail!("创建 LLM 提供商失败: {}", e);
        }
    };

    let file_content = load_file_content(file_path);

    if let Some(q) = question {
        let answer = ask_llm(&*provider, &mgr, &file_content, q).await?;
        println!("{}", answer);
    } else {
        interactive_chat(&*provider, &mgr, &file_content).await?;
    }

    Ok(())
}

fn load_file_content(file_path: &str) -> String {
    let path = Path::new(file_path);
    let content = if util_is_data_file(file_path) {
        match load_data_file_preview(file_path) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("警告: 读取数据文件失败: {}", e);
                String::new()
            }
        }
    } else {
        match std::fs::read_to_string(path) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("警告: 读取文件失败: {}", e);
                String::new()
            }
        }
    };

    content.chars().take(50000).collect()
}

fn util_is_data_file(file_path: &str) -> bool {
    crate::commands::util::is_data_file(file_path)
}

fn load_data_file_preview(file_path: &str) -> anyhow::Result<String> {
    use crate::commands::util;

    let format = util::detect_file_format(file_path)
        .ok_or_else(|| anyhow::anyhow!("不支持的文件格式"))?;
    let sep = util::csv_separator_for_file(file_path);

    let lf = crate::reader::readdata::read_to_lazyframe(file_path, format, sep, None)?;
    let df = lf.limit(100).collect()?;

    let headers: Vec<String> = df.get_column_names().iter().map(|s| s.to_string()).collect();
    let mut out = String::new();
    out.push_str(&headers.join("\t"));
    out.push('\n');

    for row_idx in 0..df.height() {
        let row = df.get_row(row_idx)?;
        let cells: Vec<String> = row.0.iter().map(|v| v.to_string()).collect();
        out.push_str(&cells.join("\t"));
        out.push('\n');
    }

    Ok(out)
}

async fn ask_llm(
    provider: &dyn crate::llm::LlmProvider,
    mgr: &ConfigManager,
    file_content: &str,
    question: &str,
) -> anyhow::Result<String> {
    let lang = mgr.output_language();

    let request = crate::llm::ChatRequest {
        model: mgr.config().llm.model.clone(),
        messages: vec![
            crate::llm::ChatMessage::system(&format!(
                "你是一个数据分析助手。请基于以下文件内容回答用户的问题。请用{}回答。\n\n文件内容:\n{}",
                lang, file_content
            )),
            crate::llm::ChatMessage::user(question),
        ],
        temperature: Some(0.3),
        max_tokens: Some(2000),
        top_p: None,
        thinking: Some(crate::llm::ThinkingConfig::from(mgr.config().llm.thinking.clone())),
        files: vec![],
    };

    let response = provider.chat(request).await?;
    Ok(response.content)
}

async fn interactive_chat(
    provider: &dyn crate::llm::LlmProvider,
    mgr: &ConfigManager,
    file_content: &str,
) -> anyhow::Result<()> {
    println!("进入交互式问答模式 (输入 /exit 退出)");
    println!();

    let lang = mgr.output_language();

    let mut conversation: Vec<crate::llm::ChatMessage> = vec![
        crate::llm::ChatMessage::system(&format!(
            "你是一个数据分析助手。请基于以下文件内容回答用户的问题。请用{}回答。\n\n文件内容:\n{}",
            lang, file_content
        )),
    ];

    loop {
        print!("> ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim().to_string();

        if input.is_empty() {
            continue;
        }

        if input == "/exit" || input == "/quit" || input == "/q" {
            println!("退出交互模式。");
            break;
        }

        conversation.push(crate::llm::ChatMessage::user(&input));

        let request = crate::llm::ChatRequest {
            model: mgr.config().llm.model.clone(),
            messages: conversation.clone(),
            temperature: Some(0.3),
            max_tokens: Some(2000),
            top_p: None,
            thinking: Some(crate::llm::ThinkingConfig::from(mgr.config().llm.thinking.clone())),
            files: vec![],
        };

        match provider.chat(request).await {
            Ok(response) => {
                println!();
                println!("{}", response.content);
                println!();
                conversation.push(crate::llm::ChatMessage::assistant(&response.content));
            }
            Err(e) => {
                eprintln!("LLM 调用失败: {}", e);
                conversation.pop();
            }
        }
    }

    Ok(())
}
