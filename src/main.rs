//! CF CLI - Cloudflare infrastructure management for Claude Code
//!
//! Pebble Spec v1.1 compliant

mod cli;
mod commands;
mod config;
mod output;

use clap::Parser;
use cli::{Cli, Commands};
use output::PebbleError;

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    // Handle --manifest before anything else
    if cli.manifest {
        output::print_manifest();
        return;
    }

    let out = output::Output::new(cli.agent);

    // If no command provided, show help
    let command = match cli.command {
        Some(cmd) => cmd,
        None => {
            eprintln!("Error: no command provided. Use --help for usage.");
            std::process::exit(1);
        }
    };

    let result = match command {
        Commands::Dns(cmd) => commands::dns::run(cmd, &out).await,
        Commands::Caddy(cmd) => commands::caddy::run(cmd, &out).await,
        Commands::Service(cmd) => commands::service::run(cmd, &out).await,
        Commands::Registry(cmd) => commands::registry::run(cmd, &out).await,
        Commands::R2(cmd) => commands::r2::run(cmd, &out).await,
    };

    if let Err(e) = result {
        out.error(PebbleError::sys("INTERNAL", &e.to_string()));
    }
}
