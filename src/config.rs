// Interactive TUI configuration panel, account management, and user settings I/O.

use std::fs;

use crate::types::{
    CacheData, ContextWindow, InputJson, ModelInfo, QuotaItem, UserConfig, VcsInfo, Workspace,
};
use crate::path::{resolve_antigravity_path, get_antigravity_dir};
use crate::platform::{read_windows_credential, write_windows_credential};
use crate::render::render_tui;
use crate::theme::{self, RESET, BOLD};

/// Polling interval (in milliseconds) when waiting for login credentials during
/// the interactive account-add flow.
const LOGIN_POLL_INTERVAL_MS: u64 = 500;

// --- User configuration I/O -------------------------------------------------

pub fn load_user_config() -> UserConfig {
    let path = resolve_antigravity_path("statusline.json");
    if path.exists() {
        if let Ok(content) = fs::read_to_string(&path) {
            if let Ok(config) = serde_json::from_str::<UserConfig>(&content) {
                return config;
            }
        }
    } else {
        let default_config = UserConfig::default();
        if let Ok(json_str) = serde_json::to_string_pretty(&default_config) {
            let _ = fs::write(&path, json_str);
        }
    }
    UserConfig::default()
}

// --- Shared helpers for account management -----------------------------------

/// Strip control characters and colons, then truncate to 64 characters.
fn sanitize_alias(input: &str) -> String {
    let mut clean: String = input
        .trim()
        .chars()
        .filter(|&c| !c.is_control() && c != ':')
        .collect();
    clean.truncate(64);
    clean
}

/// Read the cached email address from the last statusline input JSON file.
fn read_cached_email() -> String {
    let path = resolve_antigravity_path("last-input.json");
    if let Ok(content) = fs::read_to_string(&path) {
        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&content) {
            if let Some(email) = parsed.get("email").and_then(|e| e.as_str()) {
                return email.trim().to_string();
            }
        }
    }
    String::new()
}

/// Persist a credential pair (main + oauth-token) to Windows Credential Manager
/// under the given alias, and register the alias in the user config.
/// Returns true if the main credential was written successfully.
fn save_credential_with_alias(
    alias: &str,
    main_cred: &str,
    local_token: &str,
    config: &mut UserConfig,
) -> bool {
    let target = format!("gemini:antigravity:{}", alias);
    let oauth_target = format!("gemini:antigravity:oauth-token:{}", alias);

    let ok = write_windows_credential(&target, main_cred);
    if !local_token.is_empty() {
        let _ = write_windows_credential(&oauth_target, local_token);
    }
    if ok && !config.saved_accounts.contains(&alias.to_string()) {
        config.saved_accounts.push(alias.to_string());
    }
    ok
}

/// Search saved accounts for one whose stored credential matches `credential`.
fn find_alias_for_credential(credential: &str, saved_accounts: &[String]) -> Option<String> {
    for acc in saved_accounts {
        let target = format!("gemini:antigravity:{}", acc);
        if let Some(saved) = read_windows_credential(&target) {
            if saved == credential {
                return Some(acc.clone());
            }
        }
    }
    None
}

// --- Theme configuration -----------------------------------------------------

