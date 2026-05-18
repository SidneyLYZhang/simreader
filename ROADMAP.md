# SimReader 路线图

## 1. 实现管道输入

### 背景

当前 SimReader 仅支持通过命令行参数指定文件路径（如 `simreader head data.csv`），不支持从标准输入（stdin）或管道读取数据。实现管道输入后，可以与其他命令行工具组合使用，提升灵活性。

### 目标

- 支持 `stdin` / 管道输入，当未指定文件路径时自动从标准输入读取
- 保持向后兼容，原有文件路径模式不受影响

### 计划任务

- [x] **统一输入抽象层**：在 `reader` 模块中引入一个通用的输入源抽象（如 `InputSource` 枚举），支持 `File`、`Stdin`、`Bytes` 等多种输入源，统一处理不同类型的数据读取逻辑。

- [x] **支持 stdin 读取**：在各子命令（`head`、`tail`、`schema`、`summary`、`rows`、`chat`）中添加 stdin 回退逻辑。当 `FILE` 参数未提供时，读取 stdin 的内容。

- [x] **格式检测兼容**：确保 stdin 输入也能正确进行文件格式检测（CSV/TSV/JSON/Parquet/Excel/Text）。对于非文本格式（如 Parquet），考虑要求用户显式指定格式，或通过 magic bytes 自动检测。

- [x] **Chat 命令适配**：`chat` 命令在没有文件上下文时，也应支持直接从 stdin 获取内容进行分析问答。

- [x] **测试覆盖**：添加管道输入的集成测试，覆盖各子命令场景。

### 示例用法

```bash
# 管道 + head
cat data.csv | simreader head -n 5

# 管道 + tail
tail -20 large.log | simreader tail -n 10

# 管道 + 数据分析
curl -s https://example.com/data.json | simreader schema

# 管道 + AI 分析
cat report.txt | simreader chat "总结一下内容"
```

---

## 2. 使用 genai 替换现有 LLM 实现

### 背景

当前 LLM 模块通过 `reqwest` 直接构建 HTTP 请求，针对 DeepSeek 和 OpenRouter 分别实现了独立的 provider。随着支持的模型增多，这种方式会导致大量重复代码。引入 [genai](https://crates.io/crates/genai) 库可以统一多供应商的调用接口，减少维护成本。

### 目标

- 用 `genai` 替换现有的 LLM 调用层（`llm/` 模块）
- 保留 DeepSeek 和 OpenRouter 的支持，同时可轻松扩展更多模型
- 简化配置管理和流式响应的处理逻辑

### 计划任务

- [ ] **调研 genai 能力**：确认 `genai` 是否覆盖当前需要的功能：DeepSeek provider 支持、OpenRouter provider 支持（兼容 OpenAI 协议）、流式输出（streaming）、思考模式（thinking / reasoning）、temperature / max_tokens 等参数透传。

- [ ] **重构 LLM 模块**：将 `src/llm/` 下的 `deepseek.rs` 和 `openrouter.rs` 替换为基于 `genai` 的统一实现。保留或简化 `LlmProvider` trait，使其成为 `genai` 的轻量封装。

- [ ] **迁移配置系统**：`ConfigManager` 中的 LLM 配置项（provider、model、base_url、api_key、thinking 设置等）适配 `genai` 的配置方式，尽量保持配置文件格式不变。

- [ ] **迁移 Chat 与 Summary 命令**：更新 `chat` 和 `summary` 命令中对 LLM provider 的调用，适配新的 `genai` 接口，确保交互式问答和 AI 摘要功能正常。

- [ ] **清理旧代码**：移除不再需要的依赖（如直接使用的 `reqwest` LLM 客户端代码）、清理废弃的 HTTP 请求构建逻辑。

- [ ] **测试验证**：运行现有测试套件，确保迁移后功能一致性；针对性补充 `genai` 相关的单元测试。

### 潜在问题

- `genai` 对 DeepSeek "thinking" 模式的兼容性需验证
- `genai` 的 keyring 集成与现有 `keyring` crate 的协调
- 流式输出 API 差异可能需要适配
