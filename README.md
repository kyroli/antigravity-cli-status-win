# Antigravity CLI Statusline for Windows 11

TUI statusline and window title formatter for Antigravity CLI on Windows 11, implemented in Rust.

## Previews

```text
# Single-line layout (Terminal width >= 160 columns)
╭─ [READY] | Gemini 3.5 Flash | ⚡ [-----] (~3h12m) |  /path/to/my-project |  main* (+12/-5) | 󰘚 [========>-] 14.8% (148.0K/1.0M) |  rd:115.8K/wr:0 | 󰧑 4 | 󰚩 1 | 󰔛 2

# Double-line layout (Terminal width >= 80 columns)
╭─ [READY] | Claude Sonnet 3.5 | ⚡ [-----] (~3h12m) |  my-project |  main* (+12/-5)
╰─ 󰘚 [====>---] 65.0% (free:35.0%/350.0K) |  rd:115.8K/wr:0 | 󰧑 4

# Compact layout (Terminal width < 80 columns)
[THINKING] | Sonnet 3.5 | ⚡ 65%
󰘚 [====>-] 65.0% (650.0K/1.0M)

# Window title output
😴 idle | my-project
```

*Note: Directory paths and Git branch badges embed OSC8 escape sequences to enable terminal-based hyperlinks redirection.*

## Technical Architecture

- **Native Binary**: Compiled with size optimization flags (`opt-level = "z"`, Link-Time Optimization, and stripped symbols).
- **Inter-Process Communication**: Uses Windows Shared Memory (`CreateFileMappingW`) and Named Mutexes (`CreateMutexW`) for synchronization and cached data sharing between rendering calls and background updates. The IPC structure (`SharedVcsInfo`) uses a fixed memory layout (`#[repr(C)]`) and version identification (protocol version `5`).
- **Foreground Change Detection**: Performs fast checks on `.git/HEAD` and `.git/index` modification times (`mtime`) in the foreground process. Skips background refresh process execution if the file status matches the cached state and the entry age is below the 10-second Time-To-Live (TTL) threshold.
- **Layout Selection**: Switches output formatting between single-line, double-line, and compact views based on the terminal column count.
- **VCS & Subscription Queries**: Queries Git status (branch, modified status, ahead/behind counts, insertions, and deletions) and checks model subscription quotas via background execution.
- **Terminal Hyperlinks**: Emits OSC8 escape sequences for active directory paths and Git remote URLs, mapping clicks to local directory navigation or web-based repository redirection. Visual width calculations employ a parser state machine to ignore non-printing control sequences.
- **Credential Storage Access**: Reads authentication tokens via the Win32 `CredReadW` API or falls back to local files.

## Configuration

### 1. Integration Settings (`~/.gemini/antigravity-cli/settings.json`)

Integration into the CLI utilizes the following fields in `settings.json`:

```json
{
  "...": "...",
  "statusLine": {
    "type": "command",
    "command": "C:/path/to/statusline.exe",
    "enabled": true
  },
  "title": {
    "type": "command",
    "command": "C:/path/to/statusline.exe --title",
    "enabled": true
  }
}
```

The executable runs in title formatting mode when the `--title` argument is passed.

### 2. Theme Configuration (`~/.gemini/antigravity-cli/statusline/statusline.json`)

The application utilizes an opinionated zero-configuration design. Layout elements, padding, and state indicators are dynamically computed based on the active terminal column width.

Theme selection is the only configurable parameter.

```json
{
  "theme": "frost"
}
```

#### Available Themes

- `"frost"` (Default): Truecolor palette specifying cold blue and frost accents.
- `"pastel"`: Truecolor palette specifying pastel pink and mauve accents.
- `"neon"`: Truecolor palette specifying high-contrast neon cyan, green, and orange accents.

## Compilation & Execution

### Compilation Command

```powershell
cargo build --release
```

Output binary: `target/release/statusline.exe`

### Command Line Interface

- `--theme <name>`: Configures the visual theme (`"frost"`, `"pastel"`, `"neon"`).
- `--title`: Renders the terminal title string.
- `--refresh [--cwd <path>]`: Executes background caching of VCS and quota queries.

## Security Verification

```powershell
gh attestation verify statusline.exe --repo <github-username>/antigravity-statusline-win11-rs
```

## Requirements

- **Operating System**: Windows 11.
- **Dependencies**: None.

## Legacy JS Version

The legacy JavaScript implementation is removed.

## References

- [Antigravity CLI Statusline Documentation](https://antigravity.google/docs/cli-statusline)
- [Antigravity CLI Title Documentation](https://antigravity.google/docs/cli-title)
- [Google Antigravity CLI Examples](https://github.com/google-antigravity/antigravity-cli/tree/main/examples)

## License

MIT
