# Pebble 开发标准 v1.1

> 面向 Agent 的下一代 CLI 技能规范
>
> **核心理念**：用户不看文档，只靠 `--help` 就能完全掌握用法

---

## 一、双模设计

```
┌─────────────────────────────────────────┐
│         一个 Pebble 两种用法             │
├─────────────────────────────────────────┤
│                                         │
│  精简模式（CLI Tool）                    │
│  ├─ 触发：子命令调用                    │
│  ├─ 输出：stdout=JSON Lines, stderr=日志│
│  └─ 用途：被 Claude Code 调用           │
│                                         │
├─────────────────────────────────────────┤
│                                         │
│  智能体模式（Agent）= 精简模式超集       │
│  ├─ 触发：-i / --interactive            │
│  ├─ 增强：会话状态、多轮交互            │
│  └─ 用途：独立运行/售卖                 │
│                                         │
└─────────────────────────────────────────┘
```

### 1.1 入口约定（重要）

| 调用方式 | 模式 | 说明 |
|----------|------|------|
| `miao download <url>` | 精简 | 子命令 = 精简模式 |
| `miao download <url> --agent` | 精简 | 强制 JSON Lines 输出 |
| `miao -i` | 智能体 | 交互模式 |
| `miao` (无参数) | 智能体 | 默认进入交互（可配置） |

**避免**：模糊入口、既像子命令又像交互

### 1.2 输出约定（强制）

```
stdout: 仅 JSON Lines（机器可解析）
stderr: 人类日志/进度条/调试信息
```

---

## 二、强制要求

| 要求 | 说明 |
|------|------|
| **语言** | Rust |
| **--help** | 自解释，层次分明 |
| **--version** | 版本号 |
| **--manifest** | JSON 元数据 |
| **--agent** | 强制 JSON Lines 输出 |
| **退出码** | 0/1/2/3/4 |

---

## 三、命令符号兼容性（强制）

> **原则**：兼容 Claude Code 命令符号，一致功能用一致命令

### 3.1 核心符号（来自 Claude Code）

| 短参数 | 长参数 | 功能 | 说明 |
|--------|--------|------|------|
| `-p` | `--print` | 单次输出 | 非交互，输出后退出 |
| `-c` | `--continue` | 继续会话 | 继续当前目录最近会话 |
| `-r` | `--resume` | 恢复会话 | 按会话ID恢复 |
| `-d` | `--debug` | 调试模式 | 可选过滤类别 |
| `-v` | `--version` | 版本号 | |
| `-h` | `--help` | 帮助 | |

### 3.2 重要长参数（来自 Claude Code）

| 参数 | 功能 | Pebble 对应 |
|------|------|-------------|
| `--model <model>` | 模型选择 | 按需 |
| `--output-format <fmt>` | 输出格式 text/json/stream-json | `--agent` |
| `--session-id <uuid>` | 指定会话ID | `--session` |
| `--verbose` | 详细输出 | `-v` 或 `--verbose` |
| `--system-prompt` | 系统提示 | 按需 |

### 3.3 Pebble 专用符号

| 参数 | 功能 |
|------|------|
| `--agent` | JSON Lines 输出（≈ `--output-format json`） |
| `--manifest` | 输出 Pebble 元数据 |

### 3.4 规则

```
Claude Code 风格        Pebble 风格
─────────────────────────────────────
claude -p "问题"    →   pebble -p "查询"
claude -c           →   pebble -c
claude -r <id>      →   pebble -r <id>
claude --model x    →   pebble --model x

位置参数 = -p 简写
  pebble "查询" ≡ pebble -p "查询"
```

---

## 四、事件协议 (JSON Lines)

### 4.1 通用格式

```json
{"v":1,"type":"<type>","payload":{...}}
```

- `v`: Schema 版本号（必须）
- `type`: 事件类型
- `payload`: 事件数据

### 4.2 事件类型

| type | 方向 | 说明 |
|------|------|------|
| `progress` | out | 进度更新 |
| `log` | out | 日志输出 |
| `result` | out | 最终结果 |
| `error` | out | 错误信息 |
| `ask` | out | 询问用户 |
| `answer` | in | 用户回答 |
| `ready` | out | 智能体就绪 |
| `bye` | out | 会话结束 |
| `confirm` | out | 高风险操作确认 |
| `cancelled` | out | 操作被取消 |

### 4.3 示例

```json
{"v":1,"type":"progress","payload":{"percent":50,"message":"下载中..."}}
{"v":1,"type":"result","payload":{"file":"video.mp4","size":10485760}}
{"v":1,"type":"ask","payload":{"id":"q1","question":"选择格式","options":["mp4","mp3"]}}
{"v":1,"type":"confirm","payload":{"id":"c1","action":"overwrite","path":"/tmp/video.mp4"}}
```

---

## 五、错误格式

### 5.1 完整格式

