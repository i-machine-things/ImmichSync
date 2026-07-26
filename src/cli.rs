use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "immichsync",
    version,
    about = "Nightly photo backup for Immich"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Interactive setup: server URL, API key, photos directories.
    Init,
    /// Run a sync pass now.
    Run {
        /// Scan and report what would be uploaded, without uploading anything.
        #[arg(long)]
        dry_run: bool,
    },
    /// Show config location and last sync summary.
    Status,
    /// Manage the nightly scheduled task/timer.
    #[command(subcommand)]
    Service(ServiceCommand),
    /// Check GitHub releases for a newer version.
    #[command(subcommand)]
    Update(UpdateCommand),
}

#[derive(Subcommand)]
pub enum UpdateCommand {
    /// Check now and print the result.
    Check,
}

#[derive(Subcommand)]
pub enum ServiceCommand {
    /// Install a nightly schedule (systemd user timer on Linux, Task Scheduler on Windows).
    Install {
        /// Hour of day (0-23, local time) to run the nightly sync.
        #[arg(long, default_value_t = 2, value_parser = clap::value_parser!(u8).range(0..=23))]
        hour: u8,
    },
    /// Remove the nightly schedule.
    Uninstall,
}
