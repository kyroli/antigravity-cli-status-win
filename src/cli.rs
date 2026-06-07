// CLI argument parsing, mode dispatch, and application orchestration.

use std::fs;
use std::io::Read;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::types::{parse_input_json, load_user_config, CacheData, InputJson};
use crate::path::resolve_antigravity_path;
use crate::platform::{read_shared_cache, NamedMutex};
use crate::render::render_tui;
use crate::title::get_title_string;
use crate::merge::merge_input_json;
use crate::refresh::run_background_refresh;

pub fn run() {
    let args: Vec<String> = std::env::args().collect();

    if let Some(idx) = args.iter().position(|a| a == "--theme") {
        if idx + 1 < args.len() {
            set_config_theme(&args[idx + 1]);
            return;
        }
    }

    if args.contains(&"--title".to_string()) {
        run_title_mode();
        return;
    }

    if args.contains(&"--refresh".to_string()) {
        let mut cwd_force = None;
        if let Some(idx) = args.iter().position(|a| a == "--cwd") {
            if idx + 1 < args.len() {
                cwd_force = Some(args[idx + 1].clone());
            }
        }
        run_background_refresh(cwd_force);
        std::process::exit(0);
    }

    run_statusline_mode();
}

fn run_title_mode() {
    let mut input_data = String::new();
    if std::io::stdin().read_to_string(&mut input_data).is_ok() {
        let json = parse_input_json(&input_data);
        let title_str = get_title_string(&json);
        println!("{}", title_str);
    }
}

fn run_statusline_mode() {
    let mut input_data = String::new();
    if std::io::stdin().read_to_string(&mut input_data).is_ok() {
        let mut json = parse_input_json(&input_data);
        let last_input_path = resolve_antigravity_path("last-input.json");
        let last_json: Option<InputJson> = fs::read_to_string(&last_input_path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok());

        json = merge_input_json(json, last_json);
        let raw_cwd = json
            .workspace
            .as_ref()
            .and_then(|w| w.current_dir.clone())
            .or_else(|| json.cwd.clone())
            .unwrap_or_default();

        let cache = read_shared_cache().unwrap_or_else(|| {
            let status_cache_path = resolve_antigravity_path("statusline-cache.json");
            let mut c = CacheData::default();
            if let Ok(cache_str) = fs::read_to_string(&status_cache_path) {
                if let Ok(parsed) = serde_json::from_str::<CacheData>(&cache_str) {
                    c = parsed;
                }
            }
            c
        });

        let config = load_user_config();
        render_tui(&config, &json, &cache);

        // Persist current input for merge on next invocation
        if let Ok(serialized) = serde_json::to_string(&json) {
            let tmp_path = format!(
                "{}.tmp.{}",
                last_input_path.to_string_lossy(),
                std::process::id()
            );
            if fs::write(&tmp_path, serialized).is_ok() {
                let _ = fs::rename(tmp_path, &last_input_path);
            }
        }

        // Determine if background refresh is needed
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let status_cache_path = resolve_antigravity_path("statusline-cache.json");

        let need_refresh = if !status_cache_path.exists() {
            true
        } else {
            let cached_cwd = cache.vcs.as_ref().map(|v| v.cwd.as_str()).unwrap_or("");
            let vcs_last_checked = cache.vcs.as_ref().map(|v| v.last_checked).unwrap_or(0);

            if !raw_cwd.is_empty() && raw_cwd != cached_cwd {
                true
            } else {
                let quota_age = now.saturating_sub(cache.last_refreshed);
                let vcs_age = now.saturating_sub(vcs_last_checked);
                quota_age > 120 || vcs_age > 3
            }
        };

        if need_refresh {
            spawn_background_refresh(&raw_cwd);
        }
    }
}

fn spawn_background_refresh(raw_cwd: &str) {
    let mutex_active = NamedMutex::is_active("Local\\AgyStatuslineRefreshMutex");
    if mutex_active {
        return;
    }

    if let Ok(current_exe) = std::env::current_exe() {
        let mut cmd = std::process::Command::new(current_exe);
        cmd.arg("--refresh");
        if !raw_cwd.is_empty() {
            cmd.arg("--cwd").arg(raw_cwd);
        }
        cmd.stdin(std::process::Stdio::null());
        cmd.stdout(std::process::Stdio::null());
        cmd.stderr(std::process::Stdio::null());

        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            cmd.creation_flags(0x08000000);

            use std::os::windows::io::AsRawHandle;
            unsafe {
                windows_sys::Win32::Foundation::SetHandleInformation(
                    std::io::stdout().as_raw_handle() as _,
                    windows_sys::Win32::Foundation::HANDLE_FLAG_INHERIT,
                    0,
                );
                windows_sys::Win32::Foundation::SetHandleInformation(
                    std::io::stderr().as_raw_handle() as _,
                    windows_sys::Win32::Foundation::HANDLE_FLAG_INHERIT,
                    0,
                );
                windows_sys::Win32::Foundation::SetHandleInformation(
                    std::io::stdin().as_raw_handle() as _,
                    windows_sys::Win32::Foundation::HANDLE_FLAG_INHERIT,
                    0,
                );
            }
        }

        let _ = cmd.spawn();
    }
}

fn set_config_theme(theme_name: &str) {
    let mut config = load_user_config();
    let theme_lower = theme_name.trim().to_lowercase();
    let valid = matches!(theme_lower.as_str(), "frost" | "pastel" | "neon");
    if valid {
        config.theme = theme_lower;
        let path = resolve_antigravity_path("statusline.json");
        if let Ok(json_str) = serde_json::to_string_pretty(&config) {
            let _ = std::fs::write(&path, json_str);
            println!("Theme set to: '{}'", theme_name);
        } else {
            println!("Error serializing configuration.");
        }
    } else {
        println!(
            "Unknown theme name: '{}'. Valid options are: 'frost', 'pastel', 'neon'.",
            theme_name
        );
    }
}