```json
{
  "v": 1,
  "type": "error",
  "payload": {
    "code": "TIMEOUT",
    "cat": "net",
    "op": "download.fetch",
    "retryable": true,
    "retry_after_s": 5,
    "fix": ["proxy", "wait"],
    "message": "Request timed out",
    "trace_id": "abc123",
    "details": {"url": "...", "timeout_ms": 30000}
  }
}
```

### 5.2 字段说明

| 字段 | 必需 | 说明 |
|------|------|------|
| `code` | ✓ | 错误码（机器主键） |
| `cat` | ✓ | 分类 |
| `retryable` | ✓ | 是否可重试 |
| `fix` | ✓ | 修复建议（数组） |
| `message` | | 人类可读描述 |
| `op` | | 操作阶段 |
| `retry_after_s` | | 重试等待秒数 |
| `trace_id` | | 追踪ID |
| `details` | | 详细上下文 |

### 5.3 分类与修复

| cat | 含义 | 常用 fix | 退出码 |
|-----|------|----------|--------|
| `in` | 输入错误 | `param` | 1 |
| `net` | 网络错误 | `proxy`, `wait` | 2 |
| `auth` | 鉴权错误 | `auth` | 3 |
| `ext` | 外部错误 | `wait`, `report` | 2 |
| `sys` | 系统错误 | `report` | 2 |
| `time` | 超时 | `wait` | 4 |

### 5.4 Claude Code 处理

```
retryable=true → sleep(retry_after_s) → 重试
fix 包含 proxy → 提示用户设置代理
fix 包含 auth → 提示用户配置 token
fix 包含 param → 检查参数，提示修正
fix 包含 report → 报告用户/开发者
```

---

## 五、--manifest 规范

```json
{
  "schema_version": "1.0",
  "pebble": {
    "name": "miao",
    "display_name": "视频下载器",
    "version": "1.0.0",
    "description": "从抖音、B站下载视频",
    "homepage": "https://github.com/xxx/miao"
  },
  "capabilities": {
    "agent": true,
    "interactive": true,
    "streaming": true,
    "resume": true
  },
  "actions": [
    {
      "id": "download",
      "summary": "下载视频",
      "args": [
        {"name": "url", "type": "string", "required": true}
      ],
      "options": [
        {"name": "format", "type": "enum", "values": ["mp4","mp3"], "default": "mp4"},
        {"name": "output", "type": "path", "default": "."}
      ]
    }
  ],
  "permissions": {
    "network": true,
    "network_domains": ["douyin.com", "bilibili.com"],
    "filesystem": {"write": ["$OUTPUT_DIR"]},
    "env_vars": ["PROXY", "API_KEY"]
  },
  "limits": {
    "default_timeout_s": 300,
    "max_output_mb": 100
  }
}
```

---

## 六、智能体模式

### 6.1 会话管理

```bash
miao -i                     # 新会话
miao -i --session abc123    # 指定会话ID
miao -i --resume            # 继续上次会话
miao -i --export session.json  # 导出会话
```

### 6.2 会话状态存储

```
~/.config/miao/sessions/
├── latest -> abc123.json
├── abc123.json
└── def456.json
```

### 6.3 交互协议

```bash
$ miao -i
{"v":1,"type":"ready","payload":{"name":"miao","version":"1.0.0","session_id":"abc123"}}

帮我下载这个视频
{"v":1,"type":"ask","payload":{"id":"q1","question":"请提供视频链接"}}

https://douyin.com/xxx
{"v":1,"type":"progress","payload":{"percent":50}}
{"v":1,"type":"result","payload":{"file":"video.mp4"}}

exit
{"v":1,"type":"bye","payload":{"session_id":"abc123"}}
```

### 6.4 高风险操作确认

```json
{"v":1,"type":"confirm","payload":{"id":"c1","action":"overwrite","path":"/tmp/video.mp4","risk":"high"}}
```

用户回复：
```json
{"type":"confirm_response","payload":{"id":"c1","approved":true}}
```

---

## 七、CLI 结构（clap）

### 7.1 --help 示例

```
$ miao --help

miao - 视频下载器

从抖音、B站、视频号下载视频/音频/字幕

Usage: miao [OPTIONS] <COMMAND>
       miao -i [OPTIONS]

Commands:
  download    下载视频/音频
  extract     提取字幕
  info        查看视频信息

Options:
  -i, --interactive      进入交互模式
      --agent            Agent模式（JSON Lines输出）
      --manifest         输出元数据
      --session <ID>     指定会话ID
      --resume           继续上次会话
      --log-level <L>    日志级别 [default: info]
  -h, --help             帮助信息
  -V, --version          版本号

Examples:
  miao download https://douyin.com/xxx
  miao download https://bilibili.com/xxx -f mp3
  miao -i
```

### 7.2 子命令 --help

