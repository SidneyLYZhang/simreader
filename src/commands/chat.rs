use std::io::{self, Write};

use crate::commands::summary::create_llm_provider;
use crate::commands::util::input_to_file_format;
use crate::config::ConfigManager;
use crate::input::{DataFormat, InputConfig};

pub async fn chat_command(
    input: &InputConfig,
    question: Option<&str>,
) -> anyhow::Result<()> {
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

    let file_content = load_file_content(input);

    if let Some(q) = question {
        let answer = ask_llm(&*provider, &mgr, &file_content, q).await?;
        println!("{}", answer);
    } else {
        interactive_chat(&*provider, &mgr, &file_content).await?;
    }

    Ok(())
}

fn load_file_content(input: &InputConfig) -> String {
    let content = match input.format() {
        DataFormat::Text => match input.read_to_string() {
            Ok(s) => s,
            Err(e) => {
                eprintln!("警告: 读取输入失败: {}", e);
                return String::new();
            }
        },
        DataFormat::Csv { .. } => {
            if let Some(path) = input.file_path() {
                match load_data_file_preview(path.to_str().unwrap(), input) {
                    Ok(s) => s,
                    Err(e) => {
                        eprintln!("警告: 读取数据文件失败: {}", e);
                        String::new()
                    }
                }
            } else {
                match input.read_to_string() {
                    Ok(s) => s,
                    Err(e) => {
                        eprintln!("警告: 读取输入失败: {}", e);
                        String::new()
                    }
                }
            }
        }
        _ => {
            if let Some(path) = input.file_path() {
                match load_data_file_preview(path.to_str().unwrap(), input) {
                    Ok(s) => s,
                    Err(e) => {
                        eprintln!("警告: 读取数据文件失败: {}", e);
                        String::new()
                    }
                }
            } else {
                eprintln!("警告: 该格式不支持从标准输入读取");
                String::new()
            }
        }
    };

    content.chars().take(50000).collect()
}

fn load_data_file_preview(file_path: &str, input: &InputConfig) -> anyhow::Result<String> {
    let (format, sep) = input_to_file_format(input);

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
