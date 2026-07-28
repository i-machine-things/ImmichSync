# Coding Best Practices & Reminders

> **Style rule:** Notes must be clear and concise — 300 characters or less each. Group by topic, not by date. Whenever a PR review (CodeRabbit or human) catches a mistake, add or amend a note here right away so it isn't repeated.

## Resource Cleanup & Temporary Files

**IMPORTANT**: Always add proper cleanup code in programs to prevent lingering temp files after closing.

### Best Practices:

1. **GUI Applications (PyQt, Tkinter, etc.)**
   - Implement `closeEvent()` handler to cleanup resources on window close
   - Call `deleteLater()` on widgets to ensure proper Qt object cleanup
   - Process pending events with `app.processEvents()` before exit

2. **File Handling**
   - Use context managers (`with` statements) for file operations
   - Explicitly close file handles when not using context managers
   - Release file locks before program exit
   - Clean up temporary files in temp directories

3. **Background Threads & Workers**
   - Stop and join all background threads before exit
   - Cancel any pending operations
   - Clean up thread-specific resources

4. **Testing Cleanup**
   - After closing the program, verify the executable can be:
     - Deleted immediately
     - Moved to another location
     - Replaced with a new version
   - If the file is locked, cleanup code is missing or incomplete

### Example Implementation (PyQt6):

```python
def closeEvent(self, event):
    """Handle window close event - ensure proper cleanup"""
    # Cleanup modules/components
    for module in self.modules:
        try:
            module.cleanup()
        except Exception as e:
            print(f"Error cleaning up module: {e}")

    # Save state
    self.save_settings()

    # Accept close event
    event.accept()
    QApplication.quit()

def main():
    app = QApplication(sys.argv)
    window = MainWindow()
    window.show()

    exit_code = app.exec()

    # Final cleanup
    window.deleteLater()
    app.processEvents()

    sys.exit(exit_code)
```

### PyInstaller Specific:

In `.spec` file, add:
```python
exe = EXE(
    ...
    bootloader_ignore_signals=True,  # Better cleanup handling
    ...
)
```

## Date: 2025-12-16
This note was created based on issues encountered with PyInstaller executables remaining locked after closing.

## General Style Notes

- **Keep lines under 120 characters.** Long lines are hard to review side-by-side in a diff or split editor pane, and tend to signal a line doing too many things at once. Wrap or break up expressions rather than letting them run long.
- **Add docstrings to explain code.** Focus on *why* a function/class exists or *why* it does something non-obvious — the code itself already shows *what* it does. A docstring worth writing usually covers intent, assumptions, edge cases, or a gotcha a future reader would otherwise have to rediscover the hard way.
- **Strip docstrings when building a release.** Release builds don't need internal rationale shipped alongside the binary — it bloats the artifact and can leak implementation notes you didn't mean to publish. Run Python with `-OO` (or an equivalent build step) to drop docstrings and assertions from the compiled output before packaging.

## GitHub Actions Security (CodeRabbit, ImmichSync PR#1)

