//! DNS management commands

use anyhow::{Result, Context};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::cli::{DnsCommand, DnsAction};
use crate::config;
use crate::output::{Output, PebbleError};

const CF_API_BASE: &str = "https://api.cloudflare.com/client/v4";

pub async fn run(cmd: DnsCommand, out: &Output) -> Result<()> {
    config::load_env()?;

    match cmd.action {
        DnsAction::List { domain } => list(&domain, out).await,
        DnsAction::Get { domain, name } => get(&domain, &name, out).await,
        DnsAction::Create { domain, name, ip, desc, proxied } => {
            create(&domain, &name, &ip, &desc, proxied, out).await
        }
        DnsAction::Delete { domain, name } => delete(&domain, &name, out).await,
    }
}

// ============ Cloudflare API Types ============

#[derive(Debug, Deserialize)]
struct CfResponse<T> {
    success: bool,
    result: Option<T>,
    errors: Vec<CfError>,
}

#[derive(Debug, Deserialize)]
struct CfError {
    code: i32,
    message: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct CfDnsRecord {
    id: String,
    #[serde(rename = "type")]
    record_type: String,
    name: String,
    content: String,
    proxied: bool,
    ttl: u32,
}

// ============ Commands ============

async fn list(domain: &str, out: &Output) -> Result<()> {
    let (zone_id, api_token) = config::get_cf_credentials(domain)?;

    out.log("info", &format!("Fetching DNS records for {}", domain));

    let client = reqwest::Client::new();
    let url = format!("{}/zones/{}/dns_records", CF_API_BASE, zone_id);

    let resp = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", api_token))
        .header("Content-Type", "application/json")
        .send()
        .await
        .context("Failed to connect to Cloudflare API")?;

    let cf_resp: CfResponse<Vec<CfDnsRecord>> = resp.json().await
        .context("Failed to parse Cloudflare response")?;

    if !cf_resp.success {
        let err_msg = cf_resp.errors.iter()
            .map(|e| format!("[{}] {}", e.code, e.message))
            .collect::<Vec<_>>()
            .join(", ");
        out.error(PebbleError::ext("CF_API_ERROR", &err_msg)
            .with_op("dns.list")
            .with_details(json!({"domain": domain})));
        return Ok(());
    }

    let records = cf_resp.result.unwrap_or_default();

    out.result(json!({
        "success": true,
        "domain": domain,
        "count": records.len(),
        "records": records.iter().map(|r| json!({
            "id": r.id,
            "type": r.record_type,
            "name": r.name,
            "content": r.content,
            "proxied": r.proxied
        })).collect::<Vec<_>>()
    }));

    Ok(())
}

async fn get(domain: &str, name: &str, out: &Output) -> Result<()> {
    let (zone_id, api_token) = config::get_cf_credentials(domain)?;

    let fqdn = if name == "@" || name == domain {
        domain.to_string()
    } else {
        format!("{}.{}", name, domain)
    };

    out.log("info", &format!("Looking up DNS record: {}", fqdn));

    let client = reqwest::Client::new();
    let url = format!("{}/zones/{}/dns_records?name={}", CF_API_BASE, zone_id, fqdn);

    let resp = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", api_token))
        .header("Content-Type", "application/json")
        .send()
        .await
        .context("Failed to connect to Cloudflare API")?;

    let cf_resp: CfResponse<Vec<CfDnsRecord>> = resp.json().await
        .context("Failed to parse Cloudflare response")?;

    if !cf_resp.success {
        let err_msg = cf_resp.errors.iter()
            .map(|e| format!("[{}] {}", e.code, e.message))
            .collect::<Vec<_>>()
            .join(", ");
        out.error(PebbleError::ext("CF_API_ERROR", &err_msg)
            .with_op("dns.get")
            .with_details(json!({"domain": domain})));
        return Ok(());
    }

    let records = cf_resp.result.unwrap_or_default();

    if records.is_empty() {
        out.result(json!({
            "success": true,
            "exists": false,
            "fqdn": fqdn
        }));
    } else {
        let r = &records[0];
        out.result(json!({
            "success": true,
            "exists": true,
            "fqdn": fqdn,
            "record": {
                "id": r.id,
                "type": r.record_type,
                "name": r.name,
                "content": r.content,
                "proxied": r.proxied
            }
        }));
    }

    Ok(())
}

async fn create(domain: &str, name: &str, ip: &str, desc: &str, proxied: bool, out: &Output) -> Result<()> {
    let (zone_id, api_token) = config::get_cf_credentials(domain)?;

    let fqdn = if name == "@" || name == domain {
        domain.to_string()
    } else {
        format!("{}.{}", name, domain)
    };

    out.log("info", &format!("Creating DNS A record: {} -> {}", fqdn, ip));

    let client = reqwest::Client::new();
    let url = format!("{}/zones/{}/dns_records", CF_API_BASE, zone_id);

    let body = json!({
        "type": "A",
        "name": name,
        "content": ip,
        "ttl": 1,  // Auto
        "proxied": proxied,
        "comment": desc
    });

    let resp = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", api_token))
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .context("Failed to connect to Cloudflare API")?;

    let cf_resp: CfResponse<CfDnsRecord> = resp.json().await
        .context("Failed to parse Cloudflare response")?;

    if !cf_resp.success {
        let err_msg = cf_resp.errors.iter()
            .map(|e| format!("[{}] {}", e.code, e.message))
            .collect::<Vec<_>>()
            .join(", ");

        // Check if record already exists
        if err_msg.contains("already exists") || cf_resp.errors.iter().any(|e| e.code == 81057) {
            out.error(PebbleError::input("RECORD_EXISTS", "DNS record already exists")
                .with_op("dns.create")
                .with_details(json!({"fqdn": fqdn, "ip": ip})));
        } else {
            out.error(PebbleError::ext("CF_API_ERROR", &err_msg)
                .with_op("dns.create")
                .with_details(json!({"domain": domain})));
        }
        return Ok(());
    }

    let record = cf_resp.result.unwrap();

    out.result(json!({
        "success": true,
        "record_id": record.id,
        "fqdn": fqdn,
        "ip": ip,
        "proxied": proxied
    }));

    Ok(())
}

async fn delete(domain: &str, name: &str, out: &Output) -> Result<()> {
    let (zone_id, api_token) = config::get_cf_credentials(domain)?;

    let fqdn = if name == "@" || name == domain {
        domain.to_string()
    } else {
        format!("{}.{}", name, domain)
    };

    out.log("info", &format!("Deleting DNS record: {}", fqdn));

    // First, find the record ID
    let client = reqwest::Client::new();
    let search_url = format!("{}/zones/{}/dns_records?name={}", CF_API_BASE, zone_id, fqdn);

    let search_resp = client
        .get(&search_url)
        .header("Authorization", format!("Bearer {}", api_token))
        .header("Content-Type", "application/json")
        .send()
        .await
        .context("Failed to connect to Cloudflare API")?;

    let search_result: CfResponse<Vec<CfDnsRecord>> = search_resp.json().await
        .context("Failed to parse Cloudflare response")?;

    let records = search_result.result.unwrap_or_default();

    if records.is_empty() {
        out.result(json!({
            "success": true,
            "deleted": false,
            "fqdn": fqdn,
            "message": "Record not found"
        }));
        return Ok(());
    }

    // Delete the record
    let record_id = &records[0].id;
    let delete_url = format!("{}/zones/{}/dns_records/{}", CF_API_BASE, zone_id, record_id);

    let delete_resp = client
        .delete(&delete_url)
        .header("Authorization", format!("Bearer {}", api_token))
        .header("Content-Type", "application/json")
        .send()
        .await
        .context("Failed to connect to Cloudflare API")?;

    let delete_result: CfResponse<serde_json::Value> = delete_resp.json().await
        .context("Failed to parse Cloudflare response")?;

    if !delete_result.success {
        let err_msg = delete_result.errors.iter()
            .map(|e| format!("[{}] {}", e.code, e.message))
            .collect::<Vec<_>>()
            .join(", ");
        out.error(PebbleError::ext("CF_API_ERROR", &err_msg)
            .with_op("dns.delete")
            .with_details(json!({"domain": domain})));
        return Ok(());
    }

    out.result(json!({
        "success": true,
        "deleted": true,
        "fqdn": fqdn,
        "record_id": record_id
    }));

    Ok(())
}
