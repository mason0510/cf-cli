//! R2 storage commands

use anyhow::{Result, Context};
use s3::creds::Credentials;
use s3::bucket::Bucket;
use s3::Region;
use serde_json::json;
use std::path::Path;

use crate::cli::{R2Command, R2Action};
use crate::output::{Output, PebbleError};

pub async fn run(cmd: R2Command, out: &Output) -> Result<()> {
    match cmd.action {
        R2Action::Upload { file, key, public } => upload(&file, key.as_deref(), public, out).await,
        R2Action::List { prefix, limit } => list(&prefix, limit, out).await,
        R2Action::Delete { key } => delete(&key, out).await,
        R2Action::Info { key } => info(&key, out).await,
    }
}

/// Get R2 bucket configuration from environment
fn get_bucket() -> Result<Bucket> {
    let bucket_name = std::env::var("CLOUDFLARE_R2_BUCKET_NAME")
        .context("CLOUDFLARE_R2_BUCKET_NAME not set")?;

    let endpoint = std::env::var("CLOUDFLARE_R2_S3_API_URL")
        .context("CLOUDFLARE_R2_S3_API_URL not set")?;

    let access_key = std::env::var("CLOUDFLARE_R2_ACCESS_KEY_ID")
        .context("CLOUDFLARE_R2_ACCESS_KEY_ID not set")?;

    let secret_key = std::env::var("CLOUDFLARE_R2_SECRET_ACCESS_KEY")
        .context("CLOUDFLARE_R2_SECRET_ACCESS_KEY not set")?;

    let region = Region::Custom {
        region: "auto".to_string(),
        endpoint,
    };

    let credentials = Credentials::new(
        Some(&access_key),
        Some(&secret_key),
        None,
        None,
        None,
    )?;

    let bucket = Bucket::new(&bucket_name, region, credentials)?
        .with_path_style();

    Ok(bucket)
}

/// Get public URL base from environment
fn get_public_url() -> String {
    std::env::var("CLOUDFLARE_R2_PUBLIC_URL")
        .unwrap_or_else(|_| "https://pub-87cd59069cf0444aad048f7bddec99af.r2.dev".to_string())
}

/// Get folder prefix from environment
fn get_folder_prefix() -> String {
    std::env::var("CLOUDFLARE_R2_FOLDER_PREFIX")
        .unwrap_or_else(|_| "uploads/".to_string())
}

async fn upload(file_path: &str, custom_key: Option<&str>, public: bool, out: &Output) -> Result<()> {
    out.log("info", &format!("Uploading file: {}", file_path));

    let path = Path::new(file_path);

    // Check file exists
    if !path.exists() {
        out.error(PebbleError::input("FILE_NOT_FOUND", &format!("File not found: {}", file_path))
            .with_op("r2.upload"));
        return Ok(());
    }

    // Read file
    let content = tokio::fs::read(path).await
        .context("Failed to read file")?;

    let file_size = content.len();

    // Determine object key
    let key = match custom_key {
        Some(k) => k.to_string(),
        None => {
            let filename = path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown");
            format!("{}{}", get_folder_prefix(), filename)
        }
    };

    // Guess content type
    let content_type = mime_guess::from_path(path)
        .first_or_octet_stream()
        .to_string();

    out.log("info", &format!("Key: {}, Content-Type: {}, Size: {} bytes", key, content_type, file_size));

    // Get bucket
    let bucket = match get_bucket() {
        Ok(b) => b,
        Err(e) => {
            out.error(PebbleError::sys("CONFIG_ERROR", &format!("Failed to configure R2: {}", e))
                .with_op("r2.upload"));
            return Ok(());
        }
    };

    // Upload
    match bucket.put_object_with_content_type(&key, &content, &content_type).await {
        Ok(response) => {
            let status = response.status_code();

            if status == 200 {
                let public_url = if public {
                    format!("{}/{}", get_public_url(), key)
                } else {
                    String::new()
                };

                out.result(json!({
                    "success": true,
                    "key": key,
                    "size": file_size,
                    "content_type": content_type,
                    "public_url": public_url,
                    "bucket": bucket.name()
                }));
            } else {
                out.error(PebbleError::ext("UPLOAD_FAILED", &format!("Upload failed with status: {}", status))
                    .with_op("r2.upload")
                    .with_details(json!({"status": status, "key": key})));
            }
        }
        Err(e) => {
            out.error(PebbleError::net("UPLOAD_FAILED", &format!("Upload failed: {}", e))
                .with_op("r2.upload")
                .with_details(json!({"file": file_path, "key": key})));
        }
    }

    Ok(())
}

