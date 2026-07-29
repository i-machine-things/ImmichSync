use anyhow::{Context, Result, bail};
use std::process::Command;

fn unit_dir() -> Result<std::path::PathBuf> {
    let home = std::env::var("HOME").context("HOME not set")?;
    Ok(std::path::PathBuf::from(home).join(".config/systemd/user"))
}

pub fn install(hour: u8) -> Result<()> {
    let exe = std::env::current_exe().context("locating current executable")?;
    let dir = unit_dir()?;
    std::fs::create_dir_all(&dir).with_context(|| format!("creating {}", dir.display()))?;

    remove_legacy_units(&dir);

    let service = format!(
        "[Unit]\nDescription=ImmichHaul nightly backup\n\n\
         [Service]\nType=oneshot\nExecStart={} run\n",
        exe.display()
    );
    let timer = format!(
        "[Unit]\nDescription=Run ImmichHaul nightly\n\n\
         [Timer]\nOnCalendar=*-*-* {hour:02}:00:00\nPersistent=true\n\n\
         [Install]\nWantedBy=timers.target\n"
    );

    std::fs::write(dir.join("immich-haul.service"), service)?;
    std::fs::write(dir.join("immich-haul.timer"), timer)?;

    run_systemctl(&["daemon-reload"])?;
    run_systemctl(&["enable", "--now", "immich-haul.timer"])?;

    println!("Installed systemd user timer: runs daily at {hour:02}:00.");

    match linger_enabled() {
        Some(true) => {}
        _ => {
            println!(
                "Note: the timer only fires while you're logged in unless linger is enabled.\n\
                 Run this once to allow it to run in the background: loginctl enable-linger $USER"
            );
        }
    }

    Ok(())
}

pub fn uninstall() -> Result<()> {
    let dir = unit_dir()?;
    let _ = run_systemctl(&["disable", "--now", "immich-haul.timer"]);
    for name in ["immich-haul.timer", "immich-haul.service"] {
        let path = dir.join(name);
        if path.exists() {
            std::fs::remove_file(&path)?;
        }
    }
    remove_legacy_units(&dir);
    run_systemctl(&["daemon-reload"])?;
    println!("Removed the systemd user timer.");
    Ok(())
}

/// Best-effort removal of the pre-rename `immichsync` unit names, so
/// upgrading from an older ImmichSync install doesn't leave a stale timer
/// running alongside the new `immich-haul` one (different package name, so
/// apt/dpkg won't have removed it automatically).
fn remove_legacy_units(dir: &std::path::Path) {
    let _ = run_systemctl(&["disable", "--now", "immichsync.timer"]);
    for name in ["immichsync.timer", "immichsync.service"] {
        let _ = std::fs::remove_file(dir.join(name));
    }
}

fn run_systemctl(args: &[&str]) -> Result<()> {
    let status = Command::new("systemctl")
        .arg("--user")
        .args(args)
        .status()
        .context("running systemctl")?;
    if !status.success() {
        bail!("systemctl --user {} failed", args.join(" "));
    }
    Ok(())
}

fn linger_enabled() -> Option<bool> {
    let user = std::env::var("USER").ok()?;
    let output = Command::new("loginctl")
        .args(["show-user", &user, "--property=Linger"])
        .output()
        .ok()?;
    let text = String::from_utf8_lossy(&output.stdout);
    Some(text.trim() == "Linger=yes")
}
