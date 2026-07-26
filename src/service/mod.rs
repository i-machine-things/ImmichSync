#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
pub use linux::{install, uninstall};

#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "windows")]
pub use windows::{install, uninstall};

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
mod unsupported {
    use anyhow::{Result, bail};

    pub fn install(_hour: u8) -> Result<()> {
        bail!("nightly scheduling is only implemented for Linux and Windows")
    }

    pub fn uninstall() -> Result<()> {
        bail!("nightly scheduling is only implemented for Linux and Windows")
    }
}
#[cfg(not(any(target_os = "linux", target_os = "windows")))]
pub use unsupported::{install, uninstall};
