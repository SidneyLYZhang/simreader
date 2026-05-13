#![allow(dead_code)]

mod commands;
mod config;
mod llm;
mod reader;

use clap::{arg, Arg, ArgAction, Command};

const VERSION: &str = env!("CARGO_PKG_VERSION");

fn main() {
    let matches = Command::new("simreader")
        .about("A Simple Reader for data files and txt files")
        .arg_required_else_help(true)
        .subcommand(
            Command::new("config")
                .about("查看或修改配置")
                .arg(
                    Arg::new("provider")
                        .long("provider")
                        .value_name("NAME")
                        .help("LLM 供应商名称"),
                )
                .arg(
                    Arg::new("model")
                        .long("model")
                        .value_name("MODEL")
                        .help("模型名称"),
                )
                .arg(
                    Arg::new("base_url")
                        .long("base-url")
                        .value_name("URL")
                        .help("API 地址"),
                )
                .arg(
                    Arg::new("api_key")
                        .long("api-key")
                        .value_name("KEY")
                        .help("API 访问密钥"),
                )
                .arg(
                    Arg::new("think")
                        .long("think")
                        .action(ArgAction::SetTrue)
                        .help("开启思考模式"),
                )
                .arg(
                    Arg::new("no_think")
                        .long("no-think")
                        .action(ArgAction::SetTrue)
                        .help("关闭思考模式")
                        .conflicts_with("think"),
                )
                .arg(
                    Arg::new("think_intensity")
                        .long("think-intensity")
                        .value_name("INTENSITY")
                        .help("思考强度 (low/medium/high/max)"),
                )
                .arg(
                    Arg::new("line_width")
                        .long("line-width")
                        .value_name("WIDTH")
                        .help("文本行显示宽度（每行字符数）"),
                )
                .arg(
                    Arg::new("language")
                        .long("language")
                        .value_name("LANG")
                        .help("LLM 输出语言"),
                ),
        )
        .subcommand(
            Command::new("head")
                .about("查看文件的前面部分")
                .arg(arg!(<FILE> "文件路径"))
                .arg(
                    Arg::new("num")
                        .short('n')
                        .long("num")
                        .value_name("N")
                        .default_value("5")
                        .help("显示的行数"),
                )
                .arg(
                    Arg::new("no_name")
                        .long("no-name")
                        .action(ArgAction::SetTrue)
                        .help("列名用列索引序号代替"),
                )
                .arg(
                    Arg::new("csv")
                        .long("csv")
                        .action(ArgAction::SetTrue)
                        .help("强制以CSV格式读取数据"),
                )
                .arg(
                    Arg::new("separator")
                        .short('s')
                        .long("separator")
                        .value_name("SEP")
                        .help("CSV分隔符（配合--csv使用，默认为逗号）"),
                )
                .arg(
                    Arg::new("col")
                        .long("col")
                        .value_name("COLS")
                        .help("选择列 (列名, 列号范围如0:5, 或列号列表如2,4,7)"),
                ),
        )
        .subcommand(
            Command::new("tail")
                .about("查看文件的结尾部分")
                .arg(arg!(<FILE> "文件路径"))
                .arg(
                    Arg::new("num")
                        .short('n')
                        .long("num")
                        .value_name("N")
                        .default_value("5")
                        .help("显示的行数"),
                )
                .arg(
                    Arg::new("no_name")
                        .long("no-name")
                        .action(ArgAction::SetTrue)
                        .help("列名用列索引序号代替"),
                )
                .arg(
                    Arg::new("csv")
                        .long("csv")
                        .action(ArgAction::SetTrue)
                        .help("强制以CSV格式读取数据"),
                )
                .arg(
                    Arg::new("separator")
                        .short('s')
                        .long("separator")
                        .value_name("SEP")
                        .help("CSV分隔符（配合--csv使用，默认为逗号）"),
                )
                .arg(
                    Arg::new("col")
                        .long("col")
                        .value_name("COLS")
                        .help("选择列 (列名, 列号范围如0:5, 或列号列表如2,4,7)"),
                ),
        )
        .subcommand(
            Command::new("schema")
                .about("查看文件的模式信息")
                .arg(arg!(<FILE> "文件路径"))
                .arg(
                    Arg::new("direction")
                        .short('d')
                        .long("direction")
                        .value_name("DIR")
                        .default_value("col")
                        .help("统计方向 (col/row)"),
                )
                .arg(
                    Arg::new("no_name")
                        .long("no-name")
                        .action(ArgAction::SetTrue)
                        .help("列名用列索引序号代替"),
                )
                .arg(
                    Arg::new("csv")
                        .long("csv")
                        .action(ArgAction::SetTrue)
                        .help("强制以CSV格式读取数据"),
                )
                .arg(
                    Arg::new("separator")
                        .short('s')
                        .long("separator")
                        .value_name("SEP")
                        .help("CSV分隔符（配合--csv使用，默认为逗号）"),
                )
                .arg(
                    Arg::new("col")
                        .long("col")
                        .value_name("COLS")
                        .help("选择列 (列名, 列号范围如0:5, 或列号列表如2,4,7)"),
                ),
        )
        .subcommand(
            Command::new("summary")
                .about("对文件进行详细总结")
                .arg(arg!(<FILE> "文件路径"))
                .arg(
                    Arg::new("no_name")
                        .long("no-name")
                        .action(ArgAction::SetTrue)
                        .help("列名用列索引序号代替"),
                )
                .arg(
                    Arg::new("csv")
                        .long("csv")
                        .action(ArgAction::SetTrue)
                        .help("强制以CSV格式读取数据"),
                )
                .arg(
                    Arg::new("separator")
                        .short('s')
                        .long("separator")
                        .value_name("SEP")
                        .help("CSV分隔符（配合--csv使用，默认为逗号）"),
                )
                .arg(
                    Arg::new("col")
                        .long("col")
                        .value_name("COLS")
                        .help("选择列 (列名, 列号范围如0:5, 或列号列表如2,4,7)"),
                ),
        )
        .subcommand(
            Command::new("chat")
                .about("利用 LLM 对文件内容进行问答")
                .arg(arg!(<FILE> "文件路径"))
                .arg(arg!([QUESTION] "要询问的问题（可选）")),
        )
        .arg(arg!([file] "文件路径（无子命令模式）"))
        .arg(
            Arg::new("summary_flag")
                .short('s')
                .long("summary")
                .action(ArgAction::SetTrue)
                .help("对文件进行详细总结"),
        )
        .arg(
            Arg::new("head_flag")
                .short('h')
                .long("head")
                .action(ArgAction::SetTrue)
                .help("查看文件的前面部分"),
        )
        .arg(
            Arg::new("tail_flag")
                .short('t')
                .long("tail")
                .action(ArgAction::SetTrue)
                .help("查看文件的结尾部分"),
        )
        .arg(
            Arg::new("schema_flag")
                .short('e')
                .long("schema")
                .action(ArgAction::SetTrue)
                .help("查看文件的模式信息"),
        )
        .arg(
            Arg::new("quest_flag")
                .short('q')
                .long("quest")
                .value_name("QUESTION")
                .num_args(1)
                .help("利用 LLM 对文件内容进行问答"),
        )
        .arg(
            Arg::new("num")
                .short('n')
                .long("num")
                .value_name("N")
                .help("显示的行数 (head/tail)"),
        )
        .arg(
            Arg::new("no_name")
                .long("no-name")
                .action(ArgAction::SetTrue)
                .help("列名用列索引序号代替"),
        )
        .arg(
            Arg::new("name_flag")
                .long("name")
                .action(ArgAction::SetTrue)
                .help("使用第一行作为列名（默认）"),
        )
        .arg(
            Arg::new("csv")
                .long("csv")
                .action(ArgAction::SetTrue)
                .help("强制以CSV格式读取数据"),
        )
        .arg(
            Arg::new("separator")
                .short('d')
                .long("separator")
                .value_name("SEP")
                .help("CSV分隔符（配合--csv使用，默认为逗号）"),
        )
        .arg(
            Arg::new("col")
                .long("col")
                .value_name("COLS")
                .help("选择列 (列名, 列号范围如0:5, 或列号列表如2,4,7)"),
        )
        .arg(
            Arg::new("version_flag")
                .long("version")
                .action(ArgAction::SetTrue)
                .help("显示版本信息"),
        )
        .get_matches();

    if let Some(sub_matches) = matches.subcommand_matches("config") {
        let had_any = sub_matches.contains_id("provider")
            || sub_matches.contains_id("model")
            || sub_matches.contains_id("base_url")
            || sub_matches.contains_id("api_key")
            || sub_matches.contains_id("think")
            || sub_matches.contains_id("no_think")
            || sub_matches.contains_id("think_intensity")
            || sub_matches.contains_id("line_width")
            || sub_matches.contains_id("language");

        if !had_any {
            if let Err(e) = commands::config::config_show() {
                eprintln!("错误: {}", e);
                std::process::exit(1);
            }
            return;
        }

        if let Some(provider) = sub_matches.get_one::<String>("provider") {
            if let Err(e) = commands::config::config_set_provider(provider) {
                eprintln!("错误: {}", e);
                std::process::exit(1);
            }
        }
        if let Some(model) = sub_matches.get_one::<String>("model") {
            if let Err(e) = commands::config::config_set_model(model) {
                eprintln!("错误: {}", e);
                std::process::exit(1);
            }
        }
        if let Some(base_url) = sub_matches.get_one::<String>("base_url") {
            if let Err(e) = commands::config::config_set_base_url(base_url) {
                eprintln!("错误: {}", e);
                std::process::exit(1);
            }
        }
        if let Some(api_key) = sub_matches.get_one::<String>("api_key") {
            if let Err(e) = commands::config::config_set_api_key(api_key) {
                eprintln!("错误: {}", e);
                std::process::exit(1);
            }
        }
        if *sub_matches.get_one::<bool>("think").unwrap_or(&false) {
            if let Err(e) = commands::config::config_set_think(true) {
                eprintln!("错误: {}", e);
                std::process::exit(1);
            }
        }
        if *sub_matches.get_one::<bool>("no_think").unwrap_or(&false) {
            if let Err(e) = commands::config::config_set_think(false) {
                eprintln!("错误: {}", e);
                std::process::exit(1);
            }
        }
        if let Some(intensity) = sub_matches.get_one::<String>("think_intensity") {
            if let Err(e) = commands::config::config_set_think_intensity(intensity) {
                eprintln!("错误: {}", e);
                std::process::exit(1);
            }
        }
        if let Some(width_str) = sub_matches.get_one::<String>("line_width") {
            match width_str.parse::<usize>() {
                Ok(w) => {
                    if let Err(e) = commands::config::config_set_line_width(w) {
                        eprintln!("错误: {}", e);
                        std::process::exit(1);
                    }
                }
                Err(_) => {
                    eprintln!("错误: 行宽必须是正整数");
                    std::process::exit(1);
                }
            }
        }
        if let Some(lang) = sub_matches.get_one::<String>("language") {
            if let Err(e) = commands::config::config_set_language(lang) {
                eprintln!("错误: {}", e);
                std::process::exit(1);
            }
        }
        return;
    }

    if let Some(sub_matches) = matches.subcommand_matches("head") {
        let file = sub_matches.get_one::<String>("FILE").unwrap();
        let n: usize = sub_matches.get_one::<String>("num").unwrap().parse().unwrap_or(5);
        let no_name = sub_matches.get_flag("no_name");
        let force_csv = sub_matches.get_flag("csv");
        let separator = extract_separator(sub_matches);
        let col_selection = sub_matches.get_one::<String>("col").map(|s| s.as_str());

        let mgr = match config::ConfigManager::new() {
            Ok(m) => m,
            Err(e) => {
                eprintln!("错误: 无法读取配置: {}", e);
                std::process::exit(1);
            }
        };
        let line_width = mgr.line_width();

        if force_csv || commands::util::is_data_file(file) {
            if let Err(e) = commands::head::head_data_file(file, n, no_name, line_width, force_csv, separator, col_selection) {
                eprintln!("错误: {}", e);
                std::process::exit(1);
            }
        } else {
            if let Err(e) = commands::head::head_text_file(file, n, line_width) {
                eprintln!("错误: {}", e);
                std::process::exit(1);
            }
        }
        return;
    }

    if let Some(sub_matches) = matches.subcommand_matches("tail") {
        let file = sub_matches.get_one::<String>("FILE").unwrap();
        let n: usize = sub_matches.get_one::<String>("num").unwrap().parse().unwrap_or(5);
        let no_name = sub_matches.get_flag("no_name");
        let force_csv = sub_matches.get_flag("csv");
        let separator = extract_separator(sub_matches);
        let col_selection = sub_matches.get_one::<String>("col").map(|s| s.as_str());

        let mgr = match config::ConfigManager::new() {
            Ok(m) => m,
            Err(e) => {
                eprintln!("错误: 无法读取配置: {}", e);
                std::process::exit(1);
            }
        };
        let line_width = mgr.line_width();

        if force_csv || commands::util::is_data_file(file) {
            if let Err(e) = commands::tail::tail_data_file(file, n, no_name, line_width, force_csv, separator, col_selection) {
                eprintln!("错误: {}", e);
                std::process::exit(1);
            }
        } else {
            if let Err(e) = commands::tail::tail_text_file(file, n, line_width) {
                eprintln!("错误: {}", e);
                std::process::exit(1);
            }
        }
        return;
    }

    if let Some(sub_matches) = matches.subcommand_matches("schema") {
        let file = sub_matches.get_one::<String>("FILE").unwrap();
        let direction = sub_matches.get_one::<String>("direction").map(|s| s.as_str()).unwrap_or("col");
        let no_name = sub_matches.get_flag("no_name");
        let force_csv = sub_matches.get_flag("csv");
        let separator = extract_separator(sub_matches);
        let col_selection = sub_matches.get_one::<String>("col").map(|s| s.as_str());

        if direction != "col" && direction != "row" {
            eprintln!("错误: --direction 必须是 'col' 或 'row'");
            std::process::exit(1);
        }

        if force_csv || commands::util::is_data_file(file) {
            if let Err(e) = commands::schema::schema_data_file(file, direction, no_name, force_csv, separator, col_selection) {
                eprintln!("错误: {}", e);
                std::process::exit(1);
            }
        } else {
            if let Err(e) = commands::schema::schema_text_file(file) {
                eprintln!("错误: {}", e);
                std::process::exit(1);
            }
        }
        return;
    }

    if let Some(sub_matches) = matches.subcommand_matches("summary") {
        let file = sub_matches.get_one::<String>("FILE").unwrap();
        let no_name = sub_matches.get_flag("no_name");
        let force_csv = sub_matches.get_flag("csv");
        let separator = extract_separator(sub_matches);
        let col_selection = sub_matches.get_one::<String>("col").map(|s| s.as_str());

        if force_csv || commands::util::is_data_file(file) {
            if let Err(e) = commands::summary::summary_data_file(file, no_name, force_csv, separator, col_selection) {
                eprintln!("错误: {}", e);
                std::process::exit(1);
            }
        } else {
            let rt = tokio::runtime::Runtime::new().unwrap();
            if let Err(e) = rt.block_on(commands::summary::summary_text_file(file)) {
                eprintln!("错误: {}", e);
                std::process::exit(1);
            }
        }
        return;
    }

    if let Some(sub_matches) = matches.subcommand_matches("chat") {
        let file = sub_matches.get_one::<String>("FILE").unwrap();
        let question = sub_matches.get_one::<String>("QUESTION").map(|s| s.as_str());

        let rt = tokio::runtime::Runtime::new().unwrap();
        if let Err(e) = rt.block_on(commands::chat::chat_file(file, question)) {
            eprintln!("错误: {}", e);
            std::process::exit(1);
        }
        return;
    }

    let version_flag = matches.get_flag("version_flag");
    if version_flag {
        println!("SimReader {}", VERSION);
        std::process::exit(0);
    }

    let file = matches.get_one::<String>("file");
    if file.is_none() {
        eprintln!("错误: 请指定文件路径或子命令。使用 --help 查看帮助。");
        std::process::exit(1);
    }
    let file = file.unwrap();

    let summary_flag = matches.get_flag("summary_flag");
    let head_flag = matches.get_flag("head_flag");
    let tail_flag = matches.get_flag("tail_flag");
    let schema_flag = matches.get_flag("schema_flag");
    let quest_val = matches.get_one::<String>("quest_flag");

    let flags_set = [summary_flag, head_flag, tail_flag, schema_flag, quest_val.is_some()]
        .iter()
        .filter(|&&x| x)
        .count();

    if flags_set == 0 {
        eprintln!("错误: 必须指定功能选项之一: --summary/-s, --head/-h, --tail/-t, --schema/-e, --quest/-q <question>");
        std::process::exit(1);
    }

    if flags_set > 1 {
        eprintln!("错误: --summary/-s, --head/-h, --tail/-t, --schema/-e, --quest/-q, --version是互斥的");
        std::process::exit(1);
    }

    let no_name = matches.get_flag("no_name");
    let force_csv = matches.get_flag("csv");
    let separator = extract_separator(&matches);
    let col_selection = matches.get_one::<String>("col").map(|s| s.as_str());

    if summary_flag {
        if force_csv || commands::util::is_data_file(file) {
            if let Err(e) = commands::summary::summary_data_file(file, no_name, force_csv, separator, col_selection) {
                eprintln!("错误: {}", e);
                std::process::exit(1);
            }
        } else {
            let rt = tokio::runtime::Runtime::new().unwrap();
            if let Err(e) = rt.block_on(commands::summary::summary_text_file(file)) {
                eprintln!("错误: {}", e);
                std::process::exit(1);
            }
        }
        return;
    }

    if head_flag {
        let n: usize = matches.get_one::<String>("num").and_then(|s| s.parse().ok()).unwrap_or(5);

        let mgr = match config::ConfigManager::new() {
            Ok(m) => m,
            Err(e) => {
                eprintln!("错误: 无法读取配置: {}", e);
                std::process::exit(1);
            }
        };
        let line_width = mgr.line_width();

        if force_csv || commands::util::is_data_file(file) {
            if let Err(e) = commands::head::head_data_file(file, n, no_name, line_width, force_csv, separator, col_selection) {
                eprintln!("错误: {}", e);
                std::process::exit(1);
            }
        } else {
            if let Err(e) = commands::head::head_text_file(file, n, line_width) {
                eprintln!("错误: {}", e);
                std::process::exit(1);
            }
        }
        return;
    }

    if tail_flag {
        let n: usize = matches.get_one::<String>("num").and_then(|s| s.parse().ok()).unwrap_or(5);

        let mgr = match config::ConfigManager::new() {
            Ok(m) => m,
            Err(e) => {
                eprintln!("错误: 无法读取配置: {}", e);
                std::process::exit(1);
            }
        };
        let line_width = mgr.line_width();

        if force_csv || commands::util::is_data_file(file) {
            if let Err(e) = commands::tail::tail_data_file(file, n, no_name, line_width, force_csv, separator, col_selection) {
                eprintln!("错误: {}", e);
                std::process::exit(1);
            }
        } else {
            if let Err(e) = commands::tail::tail_text_file(file, n, line_width) {
                eprintln!("错误: {}", e);
                std::process::exit(1);
            }
        }
        return;
    }

    if schema_flag {
        if force_csv || commands::util::is_data_file(file) {
            if let Err(e) = commands::schema::schema_data_file(file, "col", no_name, force_csv, separator, col_selection) {
                eprintln!("错误: {}", e);
                std::process::exit(1);
            }
        } else {
            if let Err(e) = commands::schema::schema_text_file(file) {
                eprintln!("错误: {}", e);
                std::process::exit(1);
            }
        }
        return;
    }

    if let Some(question) = quest_val {
        let rt = tokio::runtime::Runtime::new().unwrap();
        if let Err(e) = rt.block_on(commands::chat::chat_file(file, Some(question))) {
            eprintln!("错误: {}", e);
            std::process::exit(1);
        }
        return;
    }
}

fn extract_separator(matches: &clap::ArgMatches) -> Option<u8> {
    let sep_str = matches.get_one::<String>("separator")?;
    if sep_str.is_empty() {
        None
    } else {
        Some(sep_str.as_bytes()[0])
    }
}