```
$ miao download --help

下载视频/音频

Usage: miao download [OPTIONS] <URL>

Arguments:
  <URL>  视频URL

Options:
  -f, --format <FORMAT>  输出格式 [default: mp4] [possible: mp4, mp3]
  -o, --output <DIR>     输出目录 [default: .]
      --timeout <MS>     超时毫秒 [default: 300000]
  -h, --help             帮助信息

Examples:
  miao download https://douyin.com/xxx
  miao download https://bilibili.com/xxx -f mp3 -o ~/Downloads
```

---

## 八、Rust 实现

### 8.1 依赖

```toml
[dependencies]
clap = { version = "4", features = ["derive"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tokio = { version = "1", features = ["full"] }
```

### 8.2 事件输出

```rust
use serde::Serialize;

const SCHEMA_VERSION: u8 = 1;

#[derive(Serialize)]
struct Event<T: Serialize> {
    v: u8,
    #[serde(rename = "type")]
    event_type: String,
    payload: T,
}

fn emit<T: Serialize>(event_type: &str, payload: T) {
    let event = Event {
        v: SCHEMA_VERSION,
        event_type: event_type.to_string(),
        payload,
    };
    println!("{}", serde_json::to_string(&event).unwrap());
}

// 用法
emit("progress", json!({"percent": 50, "message": "下载中..."}));
emit("result", json!({"file": "video.mp4"}));
```

### 8.3 错误处理

```rust
#[derive(Serialize)]
struct PebbleError {
    code: String,
    cat: String,
    op: Option<String>,
    retryable: bool,
    retry_after_s: Option<u32>,
    fix: Vec<String>,
    message: Option<String>,
    trace_id: Option<String>,
    details: Option<Value>,
}

impl PebbleError {
    fn net(code: &str) -> Self {
        Self {
            code: code.into(),
            cat: "net".into(),
            op: None,
            retryable: true,
            retry_after_s: Some(5),
            fix: vec!["proxy".into(), "wait".into()],
            message: None,
            trace_id: None,
            details: None,
        }
    }

    fn emit_and_exit(self) -> ! {
        emit("error", &self);
        let code = match self.cat.as_str() {
            "in" => 1,
            "auth" => 3,
            "time" => 4,
            _ => 2,
        };
        std::process::exit(code);
    }
}
```

---

## 九、检查清单

```
必须:
  [v] 字段      所有事件带 schema 版本
  --help        层次分明 + 示例
  --version     版本号
  --manifest    JSON 元数据
  --agent       JSON Lines 输出
  退出码        0/1/2/3/4

错误:
  code + cat + retryable + fix
  可选: op, retry_after_s, message, trace_id, details

输出:
  stdout = JSON Lines（机器）
  stderr = 人类日志

智能体模式（可选）:
  -i / --interactive
  --session / --resume
  confirm 事件（高风险操作）
```

---

## 十、兼容性策略

### 10.1 Schema 版本规则

- `v: 1` 当前版本
- 新增字段：向后兼容，消费者忽略未知字段
- 删除/修改字段：`v` 递增，提供迁移指南

### 10.2 枚举扩展规则

- `cat`/`fix` 可新增值
- 消费者遇到未知值：按默认策略处理

---

## 十一、可选实施：发布通知

> 当 Pebble 工具代码更新时，通过推送通知告知用户

### 11.1 适用场景

- CLI 工具有新版本发布
- 修复了重要 bug
- 新增了功能
- API 有破坏性变更

### 11.2 通知渠道

推荐使用 [Bark](https://github.com/Finb/Bark) 推送通知：

```bash
# 基础用法
bark "<工具名> 已更新" "<更新说明>" --group "dev"

# 带跳转链接
bark "<工具名> v1.2.0" "新增xxx功能" --url "https://github.com/xxx/releases"
```

### 11.3 发布流程模板

```bash
#!/bin/bash
# release.sh - Pebble 工具发布脚本

TOOL_NAME="cf"
VERSION=$(grep '^version' Cargo.toml | head -1 | cut -d'"' -f2)

# 1. 构建
cargo build --release

# 2. 安装
cp target/release/$TOOL_NAME ~/bin/

# 3. 通知用户
bark "$TOOL_NAME v$VERSION 已更新" "已安装到 ~/bin/$TOOL_NAME" --group "dev"

echo "✅ $TOOL_NAME v$VERSION 发布完成"
```

### 11.4 环境配置

```bash
# ~/.bashrc 或 ~/.zshrc
export BARK_KEY="your-bark-key"
export BARK_SERVER="https://api.day.app"  # 可选，默认值
```

### 11.5 通知内容规范

| 字段 | 内容 |
|------|------|
| 标题 | `<工具名> v<版本> 已更新` |
| 正文 | 简要说明更新内容（1-2句） |
| 分组 | `dev`（开发工具类） |
| 链接 | 可选，指向 changelog 或 release 页面 |

---

*Pebble Spec v1.1 | 2026-01-22 | 综合 codex 审核意见*
