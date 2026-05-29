# Antigravity CLI Statusline for Windows 11

TUI statusline and window title formatter for Antigravity CLI on Windows 11, implemented in Rust.

## Previews

```text
# Single-line layout (Terminal width >= 160 columns)
╭─ [READY] | Gemini 3.5 Flash (Medium) | q:85% ~3h12m | /path/to/my-project | @main* | ctx [=>-----------] 14.8% (148.0K/1.0M) | artifacts 4

# Double-line layout (Terminal width >= 80 columns)
╭─ [READY] | Claude Sonnet 3.5 (Thinking) | my-project | @main*
╰─ ctx [=>-----------] 14.8% (148.0K/1.0M | in:100.0K/out:48.0K) | cache rd:115.8K/wr:0 | artifacts 4

# Compact layout (Terminal width < 80 columns)
[THINKING] | Sonnet 3.5 (Th) | q:[-----]
ctx [==>-----] 25.3%

# Window title output
😴 idle | my-project
```

## Technical Architecture

- **Native Binary**: Compiled with size optimization flags (`opt-level = "z"`, Link-Time Optimization, and stripped symbols).
- **Inter-Process Communication**: Uses Windows Shared Memory (`CreateFileMappingW`) and Named Mutexes (`CreateMutexW`) for synchronization and cached data sharing between rendering calls and background updates.
- **Layout Selection**: Switches output formatting between single-line, double-line, and compact views based on the terminal column count.
- **VCS & Subscription Queries**: Queries Git status (branch, modified status, ahead/behind counts) and checks model subscription quotas via background execution.
- **Credential Storage Access**: Reads authentication tokens via the Win32 `CredReadW` API or falls back to local files.

## Configuration

### 1. Integration Settings (`~/.gemini/antigravity-cli/settings.json`)

Integration into the CLI utilizes the following fields in `settings.json`:

```json
{
  "statusLine": {
    "type": "command",
    "command": "C:/path/to/statusline.exe"
  },
  "title": {
    "type": "command",
    "command": "C:/path/to/statusline.exe --title"
  }
}
```

The executable runs in title formatting mode if the filename matches `title.exe` or if the `--title` argument is passed.

### 2. Widget Options (`~/.gemini/antigravity-cli/statusline/statusline.json`)

The layout, state display strings, and color schemes are defined in `statusline.json`.

#### Layout Configuration (`layout` field)

| Parameter | Type | Default | Description |
| :--- | :--- | :--- | :--- |
| `show_state` | boolean | `true` | Renders agent execution state |
| `show_model` | boolean | `true` | Renders configured model name |
| `show_path` | boolean | `true` | Renders current directory path |
| `show_vcs` | boolean | `true` | Renders Git repository status |
| `show_quota` | boolean | `true` | Renders model API quota usage |
| `show_quota_bar` | boolean | `true` | Renders progress bar for API quota |
| `show_pending_input` | boolean | `true` | Renders pending user input count |
| `show_approval_alert` | boolean | `true` | Renders warning alert when awaiting approval |
| `show_context_bar` | boolean | `true` | Renders progress bar for context window utilization |
| `show_cache_stats` | boolean | `true` | Renders context cache reads/writes metrics |
| `show_artifacts` | boolean | `true` | Renders artifact count |
| `show_subagents` | boolean | `true` | Renders active subagent count |
| `show_tasks` | boolean | `true` | Renders active background tasks |
| `show_sandbox` | boolean | `true` | Renders sandbox network status indicators |
| `show_conversation_id` | boolean | `false` | Renders conversation ID |
| `show_version` | boolean | `false` | Renders application version |
| `show_plan_tier` | boolean | `false` | Renders active plan tier |
| `show_email` | boolean | `false` | Renders user email |

#### Agent States (`states` field)

Maps the execution state value to an ANSI escape sequence string:
- `ready`: Default is `\x1b[92m\x1b[1m[READY]\x1b[0m`
- `thinking`: Default is `\x1b[93m\x1b[1m[THINKING]\x1b[0m`
- `working`: Default is `\x1b[96m\x1b[1m[WORKING]\x1b[0m`
- `tool_use`: Default is `\x1b[95m\x1b[1m[TOOL]\x1b[0m`
- `default`: Default is `\x1b[97m\x1b[1m[STATE]\x1b[0m`

#### Color Configuration (`colors` field)

Configures the ANSI escape sequences for text segments:
- `vcs`: Default is `\x1b[94m`
- `path`: Default is `\x1b[94m`
- `model`: Default is `\x1b[90m\x1b[3m`
- `border`: Default is `\x1b[90m`

## Compilation & Execution

### Compilation Command

```powershell
cargo build --release
```

Output binary: `target/release/statusline.exe`

### Command Line Interface

- `--configure` or `--config`: Opens terminal prompts to write configuration changes to `statusline.json`.
- `--toggle <field>`: Inverts the Boolean layout flag for the specified field.
- `--title`: Renders the terminal title string.
- `--refresh [--cwd <path>]`: Executes background caching of VCS and quota queries.

## Security Verification

```powershell
gh attestation verify statusline.exe --repo <github-username>/<repository-name>
```

## Requirements

- **Operating System**: Windows 11.
- **Dependencies**: None.

## Legacy JS Version

The legacy JavaScript implementation is archived in the `legacy-js` directory.

## References

- [Antigravity CLI Statusline Documentation](https://antigravity.google/docs/cli-statusline)
- [Antigravity CLI Title Documentation](https://antigravity.google/docs/cli-title)
- [Google Antigravity CLI Examples](https://github.com/google-antigravity/antigravity-cli/tree/main/examples)

## License

MIT
