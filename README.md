# CF CLI

> Cloudflare 基础设施管理 CLI，专为 AI Agent 设计

<div align="center">

![Rust](https://img.shields.io/badge/rust-1.70%2B-orange.svg)
![License](https://img.shields.io/badge/license-MIT-green.svg)
![Pebble](https://img.shields.io/badge/pebble-v1.1-blue.svg)

**Pebble Spec v1.1 规范兼容** - 为 AI Agent 集成而设计

[English](./README_EN.md) • [简体中文](./README.md)

</div>

---

## ✨ 特性

- 🌐 **DNS 管理** - 通过 Cloudflare API 创建、列出、获取、删除 DNS 记录
- 🔄 **Caddy 反向代理** - 通过 SSH 添加反向代理、负载均衡配置
- 🏥 **服务健康检查** - 监控端口监听、健康端点、Docker/PM2 状态
- 📋 **Registry 管理** - 本地 registry.json 验证和管理
- 🔐 **R2 存储** - 上传/下载文件到 Cloudflare R2（兼容 S3）
- 🤖 **AI Agent 模式** - JSON Lines 输出，无缝 AI 集成
- 📊 **结构化错误** - Pebble 规范兼容的错误格式，带重试提示
- 🎯 **零配置** - 使用环境变量，无需配置文件

## 🚀 快速开始

### 前置要求

- Rust 1.70 或更高版本
- Cargo（Rust 包管理器）

### 从源码安装

```bash
# 克隆仓库
git clone https://github.com/mason0510/cf-cli.git
cd cf-cli

# 编译 Release 版本
cargo build --release

# 安装到 PATH
cp target/release/cf ~/.cargo/bin/
# 或
sudo cp target/release/cf /usr/local/bin/
```

### 使用 Cargo 安装

```bash
cargo install --path .
```

## 📖 使用方法

### 基础命令

```bash
# 查看帮助
cf --help

# 查看工具清单（Agent 模式）
cf --manifest

# 查看域名解析记录
cf dns list --domain example.com

# 添加域名解析
cf dns create --domain example.com --name api --ip 1.2.3.4

# 检查网站是否正常
cf service health --url https://api.example.com/health
```

### 子命令帮助

```bash
cf dns --help           # DNS 管理
cf caddy --help         # Caddy 配置
cf service --help       # 服务检查
cf registry --help      # Registry 管理
```

## ⚙️ 配置

### 环境变量

CF CLI 使用环境变量进行配置。创建 `.env` 文件或导出变量：

```bash
# 复制示例文件
cp .env.example .env

# 编辑 .env 并填入您的值
nano .env
```

**每个域名需要的变量**：

```bash
# 模式：CLOUDFLARE_{域名简称}_{ZONE_ID|API_TOKEN}
# 示例域名：example.com
CLOUDFLARE_EXAMPLE_ZONE_ID=your_zone_id_here
CLOUDFLARE_EXAMPLE_API_TOKEN=your_api_token_here

# 示例域名：yourdomain.com
CLOUDFLARE_YOURDOMAIN_ZONE_ID=your_zone_id_here
CLOUDFLARE_YOURDOMAIN_API_TOKEN=your_api_token_here
```

**可选的 R2 存储变量**：

```bash
CLOUDFLARE_R2_BUCKET_NAME=your_bucket_name
CLOUDFLARE_R2_S3_API_URL=https://your_account_id.r2.cloudflarestorage.com
CLOUDFLARE_R2_ACCESS_KEY_ID=your_access_key_id
CLOUDFLARE_R2_SECRET_ACCESS_KEY=your_secret_access_key
CLOUDFLARE_R2_PUBLIC_URL=https://your_public_url
CLOUDFLARE_R2_FOLDER_PREFIX=uploads/
```

### 获取 API 凭证

1. **Cloudflare API Token**：
   - 访问 https://dash.cloudflare.com/profile/api-tokens
   - 点击 "Create Token"
   - 使用 "Edit zone DNS" 模板或创建自定义 Token
   - 复制 Token 到您的 `.env` 文件

2. **Zone ID**：
   - 在 Cloudflare 上进入您的域名面板
   - 滚动到右侧边栏的 "API" 部分
   - 复制 "Zone ID"

3. **R2 凭证**（可选）：
   - 进入 Cloudflare 的 R2 面板
   - 如需要，创建 R2 存储桶
   - 在 "Manage R2 API Tokens" 下生成 API Token

## 🤖 AI Agent 模式

CF CLI 完全支持 Pebble Spec v1.1，可与 Claude Code 等 AI 工具无缝集成。

### Agent 模式输出

```bash
# JSON Lines 输出
cf dns list --domain example.com --json

# 工具清单（自动发现能力）
cf --manifest
```

### 结构化错误

所有错误遵循 Pebble 规范，包含重试提示和上下文信息：

```json
{
  "error": "Missing environment variable: CLOUDFLARE_EXAMPLE_ZONE_ID",
  "context": {
    "domain": "example.com",
    "required_var": "CLOUDFLARE_EXAMPLE_ZONE_ID"
  },
  "retry_hint": "Add the missing environment variable to your .env file"
}
```

## 📋 Pebble 规范

本 CLI 遵循 [Pebble Spec v1.1](./PEBBLE-SPEC.md) - AI 友好 CLI 工具标准。

### 支持的 Pebble 特性

- ✅ `--manifest` - 输出工具清单（JSON）
- ✅ `--agent` - Agent 模式（JSON Lines 输出）
- ✅ 结构化错误（带 `retry_hint`）
- ✅ 环境变量配置（零配置文件）
- ✅ 标准退出码

## 📚 详细文档

- **DNS 管理**：创建、列出、更新、删除 DNS 记录
- **Caddy 集成**：通过 SSH 配置反向代理和负载均衡
- **健康检查**：端口检查、HTTP 健康端点、Docker/PM2 状态
- **R2 存储**：文件上传/下载、批量操作
- **Registry 管理**：本地域名和服务器信息管理

## 🛠️ 开发

```bash
# 克隆仓库
git clone https://github.com/mason0510/cf-cli.git
cd cf-cli

# 开发构建
cargo build

# 运行测试
cargo test

# 检查代码
cargo clippy

# 格式化代码
cargo fmt
```

## 📄 许可证

MIT License - 详见 [LICENSE](LICENSE)

## 🤝 贡献

欢迎贡献！请随时提交 Issue 或 Pull Request。

## 🔗 相关项目

- [pebble](https://github.com/mason0510/pebble) - 面向 AI Agent 的 CLI 技能规范查询工具
- [Pebble Spec](https://github.com/mason0510/pebble/blob/main/PEBBLE-SPEC.md) - AI 友好 CLI 工具标准

## 📧 联系方式

- GitHub: [@mason0510](https://github.com/mason0510)
- Issues: [GitHub Issues](https://github.com/mason0510/cf-cli/issues)

---

**让 Cloudflare 管理更简单！** 🚀
