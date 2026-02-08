//! Pebble Spec v1.1 compliant output
//!
//! - All JSON Lines include schema version (v: 1)
//! - stdout = JSON Lines only
//! - stderr = human logs

use serde::Serialize;
use serde_json::{json, Value};

const SCHEMA_VERSION: u8 = 1;

/// Event wrapper with schema version
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

/// Output handler
pub struct Output {
    agent_mode: bool,
}

impl Output {
    pub fn new(agent_mode: bool) -> Self {
        Self { agent_mode }
    }

    /// Log message (stderr for human, JSON Lines for agent)
    pub fn log(&self, level: &str, message: &str) {
        if self.agent_mode {
            emit("log", json!({"level": level, "message": message}));
        } else {
            eprintln!("[{}] {}", level.to_uppercase(), message);
        }
    }

    /// Progress update
    pub fn progress(&self, percent: u8, message: &str) {
        if self.agent_mode {
            emit("progress", json!({"percent": percent, "message": message}));
        } else {
            eprintln!("[{:3}%] {}", percent, message);
        }
    }

    /// Final result (always JSON to stdout)
    pub fn result<T: Serialize>(&self, data: T) {
        if self.agent_mode {
            emit("result", data);
        } else {
            // Human mode: pretty print to stdout
            println!("{}", serde_json::to_string_pretty(&data).unwrap());
        }
    }

    /// Error output (Pebble Spec v1.1)
    pub fn error(&self, err: PebbleError) {
        if self.agent_mode {
            emit("error", &err);
        } else {
            eprintln!("Error [{}][{}]: {}", err.cat, err.code, err.message.as_deref().unwrap_or(""));
            if err.retryable {
                if let Some(s) = err.retry_after_s {
                    eprintln!("  Retry after: {}s", s);
                }
            }
            eprintln!("  Fix: {:?}", err.fix);
        }
        std::process::exit(err.exit_code());
    }
}

/// Pebble Error (v1.1 spec)
#[derive(Serialize)]
pub struct PebbleError {
    pub code: String,
    pub cat: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub op: Option<String>,
    pub retryable: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retry_after_s: Option<u32>,
    pub fix: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trace_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<Value>,
}

impl PebbleError {
    /// Network error
    pub fn net(code: &str, message: &str) -> Self {
        Self {
            code: code.into(),
            cat: "net".into(),
            op: None,
            retryable: true,
            retry_after_s: Some(5),
            fix: vec!["proxy".into(), "wait".into()],
            message: Some(message.into()),
            trace_id: None,
            details: None,
        }
    }

    /// Input error
    pub fn input(code: &str, message: &str) -> Self {
        Self {
            code: code.into(),
            cat: "in".into(),
            op: None,
            retryable: false,
            retry_after_s: None,
            fix: vec!["param".into()],
            message: Some(message.into()),
            trace_id: None,
            details: None,
        }
    }

    /// Auth error
    pub fn auth(code: &str, message: &str) -> Self {
        Self {
            code: code.into(),
            cat: "auth".into(),
            op: None,
            retryable: false,
            retry_after_s: None,
            fix: vec!["auth".into()],
            message: Some(message.into()),
            trace_id: None,
            details: None,
        }
    }

    /// External service error
    pub fn ext(code: &str, message: &str) -> Self {
        Self {
            code: code.into(),
            cat: "ext".into(),
            op: None,
            retryable: true,
            retry_after_s: Some(5),
            fix: vec!["wait".into(), "report".into()],
            message: Some(message.into()),
            trace_id: None,
            details: None,
        }
    }

    /// System error
    pub fn sys(code: &str, message: &str) -> Self {
        Self {
            code: code.into(),
            cat: "sys".into(),
            op: None,
            retryable: false,
            retry_after_s: None,
            fix: vec!["report".into()],
            message: Some(message.into()),
            trace_id: None,
            details: None,
        }
    }

    /// Timeout error
    pub fn timeout(code: &str, message: &str, retry_after: u32) -> Self {
        Self {
            code: code.into(),
            cat: "time".into(),
            op: None,
            retryable: true,
            retry_after_s: Some(retry_after),
            fix: vec!["wait".into()],
            message: Some(message.into()),
            trace_id: None,
            details: None,
        }
    }

    /// Add operation context
    pub fn with_op(mut self, op: &str) -> Self {
        self.op = Some(op.into());
        self
    }

    /// Add details
    pub fn with_details<T: Serialize>(mut self, details: T) -> Self {
        self.details = Some(serde_json::to_value(details).unwrap());
        self
    }

