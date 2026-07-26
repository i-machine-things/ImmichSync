# ImmichSync

A nightly backup CLI (Rust) that uploads new photos/videos from a local
directory to a self-hosted [Immich](https://immich.app) server. Files already
present on the server (matched by SHA1 checksum, via Immich's
`bulk-upload-check` API) are skipped, so it's safe to point at a folder
that's already been uploaded manually — nothing gets re-uploaded.

## Install

**Linux** — install is handled entirely by `apt`/`dpkg`, no wrapper script:

```bash
curl -fsSLO https://github.com/i-machine-things/ImmichSync/releases/latest/download/immichsync-<version>-linux-amd64.deb
sudo apt install ./immichsync-<version>-linux-amd64.deb
```

**Windows**: download `ImmichSync-<version>-windows-setup.exe` from the
[latest release](https://github.com/i-machine-things/ImmichSync/releases/latest)
and run it. The installer offers to run setup and enable the nightly
schedule at the end.

Either way, there's no separate setup step to remember: the first time you
run any command (`immichsync run`, `immichsync status`, `immichsync service
install`) with no config yet and a terminal attached, it walks you through
setup (server URL, API key, photos directory) automatically before
continuing. Unattended invocations (the nightly timer/scheduled task) fail
with a clear error instead of hanging on a prompt if setup was never done.

## Usage

```bash
immichsync init              # (re-)run setup explicitly: server URL, API key, photos dir
immichsync run --dry-run     # preview a sync without uploading anything
immichsync run               # sync now (prompts for setup first if not configured yet)
immichsync status            # show config, log tail
immichsync service install   # schedule a nightly run (systemd user timer / Task Scheduler)
immichsync service uninstall # remove the nightly schedule
immichsync update check      # check GitHub for a newer release
```

Config lives at the OS-appropriate config dir (e.g. `~/.config/immichsync/config.toml`
on Linux, `%APPDATA%\immichsync\config.toml` on Windows) with `0600`
permissions on Unix, since it holds your Immich API key. Sync state (which
files are already uploaded) and logs live in the OS data dir alongside it.

## How it works

1. Recursively scans the configured photos directory (skipping hidden files
   and empty files).
2. For files whose size/mtime haven't changed since the last run, skips
   re-checking them entirely.
3. For everything else, computes a SHA1 checksum and asks the Immich server
   (in batches) which of those checksums it already has.
4. Uploads only the ones the server doesn't have yet, then records the
   result locally so the next run can skip them via step 2.

## Building from source

Requires Rust (`cargo build --release`). Dependencies: `clap`, `reqwest`
(rustls, no system OpenSSL needed), `serde`/`toml`/`serde_json`, `sha1`,
`walkdir`, `chrono`, `anyhow`, `directories`, `rpassword`.

Releases are built by `.github/workflows/release.yml` on `v*` tags: a `.deb`
package (via `cargo-deb`) for Linux and an Inno Setup installer for Windows.
