//! Registry.json management commands

use anyhow::Result;
use serde_json::json;

use crate::cli::{RegistryCommand, RegistryAction};
use crate::config::{self, DnsRecord};
use crate::output::{Output, PebbleError};

pub async fn run(cmd: RegistryCommand, out: &Output) -> Result<()> {
    match cmd.action {
        RegistryAction::Validate => validate(out).await,
        RegistryAction::Add { domain, name, ip, desc } => {
            add(&domain, &name, &ip, &desc, out).await
        }
        RegistryAction::Stats => stats(out).await,
    }
}

async fn validate(out: &Output) -> Result<()> {
    out.log("info", "Validating registry.json");

    match config::load_registry() {
        Ok(registry) => {
            let mut total_records = 0;
            let mut issues: Vec<String> = Vec::new();

            for (domain, config) in &registry.domains {
                let records: Vec<_> = config.records.iter()
                    .filter(|r| r.record_type != "comment")
                    .collect();
                total_records += records.len();

                // Check for duplicate names
                let mut seen_names: std::collections::HashSet<String> = std::collections::HashSet::new();
                for record in &records {
                    if !seen_names.insert(record.name.clone()) {
                        issues.push(format!("Duplicate record: {}.{}", record.name, domain));
                    }
                }
            }

            out.result(json!({
                "success": true,
                "valid": issues.is_empty(),
                "domains": registry.domains.len(),
                "records": total_records,
                "servers": registry.servers.len(),
                "issues": issues
            }));
        }
        Err(e) => {
            out.error(PebbleError::input("PARSE_FAIL", &format!("Failed to parse registry.json: {}", e))
                .with_op("registry.validate"));
        }
    }

    Ok(())
}

async fn add(domain: &str, name: &str, ip: &str, desc: &str, out: &Output) -> Result<()> {
    out.log("info", &format!("Adding to registry: {}.{} -> {}", name, domain, ip));

    let mut registry = match config::load_registry() {
        Ok(r) => r,
        Err(e) => {
            out.error(PebbleError::input("PARSE_FAIL", &format!("Failed to load registry.json: {}", e))
                .with_op("registry.add"));
            return Ok(());
        }
    };

    // Find the domain
    let domain_config = match registry.domains.get_mut(domain) {
        Some(c) => c,
        None => {
            out.error(PebbleError::input("NOT_FOUND", &format!("Domain not found in registry: {}", domain))
                .with_op("registry.add")
                .with_details(json!({"available_domains": registry.domains.keys().collect::<Vec<_>>()})));
            return Ok(());
        }
    };

    // Check if record already exists
    let exists = domain_config.records.iter()
        .any(|r| r.name == name && r.record_type == "A");

    if exists {
        out.error(PebbleError::input("RECORD_EXISTS", &format!("Record already exists: {}.{}", name, domain))
            .with_op("registry.add")
            .with_details(json!({"name": name, "domain": domain})));
        return Ok(());
    }

    // Add new record
    let new_record = DnsRecord {
        record_type: "A".to_string(),
        name: name.to_string(),
        content: ip.to_string(),
        desc: desc.to_string(),
    };

    domain_config.records.push(new_record);

    // Update timestamp
    registry.updated = chrono_lite_date();

    // Save
    match config::save_registry(&registry) {
        Ok(_) => {
            out.result(json!({
                "success": true,
                "domain": domain,
                "name": name,
                "ip": ip,
                "desc": desc,
                "path": config::project_dir().join("registry.json")
            }));
        }
        Err(e) => {
            out.error(PebbleError::sys("WRITE_FAIL", &format!("Failed to save registry.json: {}", e))
                .with_op("registry.add"));
        }
    }

    Ok(())
}

async fn stats(out: &Output) -> Result<()> {
    out.log("info", "Computing registry statistics");

    match config::load_registry() {
        Ok(registry) => {
            let mut domain_stats = Vec::new();

            for (domain, config) in &registry.domains {
                let records: Vec<_> = config.records.iter()
                    .filter(|r| r.record_type != "comment")
                    .collect();

                let a_count = records.iter().filter(|r| r.record_type == "A").count();
                let cname_count = records.iter().filter(|r| r.record_type == "CNAME").count();
                let other_count = records.len() - a_count - cname_count;

                domain_stats.push(json!({
                    "domain": domain,
                    "total": records.len(),
                    "a_records": a_count,
                    "cname_records": cname_count,
                    "other": other_count
                }));
            }

            let total_records: usize = domain_stats.iter()
                .map(|d| d["total"].as_u64().unwrap_or(0) as usize)
                .sum();

            out.result(json!({
                "success": true,
                "version": registry.version,
                "updated": registry.updated,
                "domains": domain_stats,
                "servers": registry.servers,
                "total_records": total_records,
                "total_servers": registry.servers.len()
            }));
        }
        Err(e) => {
            out.error(PebbleError::input("PARSE_FAIL", &format!("Failed to load registry.json: {}", e))
                .with_op("registry.stats"));
        }
    }

    Ok(())
}

/// Simple date function (avoid chrono dependency)
fn chrono_lite_date() -> String {
    use std::process::Command;
    Command::new("date")
        .arg("+%Y-%m-%d")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "unknown".to_string())
}
