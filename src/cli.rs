//! CLI definition using clap (Pebble Spec v1.0)

use clap::{Parser, Subcommand, Args};

#[derive(Parser)]
#[command(name = "cf")]
#[command(version)]
#[command(about = "Cloudflare 基础设施管理工具")]
#[command(long_about = r#"
CF CLI - Cloudflare 基础设施管理工具

功能说明:
  dns      域名解析管理 - 添加、查看、删除域名记录
  r2       文件存储 - 上传图片、文件到云端，获取公开链接
  caddy    反向代理 - 配置网站域名指向服务
  service  服务检查 - 检测服务是否正常运行
  registry 配置管理 - 管理本地域名配置文件

使用前准备:
  需要在 .env 文件中配置 Cloudflare API 密钥
"#)]
#[command(after_help = r#"
常用示例:

  上传图片到云存储:
    cf r2 upload --file /path/to/image.png

  查看域名解析记录:
    cf dns list --domain example.com

  添加域名解析:
    cf dns create --domain example.com --name api --ip 1.2.3.4

  检查网站是否正常:
    cf service health --url https://api.example.com/health

获取子命令帮助:
    cf r2 --help
    cf dns --help
"#)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,

    /// 机器输出模式 (JSON格式，供程序调用)
    #[arg(long, global = true)]
    pub agent: bool,

    /// 输出工具元信息
    #[arg(long)]
    pub manifest: bool,

    /// 显示详细日志
    #[arg(short, long, global = true)]
    pub verbose: bool,
}

#[derive(Subcommand)]
pub enum Commands {
    /// 域名解析管理 - 添加/查看/删除 DNS 记录
    Dns(DnsCommand),

    /// 反向代理配置 - 管理 Caddy 服务器
    Caddy(CaddyCommand),

    /// 服务状态检查 - 端口、健康检查、容器状态
    Service(ServiceCommand),

    /// 本地配置管理 - registry.json 文件操作
    Registry(RegistryCommand),

    /// 云存储操作 - 上传/下载/管理 R2 文件
    R2(R2Command),
}

// ============ DNS Commands ============

#[derive(Args)]
pub struct DnsCommand {
    #[command(subcommand)]
    pub action: DnsAction,
}

#[derive(Subcommand)]
pub enum DnsAction {
    /// List all DNS records for a domain
    #[command(after_help = "Examples:
  cf dns list --domain example.com
  cf dns list --domain yourdomain.com")]
    List {
        /// Domain name (e.g., example.com, yourdomain.com)
        #[arg(short, long)]
        domain: String,
    },

    /// Get a specific DNS record
    #[command(after_help = "Examples:
  cf dns get --domain example.com --name myapp")]
    Get {
        /// Domain name
        #[arg(short, long)]
        domain: String,

        /// Subdomain name (without domain suffix)
        #[arg(short, long)]
        name: String,
    },

    /// Create a new DNS A record
    #[command(after_help = "Examples:
  cf dns create --domain example.com --name myapp --ip 1.2.3.4 --desc 'My App'")]
    Create {
        /// Domain name
        #[arg(short, long)]
        domain: String,

        /// Subdomain name
        #[arg(short, long)]
        name: String,

        /// IP address
        #[arg(short, long)]
        ip: String,

        /// Description
        #[arg(long, default_value = "")]
        desc: String,

        /// Enable Cloudflare proxy (orange cloud)
        #[arg(long, default_value = "false")]
        proxied: bool,
    },

    /// Delete a DNS record
    #[command(after_help = "Examples:
  cf dns delete --domain example.com --name myapp")]
    Delete {
        /// Domain name
        #[arg(short, long)]
        domain: String,

        /// Subdomain name
        #[arg(short, long)]
        name: String,
    },
}

// ============ Caddy Commands ============

#[derive(Args)]
pub struct CaddyCommand {
    #[command(subcommand)]
    pub action: CaddyAction,
}

#[derive(Subcommand)]
pub enum CaddyAction {
    /// Add reverse proxy configuration
    #[command(after_help = "Examples:
  cf caddy add --server 1.2.3.4 --domain myapp.example.com --upstream localhost:3001")]
    Add {
        /// Server IP address
        #[arg(short, long)]
        server: String,

        /// Domain name (FQDN)
        #[arg(short, long)]
        domain: String,

        /// Upstream address (e.g., localhost:3001)
        #[arg(short, long)]
        upstream: String,
    },

    /// Add load balancer configuration
    #[command(after_help = "Examples:
  cf caddy add-lb --server 1.2.3.4 --domain lb.example.com --upstreams 'localhost:3001,5.6.7.8:3001'")]
    AddLb {
        /// Server IP address
        #[arg(short, long)]
        server: String,

        /// Domain name (FQDN)
        #[arg(short, long)]
        domain: String,

        /// Comma-separated upstream addresses
        #[arg(short, long)]
        upstreams: String,

        /// Health check URI
        #[arg(long, default_value = "/health")]
        health_uri: String,
    },

