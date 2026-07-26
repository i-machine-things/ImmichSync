use anyhow::{Context, Result, bail};
use std::process::Command;

const TASK_NAME: &str = "ImmichSync";

pub fn install(hour: u8) -> Result<()> {
    let exe = std::env::current_exe().context("locating current executable")?;
    let start_time = format!("{hour:02}:00");
    let exe_str = exe.to_string_lossy().to_string();

    let status = Command::new("schtasks")
        .args([
            "/Create",
            "/TN",
            TASK_NAME,
            "/TR",
            &format!("\"{exe_str}\" run"),
            "/SC",
            "DAILY",
            "/ST",
            &start_time,
            "/RL",
            "LIMITED",
            "/F",
        ])
        .status()
        .context("running schtasks /Create")?;

    if !status.success() {
        bail!("schtasks /Create failed");
    }
    println!("Installed Windows scheduled task '{TASK_NAME}': runs daily at {start_time}.");
    Ok(())
}

pub fn uninstall() -> Result<()> {
    let status = Command::new("schtasks")
        .args(["/Delete", "/TN", TASK_NAME, "/F"])
        .status()
        .context("running schtasks /Delete")?;
    if !status.success() {
        bail!("schtasks /Delete failed");
    }
    println!("Removed the '{TASK_NAME}' scheduled task.");
    Ok(())
}