    /// Get exit code based on category
    pub fn exit_code(&self) -> i32 {
        match self.cat.as_str() {
            "in" => 1,
            "auth" => 3,
            "time" => 4,
            _ => 2,
        }
    }
}

/// Print manifest (--manifest) - Pebble Spec v1.1
pub fn print_manifest() {
    let manifest = json!({
        "schema_version": "1.0",
        "pebble": {
            "name": "cf",
            "display_name": "Cloudflare CLI",
            "version": env!("CARGO_PKG_VERSION"),
            "description": "Cloudflare infrastructure management for Claude Code",
            "homepage": "https://github.com/xxx/cf-cli"
        },
        "capabilities": {
            "agent": true,
            "interactive": false,
            "streaming": false,
            "resume": false
        },
        "actions": [
            {
                "id": "dns.list",
                "summary": "List DNS records",
                "args": [],
                "options": [
                    {"name": "domain", "short": "d", "type": "string", "required": true}
                ]
            },
            {
                "id": "dns.get",
                "summary": "Get DNS record",
                "args": [],
                "options": [
                    {"name": "domain", "short": "d", "type": "string", "required": true},
                    {"name": "name", "short": "n", "type": "string", "required": true}
                ]
            },
            {
                "id": "dns.create",
                "summary": "Create DNS A record",
                "args": [],
                "options": [
                    {"name": "domain", "short": "d", "type": "string", "required": true},
                    {"name": "name", "short": "n", "type": "string", "required": true},
                    {"name": "ip", "short": "i", "type": "string", "required": true},
                    {"name": "desc", "type": "string", "default": ""},
                    {"name": "proxied", "type": "bool", "default": false}
                ]
            },
            {
                "id": "dns.delete",
                "summary": "Delete DNS record",
                "args": [],
                "options": [
                    {"name": "domain", "short": "d", "type": "string", "required": true},
                    {"name": "name", "short": "n", "type": "string", "required": true}
                ]
            },
            {
                "id": "caddy.add",
                "summary": "Add Caddy reverse proxy",
                "args": [],
                "options": [
                    {"name": "server", "short": "s", "type": "string", "required": true},
                    {"name": "domain", "short": "d", "type": "string", "required": true},
                    {"name": "upstream", "short": "u", "type": "string", "required": true}
                ]
            },
            {
                "id": "caddy.add-lb",
                "summary": "Add load balancer",
                "args": [],
                "options": [
                    {"name": "server", "short": "s", "type": "string", "required": true},
                    {"name": "domain", "short": "d", "type": "string", "required": true},
                    {"name": "upstreams", "short": "u", "type": "string", "required": true},
                    {"name": "health_uri", "type": "string", "default": "/health"}
                ]
            },
            {
                "id": "caddy.reload",
                "summary": "Reload Caddy",
                "args": [],
                "options": [
                    {"name": "server", "short": "s", "type": "string", "required": true}
                ]
            },
            {
                "id": "service.check",
                "summary": "Check port listening",
                "args": [],
                "options": [
                    {"name": "server", "short": "s", "type": "string", "required": true},
                    {"name": "port", "short": "p", "type": "integer", "required": true}
                ]
            },
            {
                "id": "service.health",
                "summary": "Health check URL",
                "args": [],
                "options": [
                    {"name": "url", "short": "u", "type": "string", "required": true},
                    {"name": "timeout", "short": "t", "type": "integer", "default": 10}
                ]
            },
            {
                "id": "registry.stats",
                "summary": "Show registry statistics",
                "args": [],
                "options": []
            },
            {
                "id": "registry.validate",
                "summary": "Validate registry.json",
                "args": [],
                "options": []
            }
        ],
        "permissions": {
            "network": true,
            "network_domains": ["api.cloudflare.com"],
            "filesystem": {
                "read": ["$CF_PROJECT_DIR"],
                "write": ["$CF_PROJECT_DIR/registry.json"]
            },
            "env_vars": [
                "CLOUDFLARE_TAP365_API_TOKEN",
                "CLOUDFLARE_TAP365_ZONE_ID",
                "CLOUDFLARE_AIHANG365_API_TOKEN",
                "CLOUDFLARE_AIHANG365_ZONE_ID"
            ]
        },
        "limits": {
            "default_timeout_s": 60,
            "max_output_mb": 10
        }
    });

    println!("{}", serde_json::to_string_pretty(&manifest).unwrap());
}
