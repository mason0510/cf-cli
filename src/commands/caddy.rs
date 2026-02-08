//! Caddy management commands (via SSH)

use anyhow::{Result, Context};
use serde_json::json;
use std::process::Command;

use crate::cli::{CaddyCommand, CaddyAction};
use crate::output::{Output, PebbleError};

pub async fn run(cmd: CaddyCommand, out: &Output) -> Result<()> {
    match cmd.action {
        CaddyAction::Add { server, domain, upstream } => {
            add(&server, &domain, &upstream, out).await
        }
        CaddyAction::AddLb { server, domain, upstreams, health_uri } => {
            add_lb(&server, &domain, &upstreams, &health_uri, out).await
        }
        CaddyAction::Reload { server } => reload(&server, out).await,
        CaddyAction::Validate { server } => validate(&server, out).await,
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

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("SSH command failed: {}", stderr);
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

async fn add(server: &str, domain: &str, upstream: &str, out: &Output) -> Result<()> {
    out.log("info", &format!("Adding Caddy reverse proxy: {} -> {}", domain, upstream));

    // Generate Caddy config block
    let config = format!(r#"
{} {{
    reverse_proxy {}
}}
"#, domain, upstream);

    // Append to Caddyfile
    let cmd = format!(
        r#"cat >> /etc/caddy/Caddyfile << 'EOF'
{}
EOF"#,
        config.trim()
    );

    match ssh_exec(server, &cmd) {
        Ok(_) => {
            out.log("info", "Caddy configuration added");

            // Validate config
            match ssh_exec(server, "caddy validate --config /etc/caddy/Caddyfile") {
                Ok(_) => {
                    out.result(json!({
                        "success": true,
                        "server": server,
                        "domain": domain,
                        "upstream": upstream,
                        "message": "Configuration added. Run 'cf caddy reload' to apply."
                    }));
                }
                Err(e) => {
                    out.error(PebbleError::ext("CADDY_INVALID", &format!("Caddy config validation failed: {}", e))
                        .with_op("caddy.add")
                        .with_details(json!({"server": server, "domain": domain})));
                }
            }
        }
        Err(e) => {
            out.error(PebbleError::net("SSH_FAILED", &format!("Failed to add Caddy config: {}", e))
                .with_op("caddy.add")
                .with_details(json!({"server": server})));
        }
    }

    Ok(())
}

async fn add_lb(server: &str, domain: &str, upstreams: &str, health_uri: &str, out: &Output) -> Result<()> {
    let upstream_list: Vec<&str> = upstreams.split(',').map(|s| s.trim()).collect();

    out.log("info", &format!("Adding Caddy load balancer: {} -> {:?}", domain, upstream_list));

    // Generate Caddy config block with load balancing
    let upstream_str = upstream_list.join(" ");
    let config = format!(r#"
{} {{
    reverse_proxy {} {{
        lb_policy round_robin
        health_uri {}
        health_interval 30s
    }}
}}
"#, domain, upstream_str, health_uri);

    // Append to Caddyfile
    let cmd = format!(
        r#"cat >> /etc/caddy/Caddyfile << 'EOF'
{}
EOF"#,
        config.trim()
    );

    match ssh_exec(server, &cmd) {
        Ok(_) => {
            out.log("info", "Load balancer configuration added");

            // Validate config
            match ssh_exec(server, "caddy validate --config /etc/caddy/Caddyfile") {
                Ok(_) => {
                    out.result(json!({
                        "success": true,
                        "server": server,
                        "domain": domain,
                        "upstreams": upstream_list,
                        "health_uri": health_uri,
                        "message": "Load balancer configured. Run 'cf caddy reload' to apply."
                    }));
                }
                Err(e) => {
                    out.error(PebbleError::ext("CADDY_INVALID", &format!("Caddy config validation failed: {}", e))
                        .with_op("caddy.add-lb")
                        .with_details(json!({"server": server, "domain": domain})));
                }
            }
        }
        Err(e) => {
            out.error(PebbleError::net("SSH_FAILED", &format!("Failed to add load balancer config: {}", e))
                .with_op("caddy.add-lb")
                .with_details(json!({"server": server})));
        }
    }

    Ok(())
}

async fn reload(server: &str, out: &Output) -> Result<()> {
    out.log("info", &format!("Reloading Caddy on {}", server));

    match ssh_exec(server, "systemctl reload caddy") {
        Ok(_) => {
            out.result(json!({
                "success": true,
                "server": server,
                "message": "Caddy reloaded successfully"
            }));
        }
        Err(e) => {
            out.error(PebbleError::ext("CADDY_RELOAD_FAILED", &format!("Failed to reload Caddy: {}", e))
                .with_op("caddy.reload")
                .with_details(json!({"server": server})));
        }
    }

    Ok(())
}

async fn validate(server: &str, out: &Output) -> Result<()> {
    out.log("info", &format!("Validating Caddy config on {}", server));

    match ssh_exec(server, "caddy validate --config /etc/caddy/Caddyfile") {
        Ok(output) => {
            out.result(json!({
                "success": true,
                "valid": true,
                "server": server,
                "output": output.trim()
            }));
        }
        Err(e) => {
            out.result(json!({
                "success": true,
                "valid": false,
                "server": server,
                "error": e.to_string()
            }));
        }
    }

    Ok(())
}
