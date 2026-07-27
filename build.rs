use std::process::Command;

fn main() {
    let version = git_describe().unwrap_or_else(|| env!("CARGO_PKG_VERSION").to_string());
    println!("cargo:rustc-env=IMMICHSYNC_VERSION={version}");
}

/// `git describe` gives an exact tag name when built at a tagged release
/// commit (what CI does), or a `<tag>-<n>-g<hash>[-dirty]` string for local
/// dev builds between tags — falls back to Cargo.toml's static version if
/// there's no git history available at all (e.g. building from a source
/// tarball with no `.git` directory).
fn git_describe() -> Option<String> {
    let output = Command::new("git")
        .args(["describe", "--tags", "--always", "--dirty"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8(output.stdout).ok()?;
    let text = text.trim();
    if text.is_empty() {
        None
    } else {
        Some(text.to_string())
    }
}
