# ImmichHaul

A nightly backup CLI (Rust) that uploads new photos/videos from a local
directory to a self-hosted [Immich](https://immich.app) server. Files already
present on the server (matched by SHA1 checksum, via Immich's
`bulk-upload-check` API) are skipped, so it's safe to point at a folder
that's already been uploaded manually — nothing gets re-uploaded.

## Install

**Linux** — install is handled entirely by `apt`/`dpkg`, no wrapper script:

```bash
curl -fsSLO https://github.com/i-machine-things/ImmichHaul/releases/latest/download/immich-haul-<version>-linux-amd64.deb
sudo apt install ./immich-haul-<version>-linux-amd64.deb
```

**Windows**: download `ImmichHaul-<version>-windows-setup.exe` from the
[latest release](https://github.com/i-machine-things/ImmichHaul/releases/latest)
and run it. The installer offers to run setup and enable the nightly
schedule at the end.

Either way, there's no separate setup step to remember: the first time you
run any command (`immich-haul run`, `immich-haul status`, `immich-haul service
install`) with no config yet and a terminal attached, it walks you through
setup (server URL, API key, photos directory) automatically before
continuing. Unattended invocations (the nightly timer/scheduled task) fail
with a clear error instead of hanging on a prompt if setup was never done.

## Usage

```bash
immich-haul init              # (re-)run setup explicitly: server URL, API key, photos dir
immich-haul run --dry-run     # preview a sync without uploading anything
immich-haul run               # sync now (prompts for setup first if not configured yet)
immich-haul status            # show config, log tail
immich-haul service install   # schedule a nightly run (systemd user timer / Task Scheduler)
immich-haul service uninstall # remove the nightly schedule
immich-haul update check      # check GitHub for a newer release
```

If the machine is off or asleep at the scheduled hour, the systemd timer's
`Persistent=true` ensures the sync catches up on Linux. On Windows, the
installer attempts to set `StartWhenAvailable` via a PowerShell follow-up
step after `schtasks` registers the task — if that step fails, it logs a
warning and the task still runs normally, just without catch-up. Neither
option wakes the machine; both just run on next opportunity.

Config lives at the OS-appropriate config dir (e.g. `~/.config/immich-haul/config.toml`
on Linux, `%APPDATA%\immich-haul\config.toml` on Windows) with `0600`
permissions on Unix, since it holds your Immich API key. Sync state (which
files are already uploaded) and logs live in the OS data dir alongside it.

## Updates

`immich-haul update check` hits GitHub's releases API and prints whether a
newer version exists. `immich-haul run` also does this passively at most once
every 24h (cached, best-effort — never blocks or fails the sync, and a
transient network error doesn't suppress the next day's check) and logs a
one-line note if an update is available.

There's no auto-download or self-replacement — the binary never modifies
itself, which matters since it usually runs unattended via a scheduled
task holding your API key. To actually update: download the new `.deb` /
installer the same way as the initial install (re-running `apt install
./newfile.deb` upgrades in place; re-running the Windows installer does the
same). Your config, sync manifest, and logs live outside the install
directory, so upgrading doesn't touch them.

## Uninstall

**Linux**: `sudo apt remove immich-haul`. The package's `prerm` script
best-effort disables and removes the systemd user timer/service for any user
that has one installed, so removal doesn't leave a timer pointing at a
binary that no longer exists. Config/manifest/logs under
`~/.config/immich-haul` and `~/.local/share/immich-haul` are left in place —
delete them yourself if you want a full wipe.

**Windows**: use "Uninstall ImmichHaul" from the Start Menu / Apps list. The
uninstaller runs `immich-haul.exe service uninstall` first (removing the
scheduled task) and removes the app from PATH. Config/data under
`%APPDATA%\immich-haul` / `%LOCALAPPDATA%\immich-haul` are left in place.

If you just want to stop the nightly runs without removing the app, use
`immich-haul service uninstall` directly on either platform.

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
package (via `cargo-deb --deb-version`) for Linux and an Inno Setup installer
for Windows.

`--version` reports a live `git describe` (e.g. `v1.0.0` exactly at a tagged
release commit, `v1.0.0-3-gabc1234` for a dev build 3 commits past the last
tag, `-dirty` appended for uncommitted changes) rather than a static number —
see `build.rs`. Building from a source tarball with no `.git` directory falls
back to the version in `Cargo.toml`.

## License

GPL-3.0-or-later — see [LICENSE](LICENSE).