async fn list(prefix: &str, limit: u32, out: &Output) -> Result<()> {
    out.log("info", &format!("Listing objects with prefix: '{}'", prefix));

    let bucket = match get_bucket() {
        Ok(b) => b,
        Err(e) => {
            out.error(PebbleError::sys("CONFIG_ERROR", &format!("Failed to configure R2: {}", e))
                .with_op("r2.list"));
            return Ok(());
        }
    };

    let prefix_str = if prefix.is_empty() { None } else { Some(prefix) };

    match bucket.list(prefix_str.unwrap_or("").to_string(), None).await {
        Ok(results) => {
            let mut objects: Vec<serde_json::Value> = Vec::new();

            for result in results {
                for obj in result.contents {
                    if objects.len() >= limit as usize {
                        break;
                    }
                    objects.push(json!({
                        "key": obj.key,
                        "size": obj.size,
                        "last_modified": obj.last_modified,
                        "public_url": format!("{}/{}", get_public_url(), obj.key)
                    }));
                }
            }

            out.result(json!({
                "success": true,
                "count": objects.len(),
                "prefix": prefix,
                "objects": objects
            }));
        }
        Err(e) => {
            out.error(PebbleError::net("LIST_FAILED", &format!("Failed to list objects: {}", e))
                .with_op("r2.list"));
        }
    }

    Ok(())
}

async fn delete(key: &str, out: &Output) -> Result<()> {
    out.log("info", &format!("Deleting object: {}", key));

    let bucket = match get_bucket() {
        Ok(b) => b,
        Err(e) => {
            out.error(PebbleError::sys("CONFIG_ERROR", &format!("Failed to configure R2: {}", e))
                .with_op("r2.delete"));
            return Ok(());
        }
    };

    match bucket.delete_object(key).await {
        Ok(response) => {
            let status = response.status_code();

            if status == 204 || status == 200 {
                out.result(json!({
                    "success": true,
                    "deleted": true,
                    "key": key
                }));
            } else {
                out.error(PebbleError::ext("DELETE_FAILED", &format!("Delete failed with status: {}", status))
                    .with_op("r2.delete")
                    .with_details(json!({"status": status, "key": key})));
            }
        }
        Err(e) => {
            out.error(PebbleError::net("DELETE_FAILED", &format!("Failed to delete object: {}", e))
                .with_op("r2.delete")
                .with_details(json!({"key": key})));
        }
    }

    Ok(())
}

async fn info(key: &str, out: &Output) -> Result<()> {
    out.log("info", &format!("Getting info for: {}", key));

    let bucket = match get_bucket() {
        Ok(b) => b,
        Err(e) => {
            out.error(PebbleError::sys("CONFIG_ERROR", &format!("Failed to configure R2: {}", e))
                .with_op("r2.info"));
            return Ok(());
        }
    };

    match bucket.head_object(key).await {
        Ok((head, status)) => {
            if status == 200 {
                out.result(json!({
                    "success": true,
                    "key": key,
                    "exists": true,
                    "size": head.content_length,
                    "content_type": head.content_type.unwrap_or_default(),
                    "last_modified": head.last_modified.unwrap_or_default(),
                    "etag": head.e_tag.unwrap_or_default(),
                    "public_url": format!("{}/{}", get_public_url(), key)
                }));
            } else {
                out.result(json!({
                    "success": true,
                    "key": key,
                    "exists": false
                }));
            }
        }
        Err(e) => {
            // 404 means not found
            let err_str = e.to_string();
            if err_str.contains("404") || err_str.contains("NoSuchKey") {
                out.result(json!({
                    "success": true,
                    "key": key,
                    "exists": false
                }));
            } else {
                out.error(PebbleError::net("INFO_FAILED", &format!("Failed to get object info: {}", e))
                    .with_op("r2.info")
                    .with_details(json!({"key": key})));
            }
        }
    }

    Ok(())
}