    /// Reload Caddy configuration
    #[command(after_help = "Examples:
  cf caddy reload --server 1.2.3.4")]
    Reload {
        /// Server IP address
        #[arg(short, long)]
        server: String,
    },

    /// Validate Caddy configuration
    #[command(after_help = "Examples:
  cf caddy validate --server 1.2.3.4")]
    Validate {
        /// Server IP address
        #[arg(short, long)]
        server: String,
    },
}

// ============ Service Commands ============

#[derive(Args)]
pub struct ServiceCommand {
    #[command(subcommand)]
    pub action: ServiceAction,
}

#[derive(Subcommand)]
pub enum ServiceAction {
    /// Check if a port is listening
    #[command(after_help = "Examples:
  cf service check --server 1.2.3.4 --port 3001")]
    Check {
        /// Server IP address
        #[arg(short, long)]
        server: String,

        /// Port number
        #[arg(short, long)]
        port: u16,
    },

    /// Check URL health endpoint
    #[command(after_help = "Examples:
  cf service health --url https://myapp.example.com/health")]
    Health {
        /// Health check URL
        #[arg(short, long)]
        url: String,

        /// Timeout in seconds
        #[arg(short, long, default_value = "10")]
        timeout: u64,
    },

    /// List Docker containers on server
    #[command(after_help = "Examples:
  cf service docker-ps --server 1.2.3.4")]
    DockerPs {
        /// Server IP address
        #[arg(short, long)]
        server: String,
    },

    /// List PM2 processes on server
    #[command(after_help = "Examples:
  cf service pm2-list --server 1.2.3.4")]
    Pm2List {
        /// Server IP address
        #[arg(short, long)]
        server: String,
    },
}

// ============ Registry Commands ============

#[derive(Args)]
pub struct RegistryCommand {
    #[command(subcommand)]
    pub action: RegistryAction,
}

#[derive(Subcommand)]
pub enum RegistryAction {
    /// Validate registry.json
    #[command(after_help = "Examples:
  cf registry validate")]
    Validate,

    /// Add record to registry.json
    #[command(after_help = "Examples:
  cf registry add --domain example.com --name myapp --ip 1.2.3.4 --desc 'My App'")]
    Add {
        /// Domain name
        #[arg(short, long)]
        domain: String,

        /// Subdomain name
        #[arg(short, long)]
        name: String,

        /// IP address
        #[arg(short, long)]
        ip: String,

        /// Description
        #[arg(long)]
        desc: String,
    },

    /// Show registry statistics
    #[command(after_help = "Examples:
  cf registry stats")]
    Stats,
}

// ============ R2 Commands ============

#[derive(Args)]
#[command(about = "云存储操作 - 上传文件到 Cloudflare R2，获取公开访问链接")]
#[command(after_help = r#"
快速上传:
  cf r2 upload --file 图片.png        # 上传后返回公开链接

查看已上传文件:
  cf r2 list                          # 列出所有文件
  cf r2 list --prefix uploads/        # 只看 uploads 目录

提示: 上传成功后会返回 public_url，可直接在浏览器打开
"#)]
pub struct R2Command {
    #[command(subcommand)]
    pub action: R2Action,
}

#[derive(Subcommand)]
pub enum R2Action {
    /// 上传文件 - 支持图片、视频、文档等任意文件
    #[command(after_help = r#"
示例:
  cf r2 upload --file /path/to/image.png
  cf r2 upload --file photo.jpg --key images/2024/photo.jpg

上传成功后返回:
  - public_url: 公开访问链接，可直接分享
  - key: 文件在云端的路径
  - size: 文件大小
"#)]
    Upload {
        /// 要上传的本地文件路径
        #[arg(short, long)]
        file: String,

        /// 自定义云端路径 (不填则自动放到 uploads/ 目录)
        #[arg(short, long)]
        key: Option<String>,

        /// 是否返回公开链接 (默认: 是)
        #[arg(long, default_value = "true")]
        public: bool,
    },

    /// 查看文件列表 - 列出云端已上传的文件
    #[command(after_help = r#"
示例:
  cf r2 list                    # 列出所有文件
  cf r2 list --prefix uploads/  # 只看 uploads 目录下的文件
  cf r2 list --limit 20         # 只显示前 20 个
"#)]
    List {
        /// 按前缀筛选 (如: uploads/, images/)
        #[arg(short, long, default_value = "")]
        prefix: String,

        /// 最多显示多少个文件
        #[arg(short, long, default_value = "100")]
        limit: u32,
    },

    /// 删除文件 - 从云端删除指定文件
    #[command(after_help = r#"
示例:
  cf r2 delete --key uploads/image.png

注意: 删除后无法恢复，请谨慎操作
"#)]
    Delete {
        /// 要删除的文件路径 (从 list 命令获取)
        #[arg(short, long)]
        key: String,
    },

    /// 查看文件信息 - 获取文件大小、类型、链接等
    #[command(after_help = r#"
示例:
  cf r2 info --key uploads/image.png

返回: 文件大小、类型、最后修改时间、公开链接
"#)]
    Info {
        /// 文件路径
        #[arg(short, long)]
        key: String,
    },
}
