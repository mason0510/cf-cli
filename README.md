# CF CLI

> Cloudflare infrastructure management CLI for Claude Code and AI Agents

<div align="center">

![Rust](https://img.shields.io/badge/rust-1.70%2B-orange.svg)
![License](https://img.shields.io/badge/license-MIT-green.svg)
![Pebble](https://img.shields.io/badge/pebble-v1.1-blue.svg)

**Pebble Spec v1.1 compliant** - Designed for AI agent integration

[Features](#features) • [Installation](#installation) • [Usage](#usage) • [Configuration](#configuration) • [Development](#development)

</div>

---

## ✨ Features

- 🌐 **DNS Management** - Create, list, get, delete DNS records via Cloudflare API
- 🔄 **Caddy Proxy** - Add reverse proxy, load balancer configs via SSH
- 🏥 **Service Health Checks** - Monitor port listening, health endpoints, Docker/PM2 status
- 📋 **Registry Management** - Local registry.json validation and management
- 🔐 **R2 Storage** - Upload/download files to Cloudflare R2 (S3-compatible)
- 🤖 **AI Agent Mode** - JSON Lines output for seamless AI integration
- 📊 **Structured Errors** - Pebble-compliant error format with retry hints
- 🎯 **Zero Config** - Works with environment variables, no config files needed

## 🚀 Installation

### Prerequisites

- Rust 1.70 or higher
- Cargo (Rust package manager)

### From Source

```bash
# Clone the repository
git clone https://github.com/YOUR_USERNAME/cf-cli.git
cd cf-cli

# Build release binary
cargo build --release

# Install to PATH
cp target/release/cf ~/.cargo/bin/
# or
sudo cp target/release/cf /usr/local/bin/
```

### Using Cargo

```bash
cargo install --path .
```

## Usage

```bash
# DNS operations
cf dns list --domain example.com
cf dns create --domain example.com --name api --ip 1.2.3.4
cf dns delete --domain example.com --name api

# Caddy reverse proxy
cf caddy add --server 1.2.3.4 --domain api.example.com --upstream localhost:3000
cf caddy reload --server 1.2.3.4

# Service checks
cf service health --url https://api.example.com/health
cf service check --server 1.2.3.4 --port 3000

# Registry management
cf registry stats
cf registry validate
```

## Agent Mode

Use `--agent` flag for JSON Lines output (for Claude Code integration):

```bash
cf dns list --domain example.com --agent
```

Output format (Pebble Spec v1.1):
```json
{"v":1,"type":"log","payload":{"level":"info","message":"..."}}
{"v":1,"type":"result","payload":{...}}
```

## Manifest

```bash
cf --manifest
```

Returns machine-readable tool manifest for agent discovery.

## ⚙️ Configuration

### Environment Variables

CF CLI uses environment variables for configuration. Create a `.env` file or export variables:

```bash
# Copy the example file
cp .env.example .env

# Edit .env and fill in your values
nano .env
```

**Required variables per domain**:

```bash
# Pattern: CLOUDFLARE_{DOMAIN_SLUG}_{ZONE_ID|API_TOKEN}
# Example for domain example.com:
CLOUDFLARE_EXAMPLE_ZONE_ID=your_zone_id_here
CLOUDFLARE_EXAMPLE_API_TOKEN=your_api_token_here

# Example for domain yourdomain.com:
CLOUDFLARE_YOURDOMAIN_ZONE_ID=your_zone_id_here
CLOUDFLARE_YOURDOMAIN_API_TOKEN=your_api_token_here
```

**Optional R2 storage variables**:

```bash
CLOUDFLARE_R2_BUCKET_NAME=your_bucket_name
CLOUDFLARE_R2_S3_API_URL=https://your_account_id.r2.cloudflarestorage.com
CLOUDFLARE_R2_ACCESS_KEY_ID=your_access_key_id
CLOUDFLARE_R2_SECRET_ACCESS_KEY=your_secret_access_key
CLOUDFLARE_R2_PUBLIC_URL=https://your_public_url
CLOUDFLARE_R2_FOLDER_PREFIX=uploads/
```

### Getting API Credentials

1. **Cloudflare API Token**:
   - Visit https://dash.cloudflare.com/profile/api-tokens
   - Click "Create Token"
   - Use "Edit zone DNS" template or create custom token
   - Copy the token to your `.env` file

2. **Zone ID**:
   - Go to your domain dashboard on Cloudflare
   - Scroll down to "API" section on the right sidebar
   - Copy the "Zone ID"

3. **R2 Credentials** (optional):
   - Go to R2 dashboard on Cloudflare
   - Create R2 bucket if needed
   - Generate API tokens under "Manage R2 API Tokens"

## Pebble Spec

This CLI follows [Pebble Spec v1.1](./PEBBLE-SPEC.md) - a standard for AI-friendly CLI tools.

Key features:
- Dual mode: Human-readable (default) + Agent mode (JSON Lines)
- Structured error format with retry hints
- Machine-readable manifest
- Consistent exit codes

## Development

### Release Process

**Important**: When code is updated, notify users via Bark:

```bash
# 1. Build new version
cd ~/code/opensource/cf-cli
cargo build --release

# 2. Install
cp target/release/cf ~/bin/cf

# 3. Notify user via Bark (requires BARK_KEY env or --key)
bark "cf-cli 已更新" "新版本已安装到 ~/bin/cf" --group "dev"
```

### Sync to Production

Keep both directories in sync:

```bash
# Opensource → Production
cp -r ~/code/opensource/cf-cli/src/* ~/code/06-production-business-money-live/cloudflare/cf-cli/src/

# Production → Opensource
cp -r ~/code/06-production-business-money-live/cloudflare/cf-cli/src/* ~/code/opensource/cf-cli/src/
```

## License

MIT