- **Set `persist-credentials: false` on every `actions/checkout` step** unless that job actually needs to push back to the repo. Otherwise the `GITHUB_TOKEN` sits in the local git config for the rest of the job — unnecessary exposure surface (zizmor's `artipacked` check flags this).
- **Declare a workflow-level `permissions:` block** (e.g. `contents: read`) so jobs don't implicitly inherit a broader default token scope. Jobs that need more (e.g. `contents: write` to create a release) declare it at the job level, which overrides the top-level default for that job only.
- **Don't re-template `${{ env.X }}` into a shell/pwsh script body** when `X` is already a job/step-level env var — it's already available as `$env:X`/`$X` directly, and re-interpolating it into the script text is an unnecessary template/script-injection surface (zizmor's `template-injection` check).

## Inno Setup PATH Management

- **When removing a directory from PATH, pad both sides before searching, or better, tokenize and filter.** A search for `;AppDir;` against `EnvPath + ';'` misses the case where `AppDir` is the *first* PATH entry (no leading separator to match). Splitting on `;`, filtering out the matching entry (case-insensitively), and rejoining is more robust than position/padding arithmetic.
- **Don't chain two quoted-path commands with `&&` inside `cmd /K`.** cmd.exe's leading-quote-stripping can corrupt the second quoted segment after `&&` ("filename, directory name, or volume label syntax is incorrect") — documented cmd.exe behavior, not verified on real Windows.
- **Prefer invoking the target exe directly over wrapping it in `cmd /K`.** Sidesteps the quoting pitfall by construction, and avoids `/K` leaving a shell open after the command finishes, which can block an Inno Setup installer's Finish page until the user closes it manually.

## Rust CLI Patterns

- **Constrain numeric CLI args to their valid domain with clap's `value_parser!(T).range(..)`**, not just the type's full range — e.g. an "hour of day" field typed `u8` still accepts up to 255 unless explicitly range-limited to `0..=23`.
- **Create secret-bearing files (config with API keys, etc.) with restrictive permissions from the moment of creation** (`OpenOptions::mode(0o600)` on Unix), not write-then-chmod — the latter leaves a TOCTOU window where the file is readable at the OS-default mode.
- **Give upload/large-payload HTTP calls their own longer timeout**, separate from the client's default timeout used for quick metadata calls — a shared short timeout can abort a large file upload over slow home upload bandwidth well before it finishes.
- **Stream large file uploads from disk** (e.g. `multipart::Form::file(name, path)`) instead of `std::fs::read` + `Part::bytes`, so uploading a multi-GB video doesn't buffer the whole thing in memory first.
- **Don't let one bad item abort a whole batch loop.** In a loop over independent inputs (e.g. multiple configured directories), catch and log per-item errors and `continue` rather than propagating with `?` — one inaccessible directory shouldn't block every other directory's nightly backup.
- **Cap/rotate append-only log files used by scheduled jobs, and check the cap on every write, not just at process start.** A single large run (e.g. first-time backfill of an existing library) can cross the size threshold entirely within one invocation — a startup-only check misses that until the *next* run. Simple single-backup rotation (rename to `.1`) is enough; no need for a full rotation library.
- **Check the size cap *after* writing, not before.** A before-write check only catches files already over the limit from a previous write — the write that actually crosses the threshold stays unrotated until some later call. If that's the last line logged in a run, the file stays over-cap indefinitely.
- **Root-run maintainer scripts (deb `prerm`/`postrm`, etc.) must never `rm`/operate directly on paths under a user's home directory.** `~/.config` (and everything under it) is entirely user-controlled, including via symlinks — a root-run `rm` following such a path can be redirected by the user to delete a file they couldn't otherwise touch. Do the actual file operation via `runuser -l <user> -c '...'` so it runs with that user's own permissions instead.
- **Pin CI toolchain/tool versions only when you can actually verify the pinned value exists** (e.g. via the registry's API) — guessing an unverified version number to satisfy a reproducibility nitpick risks a hard CI failure on every future run, which is worse than the non-determinism being fixed.
- **When treating "file doesn't exist" as a no-op, match `ErrorKind::NotFound` specifically** rather than swallowing every `metadata()`/`open()` error — a permission-denied error silently treated the same way hides the real cause.
- **Only persist a "last checked" cache timestamp on success**, not unconditionally — otherwise a single transient network failure suppresses retries for the whole cache interval instead of just that one run.
- **Dot-prefix alone doesn't cover Windows hidden files.** Also check the `FILE_ATTRIBUTE_HIDDEN` bit via `MetadataExt::file_attributes()` on Windows, or files like `Thumbs.db`/`desktop.ini` won't be filtered on that platform.
- **In recursive walks, skip nested IO errors; only fail at root.** Check `walkdir::Error::depth()`:
  depth 0 → propagate (root unreadable); depth > 0 → `continue`. Same policy for
  `DirEntry::metadata()` failures using `entry.depth()`.
- **Reject root drives as photo directories during setup.** `PathBuf::parent().is_none()`
  identifies `C:\` / `/`. Canonicalize first (resolves `..` and symlinks), but store the
  original path to avoid Windows `\\?\` prefix in config.
