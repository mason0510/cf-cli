//! Configuration loading (.env + registry.json)

use anyhow::{Result, Context};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Project directory (where .env and registry.json are)
pub fn project_dir() -> PathBuf {
    // Check CF_PROJECT_DIR env first
    if let Ok(dir) = std::env::var("CF_PROJECT_DIR") {
        return PathBuf::from(dir);
    }
    // Default to current directory
    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

/// Load .env file
pub fn load_env() -> Result<()> {
    let env_path = project_dir().join(".env");
    dotenvy::from_path(&env_path)
        .with_context(|| format!("Failed to load .env from {:?}", env_path))?;
    Ok(())
}

/// Get Cloudflare credentials for a domain
///
/// Converts domain name to environment variable names.
/// Example: example.com -> CLOUDFLARE_EXAMPLE_ZONE_ID, CLOUDFLARE_EXAMPLE_API_TOKEN
pub fn get_cf_credentials(domain: &str) -> Result<(String, String)> {
    // Convert domain to env var name slug (e.g., "example.com" -> "EXAMPLE")
    let domain_slug = domain
        .split('.')
        .next()
        .unwrap_or(domain)
        .to_uppercase();

    let zone_env = format!("CLOUDFLARE_{}_ZONE_ID", domain_slug);
    let token_env = format!("CLOUDFLARE_{}_API_TOKEN", domain_slug);

    let zone_id = std::env::var(&zone_env)
        .with_context(|| format!("Missing env var: {}. Add it to your .env file.", zone_env))?;
    let api_token = std::env::var(&token_env)
        .with_context(|| format!("Missing env var: {}. Add it to your .env file.", token_env))?;

    Ok((zone_id, api_token))
}

// ============ Registry Types ============

#[derive(Debug, Serialize, Deserialize)]
pub struct Registry {
    pub version: String,
    pub updated: String,
    pub domains: HashMap<String, DomainConfig>,
    pub servers: HashMap<String, ServerInfo>,
    #[serde(default)]
    pub tunnels: HashMap<String, String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DomainConfig {
    pub zone_id: String,
    pub records: Vec<DnsRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnsRecord {
    #[serde(rename = "type")]
    pub record_type: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub content: String,
    #[serde(default)]
    pub desc: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ServerInfo {
    pub location: String,
    pub name: String,
}

/// Load registry.json
pub fn load_registry() -> Result<Registry> {
    let path = project_dir().join("registry.json");
    let content = std::fs::read_to_string(&path)
        .with_context(|| format!("Failed to read registry.json from {:?}", path))?;
    let registry: Registry = serde_json::from_str(&content)
        .with_context(|| "Failed to parse registry.json")?;
    Ok(registry)
}

/// Save registry.json
pub fn save_registry(registry: &Registry) -> Result<()> {
    let path = project_dir().join("registry.json");
    let content = serde_json::to_string_pretty(registry)?;
    std::fs::write(&path, content)
        .with_context(|| format!("Failed to write registry.json to {:?}", path))?;
    Ok(())
}