pub fn set_config_theme(theme_name: &str) {
    let mut config = load_user_config();
    let theme_lower = theme_name.trim().to_lowercase();
    let valid = matches!(theme_lower.as_str(), "frost" | "pastel" | "neon");
    if valid {
        config.theme = theme_lower;
        let path = resolve_antigravity_path("statusline.json");
        if let Ok(json_str) = serde_json::to_string_pretty(&config) {
            let _ = fs::write(&path, json_str);
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

// --- Interactive configuration UI --------------------------------------------

pub fn run_interactive_config() {
    use console::{Term, Key};

    let term = Term::stdout();
    let _ = term.hide_cursor();

    let mut config = load_user_config();

    let mock_json = InputJson {
        agent_state: Some("idle".to_string()),
        model: Some(ModelInfo {
            id: Some("gemini-3.5-flash".to_string()),
            display_name: Some("Gemini 3.5 Flash".to_string()),
        }),
        workspace: Some(Workspace {
            current_dir: Some("C:\\Develop\\my-project".to_string()),
            project_dir: Some("C:\\Develop\\my-project".to_string()),
        }),
        cwd: Some("C:\\Develop\\my-project".to_string()),
        context_window: Some(ContextWindow {
            used_percentage: Some(14.8),
            remaining_percentage: Some(85.2),
            total_input_tokens: Some(148000),
            total_output_tokens: Some(2000),
            context_window_size: Some(1000000),
            current_usage: None,
        }),
        terminal_width: Some(100),
        ..Default::default()
    };

    let mock_cache = CacheData {
        quota: vec![QuotaItem {
            id: "gemini-3.5-flash".to_string(),
            display_name: "Gemini 3.5 Flash".to_string(),
            remaining_fraction: 0.95,
            reset_time: Some("2026-06-09T05:12:00Z".to_string()),
        }],
        vcs: Some(VcsInfo {
            cwd: "C:\\Develop\\my-project".to_string(),
            branch: "main".to_string(),
            dirty: true,
            ahead: 1,
            behind: 0,
            modified: 3,
            last_checked: 0,
            head_mtime: None,
            index_mtime: None,
            remote_web_url: Some("https://github.com/user/my-project".to_string()),
            insertions: 12,
            deletions: 5,
        }),
        last_refreshed: 0,
        token_hash: None,
        needs_login: Some(false),
    };

    let mut selected_idx = 0;

    loop {
        let _ = term.clear_screen();
        let theme = theme::resolve(&config.theme);
        let p = theme.palette;
        let r = RESET;
        let b = BOLD;

        println!("{}{}{}", p.border, "==================================================================", r);
        println!("{}{}                 Antigravity Statusline Config                    {}", p.accent, b, r);
        println!("{}{}{}", p.border, "==================================================================", r);
        println!("{}--- PREVIEW ------------------------------------------------------{}", p.border, r);
        render_tui(&config, &mock_json, &mock_cache);
        println!("{}{}{}", p.border, "------------------------------------------------------------------", r);
        println!("\n{}[SETTINGS]{}", p.label, r);

        const MENU_ITEM_COUNT: usize = 8;
        for i in 0..MENU_ITEM_COUNT {
            let text = match i {
                0 => format!("Theme: {}", config.theme),
                1 => format!("Show Git VCS Info: {}", if config.show_vcs { "\u{25cf} Enabled" } else { "\u{25cb} Disabled" }),
                2 => format!("Show Model Quota: {}", if config.show_quota { "\u{25cf} Enabled" } else { "\u{25cb} Disabled" }),
                3 => format!("Show Context Token Bar: {}", if config.show_context { "\u{25cf} Enabled" } else { "\u{25cb} Disabled" }),
                4 => format!("Show Settings Gear Icon: {}", if config.show_settings { "\u{25cf} Enabled" } else { "\u{25cb} Disabled" }),
                5 => "Account Management".to_string(),
                6 => "Save & Exit".to_string(),
                7 => "Cancel & Exit".to_string(),
                _ => unreachable!(),
            };

            if i == selected_idx {
                println!(" {}{}\u{276f} {}{}", p.accent, b, text, r);
            } else {
                let colored_text = match i {
                    1 => format!("Show Git VCS Info: {}", if config.show_vcs { format!("{}\u{25cf} Enabled{}", p.success, p.label) } else { format!("{}\u{25cb} Disabled{}", p.border, p.label) }),
                    2 => format!("Show Model Quota: {}", if config.show_quota { format!("{}\u{25cf} Enabled{}", p.success, p.label) } else { format!("{}\u{25cb} Disabled{}", p.border, p.label) }),
                    3 => format!("Show Context Token Bar: {}", if config.show_context { format!("{}\u{25cf} Enabled{}", p.success, p.label) } else { format!("{}\u{25cb} Disabled{}", p.border, p.label) }),
                    4 => format!("Show Settings Gear Icon: {}", if config.show_settings { format!("{}\u{25cf} Enabled{}", p.success, p.label) } else { format!("{}\u{25cb} Disabled{}", p.border, p.label) }),
                    _ => text,
                };
                println!("   {}{}{}", p.label, colored_text, r);
            }
        }

        println!("\n{}  Use \u{2191}/\u{2193} to navigate \u{2022} Enter to toggle/select \u{2022} Esc to abort{}", p.border, r);
        println!("{}{}{}", p.border, "==================================================================", r);

        let key = term.read_key().unwrap_or(Key::Unknown);
        match key {
            Key::ArrowUp => {
                if selected_idx > 0 {
                    selected_idx -= 1;
                } else {
                    selected_idx = MENU_ITEM_COUNT - 1;
                }
            }
            Key::ArrowDown => {
                if selected_idx < MENU_ITEM_COUNT - 1 {
                    selected_idx += 1;
                } else {
                    selected_idx = 0;
                }
            }
            Key::Enter => {
                match selected_idx {
                    0 => {
                        config.theme = match config.theme.as_str() {
                            "frost" => "pastel".to_string(),
                            "pastel" => "neon".to_string(),
                            _ => "frost".to_string(),
                        };
                    }
                    1 => config.show_vcs = !config.show_vcs,
                    2 => config.show_quota = !config.show_quota,
                    3 => config.show_context = !config.show_context,
                    4 => config.show_settings = !config.show_settings,
                    5 => {
                        run_account_management(&term, &mut config);
                        continue;
                    }
                    6 => {
                        let path = resolve_antigravity_path("statusline.json");
                        if let Ok(json_str) = serde_json::to_string_pretty(&config) {
                            if fs::write(&path, json_str).is_ok() {
                                println!("\n{}[SUCCESS] Configuration saved to:{}", p.success, r);
                                println!("  {}", path.to_string_lossy());
                            } else {
                                println!("\n{}[ERROR] Failed to write config file.{}", p.critical, r);
                            }
                        }
                        break;
                    }
                    7 => break,
                    _ => {}
                }
            }
            Key::Escape => break,
            _ => {}
        }
    }

    let _ = term.show_cursor();
}

// --- Account management submenu ----------------------------------------------

fn run_account_management(term: &console::Term, config: &mut UserConfig) {
    use console::Key;

    let mut switch_idx = 0;
    loop {
        let _ = term.clear_screen();
        let theme = theme::resolve(&config.theme);
        let p = theme.palette;
        let r = RESET;
        let b = BOLD;

        let old_main_cred = read_windows_credential("gemini:antigravity")
            .unwrap_or_default();
        let root = get_antigravity_dir();
        let oauth_path = root.join("antigravity-oauth-token");
        let old_local_token = fs::read_to_string(&oauth_path).unwrap_or_default();

        // Determine if the active account matches a saved alias
        let current_alias = if !old_main_cred.is_empty() {
            find_alias_for_credential(&old_main_cred, &config.saved_accounts)
        } else {
            None
        };

        // --- Draw header ---
        println!("{}{}{}", p.border, "==============================================================", r);
        println!("                      {}{}Account Management{}{}", b, p.accent, r, r);
        println!("{}{}{}", p.border, "==============================================================", r);
        if old_main_cred.is_empty() {
            println!("  {}Current Active: {}Not Logged In{}", p.label, p.critical, r);
        } else if let Some(ref alias) = current_alias {
            println!("  {}Current Active: {}{} (Saved){}", p.label, p.success, alias, r);
        } else {
            println!("  {}Current Active: {}Unsaved Account{}", p.label, p.warning, r);
        }
        println!("{}{}{}", p.border, "--------------------------------------------------------------", r);

        // --- Draw saved account items ---
        let saved_count = config.saved_accounts.len();
        for (i, acc) in config.saved_accounts.iter().enumerate() {
            if i == switch_idx {
                println!(" {}{}\u{276f} {}{}", p.accent, b, acc, r);
            } else {
                println!("   {}{}{}", p.label, acc, r);
            }
        }

        // --- Draw action items ---
        let save_label = if let Some(ref alias) = current_alias {
            format!("[Save Current Account] (Saved as '{}')", alias)
        } else {
            "[Save Current Account]".to_string()
        };

        let actions = [
            save_label,
            "[Add Account] (Login via 'agy')".to_string(),
            "[Back to Settings]".to_string(),
        ];
        for (i, label) in actions.iter().enumerate() {
            let idx = saved_count + i;
            if switch_idx == idx {
                println!(" {}{}\u{276f} {}{}", p.accent, b, label, r);
            } else {
                println!("   {}{}{}", p.label, label, r);
            }
        }

        println!("\n{}  Use \u{2191}/\u{2193} to navigate \u{2022} Enter to confirm \u{2022} Esc to cancel{}", p.border, r);
        println!("{}{}{}", p.border, "==============================================================", r);

        // --- Key handling ---
        let total_items = saved_count + 3;
        let sub_key = term.read_key().unwrap_or(Key::Unknown);
        match sub_key {
            Key::ArrowUp => {
                if switch_idx > 0 {
                    switch_idx -= 1;
                } else {
                    switch_idx = total_items - 1;
                }
            }
            Key::ArrowDown => {
                if switch_idx < total_items - 1 {
                    switch_idx += 1;
                } else {
                    switch_idx = 0;
                }
            }
            Key::Enter => {
                // [Back to Settings]
                if switch_idx == saved_count + 2 {
                    break;
                }

                // [Save Current Account]
                if switch_idx == saved_count {
                    if old_main_cred.is_empty() {
                        println!("\n{}[ERROR] No active credentials found to save. Please login first.{}", p.critical, r);
                        let _ = term.read_key();
                        continue;
                    }

                    let cached_email = read_cached_email();
                    let _ = term.show_cursor();
                    print!("\n\u{276f} Account alias: ");
                    let _ = std::io::Write::flush(&mut std::io::stdout());
                    if let Ok(alias) = read_line_with_default(term, &cached_email) {
                        let clean = sanitize_alias(&alias);
                        if !clean.is_empty() {
                            if save_credential_with_alias(&clean, &old_main_cred, &old_local_token, config) {
                                println!("\n{}[SUCCESS] Account successfully encrypted and saved as '{}'.{}", p.success, clean, r);
                            } else {
                                println!("\n{}[ERROR] Failed to write backup to Windows Credential Manager.{}", p.critical, r);
                            }
                        } else {
                            println!("\n{}[ERROR] Invalid alias.{}", p.critical, r);
                        }
                    }
                    let _ = term.hide_cursor();
                    let _ = term.read_key();
                    continue;
                }

                // [Add Account]
                if switch_idx == saved_count + 1 {
                    let _ = term.clear_screen();

                    // Check if the current active account is already saved
                    let current_is_saved = old_main_cred.is_empty()
                        || find_alias_for_credential(&old_main_cred, &config.saved_accounts).is_some();

                    // Force-save unsaved active credentials before switching
                    if !current_is_saved {
                        println!("{}{}{}", p.border, "==============================================================", r);
                        println!("  {}[NOTICE] Unsaved Active Account Detected                    {}", p.warning, r);
                        println!("{}{}{}", p.border, "==============================================================", r);
                        println!("{}You are currently logged in, but this account has no alias.{}", p.label, r);
                        println!("{}To prevent losing it when adding a new account, please save it first.{}", p.label, r);

                        let cached_email = read_cached_email();
                        let _ = term.show_cursor();
                        let mut save_success = false;
                        loop {
                            print!("\n\u{276f} Enter alias for your CURRENT active account: ");
                            let _ = std::io::Write::flush(&mut std::io::stdout());
                            if let Ok(alias) = read_line_with_default(term, &cached_email) {
                                let clean = sanitize_alias(&alias);
                                if !clean.is_empty() {
                                    if save_credential_with_alias(&clean, &old_main_cred, &old_local_token, config) {
                                        println!("\n{}[SUCCESS] Current account saved as '{}'.{}", p.success, clean, r);
                                        save_success = true;
                                        break;
                                    } else {
                                        println!("\n{}[ERROR] Failed to write backup to Windows Credential Manager. Try again.{}", p.critical, r);
                                    }
                                } else {
                                    println!("\n{}[ERROR] Alias cannot be empty.{}", p.critical, r);
                                }
                            } else {
                                break;
                            }
                        }
                        let _ = term.hide_cursor();

                        if !save_success {
                            println!("\n{}[NOTICE] Aborted saving current account. Add Account cancelled.{}", p.warning, r);
                            let _ = term.read_key();
                            continue;
                        }
                    }

                    // Temporarily clear current credentials to force login flow in agy
                    let _ = term.clear_screen();
                    println!("Preparing environment for a new login...");
                    let _ = write_windows_credential("gemini:antigravity", "");
                    let _ = fs::remove_file(&oauth_path);

                    println!("Starting 'agy' CLI to login/switch account...");
                    println!("Please follow the instructions on screen.\n");
                    let _ = term.show_cursor();

                    #[cfg(windows)]
                    let (stdin_h, stdout_h, orig_in_mode, orig_out_mode) = unsafe {
                        use windows_sys::Win32::System::Console::{
                            GetConsoleMode, GetStdHandle, STD_INPUT_HANDLE, STD_OUTPUT_HANDLE,
                        };
                        let stdin_h = GetStdHandle(STD_INPUT_HANDLE);
                        let stdout_h = GetStdHandle(STD_OUTPUT_HANDLE);
                        let mut in_mode = 0;
                        let mut out_mode = 0;
                        let _ = GetConsoleMode(stdin_h, &mut in_mode);
                        let _ = GetConsoleMode(stdout_h, &mut out_mode);
                        (stdin_h, stdout_h, in_mode, out_mode)
                    };

                    let spawn_res = std::process::Command::new("agy")
                        .stdin(std::process::Stdio::inherit())
                        .stdout(std::process::Stdio::inherit())
                        .stderr(std::process::Stdio::inherit())
                        .spawn();

                    match spawn_res {
                        Ok(mut child) => {
                            // Wait for the login to complete by polling credentials
                            loop {
                                match child.try_wait() {
                                    Ok(Some(_status)) => {
                                        break;
                                    }
                                    Ok(None) => {
                                        let check_main = read_windows_credential("gemini:antigravity").unwrap_or_default();
                                        let check_local = fs::read_to_string(&oauth_path).unwrap_or_default();
                                        if !check_main.is_empty() || !check_local.is_empty() {
                                            let _ = child.kill();
                                            let _ = child.wait();
                                            break;
                                        }
                                    }
                                    Err(_) => {
                                        break;
                                    }
                                }
                                std::thread::sleep(std::time::Duration::from_millis(LOGIN_POLL_INTERVAL_MS));
                            }
                        }
                        Err(e) => {
                            println!("\n{}[ERROR] Failed to launch 'agy' CLI: {}{}", p.critical, e, r);
                        }
                    }

                    #[cfg(windows)]
                    unsafe {
                        use windows_sys::Win32::System::Console::{
                            SetConsoleMode, FlushConsoleInputBuffer,
                        };
                        let _ = SetConsoleMode(stdin_h, orig_in_mode);
                        let _ = SetConsoleMode(stdout_h, orig_out_mode);
                        // Discard any residual input events left by the agy process
                        // (e.g. Enter key from spinner) to prevent read_line_with_default
                        // from consuming stale keystrokes.
                        let _ = FlushConsoleInputBuffer(stdin_h);
                    }

                    // Check if new credentials were created
                    let new_main_cred = read_windows_credential("gemini:antigravity").unwrap_or_default();
                    let new_local_token = fs::read_to_string(&oauth_path).unwrap_or_default();

                    if !new_main_cred.is_empty() || !new_local_token.is_empty() {
                        println!("\n'agy' process finished. New account detected.");

                        let effective_cred = if !new_main_cred.is_empty() {
                            new_main_cred.clone()
                        } else {
                            new_local_token.clone()
                        };

                        // Check if this account has already been saved with an alias
                        if let Some(existing) = find_alias_for_credential(&effective_cred, &config.saved_accounts) {
                            println!("\n{}[NOTICE] This account is already saved as '{}'.{}", p.warning, existing, r);
                            let _ = term.read_key();
                            continue;
                        }

                        let _ = term.show_cursor();
                        print!("\u{276f} Save this new account with alias: ");
                        let _ = std::io::Write::flush(&mut std::io::stdout());
                        if let Ok(alias) = read_line_with_default(term, "") {
                            let clean = sanitize_alias(&alias);
                            if !clean.is_empty() {
                                if save_credential_with_alias(&clean, &effective_cred, &new_local_token, config) {
                                    println!("\n{}[SUCCESS] Account successfully encrypted and saved as '{}'.{}", p.success, clean, r);
                                } else {
                                    println!("\n{}[ERROR] Failed to write backup to Windows Credential Manager.{}", p.critical, r);
                                }
                            } else {
                                println!("\n{}[ERROR] Invalid alias.{}", p.critical, r);
                            }
                        }
                    } else {
                        // Restore previous credentials
                        println!("\n{}[NOTICE] No new credentials detected. Restoring previous account state...{}", p.warning, r);
                        if !old_main_cred.is_empty() {
                            let _ = write_windows_credential("gemini:antigravity", &old_main_cred);
                        }
                        if !old_local_token.is_empty() {
                            let _ = fs::write(&oauth_path, &old_local_token);
                        }
                    }

                    let _ = term.hide_cursor();
                    let _ = term.read_key();
                    continue;
                }

                // Switch to an existing saved account
                let target_alias = &config.saved_accounts[switch_idx];
                let target_name = format!("gemini:antigravity:{}", target_alias);
                let oauth_target_name = format!("gemini:antigravity:oauth-token:{}", target_alias);

                if let Some(backup_cred) = read_windows_credential(&target_name) {
                    let write_ok = write_windows_credential("gemini:antigravity", &backup_cred);

                    // Also write back local oauth-token file
                    let file_write_ok = if let Some(local_token) = read_windows_credential(&oauth_target_name) {
                        fs::write(&oauth_path, &local_token).is_ok()
                    } else {
                        fs::write(&oauth_path, &backup_cred).is_ok()
                    };

                    if write_ok || file_write_ok {
                        let status_cache_path = resolve_antigravity_path("statusline-cache.json");
                        let _ = fs::remove_file(&status_cache_path);
                        println!("\n{}[SUCCESS] Credentials successfully switched to '{}'.{}", p.success, target_alias, r);
                        println!("{}{}[SUCCESS] Cache cleared. The statusline will refresh on the next command execution.{}", p.success, b, r);
                    } else {
                        println!("\n{}[ERROR] Failed to write credentials back to system storage.{}", p.critical, r);
                    }
                } else {
                    println!("\n{}[ERROR] Failed to read backup credential '{}' from Windows Credential Manager.{}", p.critical, target_alias, r);
                }
                let _ = term.read_key();
            }
            Key::Escape => break,
            _ => {}
        }
    }
}

// --- Terminal input helper ---------------------------------------------------

fn read_line_with_default(term: &console::Term, default: &str) -> std::io::Result<String> {
    use console::Key;

    term.write_str(default)?;
    let mut input: Vec<char> = default.chars().collect();

    loop {
        match term.read_key()? {
            Key::Char(c) => {
                if !c.is_control() && c != ':' {
                    input.push(c);
                    term.write_str(&c.to_string())?;
                }
            }
            Key::Backspace => {
                if !input.is_empty() {
                    input.pop();
                    term.write_str("\x08 \x08")?;
                }
            }
            Key::Enter => {
                term.write_line("")?;
                break;
            }
            Key::Escape => {
                term.write_line("")?;
                return Ok(String::new());
            }
            _ => {}
        }
    }

    Ok(input.into_iter().collect())
}
