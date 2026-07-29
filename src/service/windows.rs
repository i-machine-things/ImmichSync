use anyhow::{Context, Result, bail};
use std::process::Command;

const TASK_NAME: &str = "ImmichHaul";
const LEGACY_TASK_NAME: &str = "ImmichSync";

pub fn install(hour: u8) -> Result<()> {
    let exe = std::env::current_exe().context("locating current executable")?;
    let start_time = format!("{hour:02}:00");
    let exe_str = exe.to_string_lossy().to_string();

    remove_legacy_task();

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

    // schtasks.exe has no CLI flag for "start when available", so a run
    // missed because the machine was off/asleep at the scheduled time would
    // just never happen that day. Set it as a follow-up step via PowerShell
    // (Set-ScheduledTask only touches the Settings we hand it — action,
    // trigger, and the run-as-user schtasks already configured are left
    // alone) so a missed run catches up next time the machine is on,
    // matching the systemd timer's Persistent=true on Linux. Doesn't wake
    // the machine — same as the Linux side.
    let ps_script = format!(
        "$ErrorActionPreference = 'Stop'; \
         $t = Get-ScheduledTask -TaskName '{TASK_NAME}' -ErrorAction Stop; \
         if ($t -eq $null) {{ throw 'task not found' }}; \
         $t.Settings.StartWhenAvailable = $true; \
         Set-ScheduledTask -InputObject $t -ErrorAction Stop | Out-Null"
    );
    let ps_status = Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", &ps_script])
        .status();
    match ps_status {
        Ok(s) if s.success() => {}
        _ => println!(
            "Note: couldn't set 'start when available' on the scheduled task — \
             it won't catch up automatically if the machine is off at {start_time}. \
             The task itself is still installed and will run normally otherwise."
        ),
    }

    println!("Installed Windows scheduled task '{TASK_NAME}': runs daily at {start_time}.");
    Ok(())
}

pub fn uninstall() -> Result<()> {
    remove_legacy_task();

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

/// Best-effort removal of the pre-rename "ImmichSync" scheduled task, so
/// upgrading from an older ImmichSync install doesn't leave two nightly
/// tasks registered at once.
fn remove_legacy_task() {
    let _ = Command::new("schtasks")
        .args(["/Delete", "/TN", LEGACY_TASK_NAME, "/F"])
        .status();
}
