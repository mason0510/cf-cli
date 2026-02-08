//! Service check commands

use anyhow::{Result, Context};
use serde_json::json;
use std::process::Command;
use std::time::{Duration, Instant};

use crate::cli::{ServiceCommand, ServiceAction};
use crate::output::{Output, PebbleError};

pub async fn run(cmd: ServiceCommand, out: &Output) -> Result<()> {
    match cmd.action {
        ServiceAction::Check { server, port } => check(&server, port, out).await,
        ServiceAction::Health { url, timeout } => health(&url, timeout, out).await,
        ServiceAction::DockerPs { server } => docker_ps(&server, out).await,
        ServiceAction::Pm2List { server } => pm2_list(&server, out).await,
    }
}

/// Execute SSH command and return output
fn ssh_exec(server: &str, cmd: &str) -> Result<String> {
    let output = Command::new("ssh")
        .args(["-o", "StrictHostKeyChecking=no", "-o", "ConnectTimeout=10"])
        .arg(format!("root@{}", server))
        .arg(cmd)
        .output()
        .context("Failed to execute SSH command")?;

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

async fn check(server: &str, port: u16, out: &Output) -> Result<()> {
    out.log("info", &format!("Checking port {} on {}", port, server));

    // Use ss to check if port is listening
    let cmd = format!("ss -tlnp | grep ':{}'", port);

    match ssh_exec(server, &cmd) {
        Ok(output) => {
            let listening = !output.trim().is_empty();

            if listening {
                // Try to extract process name
                let process = output
                    .split("users:")
                    .nth(1)
                    .and_then(|s| s.split('"').nth(1))
                    .unwrap_or("unknown")
                    .to_string();

                out.result(json!({
                    "success": true,
                    "server": server,
                    "port": port,
                    "listening": true,
                    "process": process
                }));
            } else {
                out.result(json!({
                    "success": true,
                    "server": server,
                    "port": port,
                    "listening": false
                }));
            }
        }
        Err(e) => {
            out.error(PebbleError::net("SSH_FAILED", &format!("Failed to check port: {}", e))
                .with_op("service.check")
                .with_details(json!({"server": server, "port": port})));
        }
    }

    Ok(())
}

async fn health(url: &str, timeout_secs: u64, out: &Output) -> Result<()> {
    out.log("info", &format!("Health check: {}", url));

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(timeout_secs))
        .danger_accept_invalid_certs(true)
        .build()
        .context("Failed to create HTTP client")?;

    let start = Instant::now();

    match client.get(url).send().await {
        Ok(resp) => {
            let elapsed = start.elapsed();
            let status = resp.status().as_u16();
            let healthy = status >= 200 && status < 400;

            // Try to get response body
            let body = resp.text().await.unwrap_or_default();
            let body_preview = if body.len() > 200 {
                format!("{}...", &body[..200])
            } else {
                body
            };

            out.result(json!({
                "success": true,
                "url": url,
                "healthy": healthy,
                "status": status,
                "response_time_ms": elapsed.as_millis(),
                "body_preview": body_preview
            }));
        }
        Err(e) => {
            let elapsed = start.elapsed();

            if e.is_timeout() {
                out.error(PebbleError::timeout("TIMEOUT", &format!("Health check timed out after {}s", timeout_secs), timeout_secs as u32)
                    .with_op("service.health")
                    .with_details(json!({"url": url})));
            } else if e.is_connect() {
                out.error(PebbleError::net("CONNECT_FAILED", &format!("Connection failed: {}", e))
                    .with_op("service.health")
                    .with_details(json!({"url": url})));
            } else {
                out.result(json!({
                    "success": true,
                    "url": url,
                    "healthy": false,
                    "error": e.to_string(),
                    "response_time_ms": elapsed.as_millis()
                }));
            }
        }
    }

    Ok(())
}

async fn docker_ps(server: &str, out: &Output) -> Result<()> {
    out.log("info", &format!("Listing Docker containers on {}", server));

    // Get container info in JSON format
    let cmd = r#"docker ps --format '{"name":"{{.Names}}","image":"{{.Image}}","status":"{{.Status}}","ports":"{{.Ports}}"}'"#;

    match ssh_exec(server, cmd) {
        Ok(output) => {
            let containers: Vec<serde_json::Value> = output
                .lines()
                .filter(|line| !line.trim().is_empty())
                .filter_map(|line| serde_json::from_str(line).ok())
                .collect();

            out.result(json!({
                "success": true,
                "server": server,
                "count": containers.len(),
                "containers": containers
            }));
        }
        Err(e) => {
            out.error(PebbleError::net("SSH_FAILED", &format!("Failed to list Docker containers: {}", e))
                .with_op("service.docker-ps")
                .with_details(json!({"server": server})));
        }
    }

    Ok(())
}

async fn pm2_list(server: &str, out: &Output) -> Result<()> {
    out.log("info", &format!("Listing PM2 processes on {}", server));

    let cmd = "pm2 jlist 2>/dev/null || echo '[]'";

    match ssh_exec(server, cmd) {
        Ok(output) => {
            let processes: Vec<serde_json::Value> = serde_json::from_str(&output)
                .unwrap_or_default();

            let summary: Vec<serde_json::Value> = processes
                .iter()
                .map(|p| {
                    json!({
                        "name": p.get("name").and_then(|v| v.as_str()).unwrap_or("unknown"),
                        "status": p.get("pm2_env").and_then(|e| e.get("status")).and_then(|v| v.as_str()).unwrap_or("unknown"),
                        "pid": p.get("pid").and_then(|v| v.as_i64()).unwrap_or(0),
                        "memory": p.get("monit").and_then(|m| m.get("memory")).and_then(|v| v.as_i64()).unwrap_or(0),
                        "cpu": p.get("monit").and_then(|m| m.get("cpu")).and_then(|v| v.as_f64()).unwrap_or(0.0)
                    })
                })
                .collect();

            out.result(json!({
                "success": true,
                "server": server,
                "count": summary.len(),
                "processes": summary
            }));
        }
        Err(e) => {
            out.error(PebbleError::net("SSH_FAILED", &format!("Failed to list PM2 processes: {}", e))
                .with_op("service.pm2-list")
                .with_details(json!({"server": server})));
        }
    }

    Ok(())
}
