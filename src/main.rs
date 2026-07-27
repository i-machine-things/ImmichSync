mod cli;
mod config;
mod immich;
mod logging;
mod manifest;
mod scanner;
mod service;
mod sync;
mod update;

use anyhow::{Context, Result};
use clap::Parser;
use cli::{Cli, Command, ServiceCommand, UpdateCommand};
use config::Config;
use immich::ImmichClient;
use logging::Logger;
use std::io::{IsTerminal, Write};
use std::path::PathBuf;

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Init => cmd_init(),
        Command::Run { dry_run } => cmd_run(dry_run),
        Command::Status => cmd_status(),
        Command::Service(ServiceCommand::Install { hour }) => {
            ensure_config()?;
            service::install(hour)
        }
        Command::Service(ServiceCommand::Uninstall) => service::uninstall(),
        Command::Update(UpdateCommand::Check) => cmd_update_check(),
    }
}

/// Loads the config, running the interactive setup wizard first if none
/// exists yet and we're attached to a terminal. Unattended invocations (the
/// nightly scheduled task/timer) get a clear error instead of hanging on a
/// prompt with nowhere to type.
fn ensure_config() -> Result<Config> {
    if Config::exists()? {
        return Config::load();
    }
    if !std::io::stdin().is_terminal() {
        anyhow::bail!(
            "no config found at {} — run `immichsync init` (or any command) from a terminal to set up",
            config::config_dir()?.join("config.toml").display()
        );
    }
    println!("No configuration found yet — let's set it up.\n");
    run_setup_wizard()
}

fn cmd_update_check() -> Result<()> {
    match update::check_now()? {
        Some(info) => println!("Update available: {} ({})", info.tag, info.url),
        None => println!("Up to date ({}).", env!("IMMICHSYNC_VERSION")),
    }
    Ok(())
}

fn prompt(label: &str, default: Option<&str>) -> Result<String> {
    match default {
        Some(d) => print!("{label} [{d}]: "),
        None => print!("{label}: "),
    }
    std::io::stdout().flush()?;
    let mut line = String::new();
    std::io::stdin().read_line(&mut line)?;
    let trimmed = line.trim();
    if trimmed.is_empty() {
        Ok(default.unwrap_or("").to_string())
    } else {
        Ok(trimmed.to_string())
    }
}

/// Expands a leading `~` to the user's home directory. Good enough for the
/// paths this prompt will realistically see; not a general shell-expansion.
fn expand_tilde(path: &str) -> PathBuf {
    if let Some(rest) = path.strip_prefix('~')
        && let Ok(home) = std::env::var(if cfg!(windows) { "USERPROFILE" } else { "HOME" })
    {
        return PathBuf::from(home).join(rest.trim_start_matches('/').trim_start_matches('\\'));
    }
    PathBuf::from(path)
}

fn run_setup_wizard() -> Result<Config> {
    let default_pictures = dirs_pictures();
    let server_url = prompt("Immich server URL", Some("http://immich.home"))?;
    let api_key = rpassword::prompt_password("Immich API key: ")?;
    let photos_input = prompt("Photos directory to back up", default_pictures.as_deref())?;
    let photos_dir = expand_tilde(&photos_input);

    if !photos_dir.is_dir() {
        anyhow::bail!("{} is not a directory", photos_dir.display());
    }

    println!("\nChecking connection to {server_url} ...");
    let client = ImmichClient::new(&server_url, &api_key)?;
    client.ping().context("could not reach the Immich server")?;
    let email = client
        .validate_api_key()
        .context("could not validate the API key")?;
    println!("Connected as {email}.");

    let config = Config {
        server_url,
        api_key,
        photos_dirs: vec![photos_dir],
    };
    config.save()?;
    println!("\nSaved config to {}.", config::config_dir()?.display());
    Ok(config)
}

fn cmd_init() -> Result<()> {
    println!("ImmichSync setup\n");
    run_setup_wizard()?;
    println!(
        "Run `immichsync run --dry-run` to preview a sync, or `immichsync service install` \
         to schedule nightly backups."
    );
    Ok(())
}

fn cmd_run(dry_run: bool) -> Result<()> {
    let config = ensure_config()?;
    let log = Logger::to_file(&config::log_path()?)?;
    if dry_run {
        log.log("starting sync (dry-run)");
    } else {
        log.log("starting sync");
    }

    let manifest_path = config::manifest_path()?;
    let summary = sync::run(&config, &manifest_path, dry_run, &log)?;

    log.log(&format!(
        "done: scanned={} skipped_cached={} already_on_server={} uploaded={} failed={}",
        summary.scanned,
        summary.skipped_cached,
        summary.already_on_server,
        summary.uploaded,
        summary.failed
    ));

    if let Ok(cache_path) = config::update_cache_path()
        && let Some(info) = update::check_if_due(&cache_path)
    {
        log.log(&format!("update available: {} ({})", info.tag, info.url));
    }

    if summary.failed > 0 {
        std::process::exit(1);
    }
    Ok(())
}

fn cmd_status() -> Result<()> {
    let config_dir = config::config_dir()?;
    let log_path = config::log_path()?;
    println!("Config dir: {}", config_dir.display());
    println!("Log file:   {}", log_path.display());

    match ensure_config() {
        Ok(cfg) => {
            println!("Server:     {}", cfg.server_url);
            println!("Photos dirs:");
            for d in &cfg.photos_dirs {
                println!("  - {}", d.display());
            }
        }
        Err(e) => println!("Config not set up yet ({e}). Run `immichsync init`."),
    }

    if let Ok(text) = std::fs::read_to_string(&log_path) {
        println!("\nLast log lines:");
        for line in text
            .lines()
            .rev()
            .take(10)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
        {
            println!("  {line}");
        }
    }
    Ok(())
}

fn dirs_pictures() -> Option<String> {
    directories::UserDirs::new()
        .and_then(|u| u.picture_dir().map(|p| p.to_string_lossy().to_string()))
}
