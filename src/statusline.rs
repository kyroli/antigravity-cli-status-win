use std::fs;
use std::io::Read;
use std::path::Path;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};
use agy_statusline_lib::{
    get_access_token, get_configs_last_modified_time, get_human_format, get_short_model_name,
    get_shorten_path, parse_input_json, resolve_antigravity_path, CacheData, InputJson,
    QuotaItem, VcsInfo, load_user_config, UserConfig,
};

fn sha256(data: &[u8]) -> [u8; 32] {
    let mut h: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a,
        0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19,
    ];

    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
        0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
        0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
        0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
        0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
        0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
        0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
    ];

    let mut padded = data.to_vec();
    let original_len_bits = (data.len() as u64) * 8;
    padded.push(0x80);
    while (padded.len() + 8) % 64 != 0 {
        padded.push(0);
    }
    padded.extend_from_slice(&original_len_bits.to_be_bytes());

    for chunk in padded.chunks_exact(64) {
        let mut w = [0u32; 64];
        for i in 0..16 {
            w[i] = u32::from_be_bytes([
                chunk[i * 4],
                chunk[i * 4 + 1],
                chunk[i * 4 + 2],
                chunk[i * 4 + 3],
            ]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }

        let mut a = h[0];
        let mut b = h[1];
        let mut c = h[2];
        let mut d = h[3];
        let mut e = h[4];
        let mut f = h[5];
        let mut g = h[6];
        let mut h_val = h[7];

        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ (!e & g);
            let temp1 = h_val
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(K[i])
                .wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let temp2 = s0.wrapping_add(maj);

            h_val = g;
            g = f;
            f = e;
            e = d.wrapping_add(temp1);
            d = c;
            c = b;
            b = a;
            a = temp1.wrapping_add(temp2);
        }

        h[0] = h[0].wrapping_add(a);
        h[1] = h[1].wrapping_add(b);
        h[2] = h[2].wrapping_add(c);
        h[3] = h[3].wrapping_add(d);
        h[4] = h[4].wrapping_add(e);
        h[5] = h[5].wrapping_add(f);
        h[6] = h[6].wrapping_add(g);
        h[7] = h[7].wrapping_add(h_val);
    }

    let mut result = [0u8; 32];
    for i in 0..8 {
        let bytes = h[i].to_be_bytes();
        result[i * 4..i * 4 + 4].copy_from_slice(&bytes);
    }
    result
}

fn sha256_hex(data: &str) -> String {
    let hash = sha256(data.as_bytes());
    let mut s = String::with_capacity(64);
    const HEX_CHARS: &[u8; 16] = b"0123456789abcdef";
    for &byte in &hash {
        s.push((HEX_CHARS[(byte >> 4) as usize]) as char);
        s.push((HEX_CHARS[(byte & 0xf) as usize]) as char);
    }
    s
}

// --- Theme & Visual Enhancement Systems --------------------------------------

struct ResolvedTheme {
    border: String,
    label: String,
    accent: String,
    success: String,
    warning: String,
    critical: String,
    state_ready: String,
    state_thinking: String,
    state_working: String,
    state_tool_use: String,
    state_default: String,
}

impl ResolvedTheme {
    fn resolve(config: &UserConfig) -> Self {
        match config.theme.to_lowercase().as_str() {
            "pastel" => {
                let border = "\x1b[38;2;88;91;112m".to_string();
                let label = "\x1b[38;2;205;214;244m".to_string();
                let accent = "\x1b[38;2;203;166;247m".to_string();
                let success = "\x1b[38;2;166;227;161m".to_string();
                let warning = "\x1b[38;2;249;226;175m".to_string();
                let critical = "\x1b[38;2;243;139;168m".to_string();
                Self {
                    state_ready: format!("{}{}[READY]\x1b[0m", success, "\x1b[1m"),
                    state_thinking: format!("{}{}[THINKING]\x1b[0m", warning, "\x1b[1m"),
                    state_working: format!("{}{}[WORKING]\x1b[0m", accent, "\x1b[1m"),
                    state_tool_use: format!("{}{}[TOOL]\x1b[0m", accent, "\x1b[1m"),
                    state_default: format!("{}{}[STATE]\x1b[0m", label, "\x1b[1m"),
                    border,
                    label,
                    accent,
                    success,
                    warning,
                    critical,
                }
            }
            "neon" => {
                let border = "\x1b[38;2;59;66;97m".to_string();
                let label = "\x1b[38;2;169;177;214m".to_string();
                let accent = "\x1b[38;2;125;207;255m".to_string();
                let success = "\x1b[38;2;115;218;202m".to_string();
                let warning = "\x1b[38;2;255;158;100m".to_string();
                let critical = "\x1b[38;2;247;118;142m".to_string();
                Self {
                    state_ready: format!("{}{}[READY]\x1b[0m", success, "\x1b[1m"),
                    state_thinking: format!("{}{}[THINKING]\x1b[0m", warning, "\x1b[1m"),
                    state_working: format!("{}{}[WORKING]\x1b[0m", accent, "\x1b[1m"),
                    state_tool_use: format!("{}{}[TOOL]\x1b[0m", accent, "\x1b[1m"),
                    state_default: format!("{}{}[STATE]\x1b[0m", label, "\x1b[1m"),
                    border,
                    label,
                    accent,
                    success,
                    warning,
                    critical,
                }
            }
            "frost" => {
                let border = "\x1b[38;2;76;86;106m".to_string();
                let label = "\x1b[38;2;216;222;233m".to_string();
                let accent = "\x1b[38;2;136;192;208m".to_string();
                let success = "\x1b[38;2;163;190;140m".to_string();
                let warning = "\x1b[38;2;235;203;139m".to_string();
                let critical = "\x1b[38;2;191;97;106m".to_string();
                Self {
                    state_ready: format!("{}{}[READY]\x1b[0m", success, "\x1b[1m"),
                    state_thinking: format!("{}{}[THINKING]\x1b[0m", warning, "\x1b[1m"),
                    state_working: format!("{}{}[WORKING]\x1b[0m", accent, "\x1b[1m"),
                    state_tool_use: format!("{}{}[TOOL]\x1b[0m", accent, "\x1b[1m"),
                    state_default: format!("{}{}[STATE]\x1b[0m", label, "\x1b[1m"),
                    border,
                    label,
                    accent,
                    success,
                    warning,
                    critical,
                }
            }
            _ => {
                let border = String::from("\x1b[90m");
                let label = String::from("\x1b[37m");
                let accent = String::from("\x1b[94m");
                let success = String::from("\x1b[96m");
                let warning = String::from("\x1b[93m");
                let critical = "\x1b[91m".to_string();
                Self {
                    state_ready: String::from("\x1b[92m\x1b[1m[READY]\x1b[0m"),
                    state_thinking: String::from("\x1b[93m\x1b[1m[THINKING]\x1b[0m"),
                    state_working: String::from("\x1b[96m\x1b[1m[WORKING]\x1b[0m"),
                    state_tool_use: String::from("\x1b[95m\x1b[1m[TOOL]\x1b[0m"),
                    state_default: String::from("\x1b[97m\x1b[1m[STATE]\x1b[0m"),
                    border,
                    label,
                    accent,
                    success,
                    warning,
                    critical,
                }
            }
        }
    }
}

fn get_icon(widget: &str) -> &'static str {
    match widget {
        "vcs" => "\u{e0a0} ",
        "path" => "\u{f07c} ",
        "quota" => "\u{26a1}",
        "context" => "\u{f061a} ",
        "cache" => "\u{f0a0} ",
        "artifacts" => "\u{f09d1} ",
        "subagents" => "\u{f06a9} ",
        "tasks" => "\u{f051b} ",
        "sandbox" => "\u{f132} ",
        _ => "",
    }
}

fn render_progress_bar(pct: f64, bar_len: usize, color: &str, is_quota: bool) -> String {
    if bar_len == 0 {
        return String::new();
    }
    let mut bar = String::new();
    bar.push_str("\x1b[38;2;80;80;80m[");
    let filled = ((pct / 100.0) * (bar_len as f64)).round() as usize;
    let filled = std::cmp::min(bar_len, filled);
    if is_quota {
        bar.push_str(color);
        for _ in 0..filled {
            bar.push('-');
        }
        bar.push_str("\x1b[38;2;80;80;80m");
        for _ in filled..bar_len {
            bar.push('-');
        }
    } else {
        bar.push_str(color);
        for i in 0..bar_len {
            if i < filled {
                if i == filled - 1 {
                    bar.push('>');
                } else {
                    bar.push('=');
                }
            } else {
                if i == filled {
                    bar.push_str("\x1b[38;2;80;80;80m");
                }
                bar.push('-');
            }
        }
    }
    bar.push_str("\x1b[38;2;80;80;80m]\x1b[0m");
    bar
}

// --- Config Configuration ----------------------------------------------------
// (Runtime configuration is loaded via load_user_config)

// --- Render Widgets Definitions ----------------------------------------------

#[derive(Clone)]
struct Widget {
    text: String,
    len: usize,
}

fn get_visual_length(s: &str) -> usize {
    let mut len = 0;
    let mut in_ansi = false;
    let chars: Vec<char> = s.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '\x1b' && i + 1 < chars.len() && chars[i + 1] == '[' {
            in_ansi = true;
            i += 2;
            continue;
        }
        if in_ansi {
            if chars[i].is_ascii_alphabetic() {
                in_ansi = false;
            }
            i += 1;
            continue;
        }
        len += 1;
        i += 1;
    }
    len
}

fn w(text: String) -> Widget {
    let len = get_visual_length(&text);
    Widget { text, len }
}

// --- TUI Layout Rendering ----------------------------------------------------

fn get_model_quota_string(config: &UserConfig, cache: &CacheData, current_model: &str, hide_time: bool) -> String {
    let theme = ResolvedTheme::resolve(config);

    if cache.needs_login == Some(true) {
        let icon = get_icon("quota");
        return format!("{}{}{}\x1b[1m[LOGIN]\x1b[0m", theme.label, icon, theme.warning);
    }

    let clean_name = |n: &str| n.to_lowercase().replace(|c: char| !c.is_alphanumeric(), "");
    let target_clean = clean_name(current_model);

    let matched = cache.quota.iter().find(|item| {
        clean_name(&item.displayName) == target_clean || clean_name(&item.id) == target_clean
    }).or_else(|| {
        cache.quota.iter().find(|item| {
            target_clean.contains(&clean_name(&item.displayName)) || clean_name(&item.displayName).contains(&target_clean)
        })
    });

    if let Some(item) = matched {
        let pct = (item.remainingFraction * 100.0).floor() as i64;
        let mut time_str = String::new();

        if let Some(ref r_time) = item.resetTime {
            if !hide_time {
                if let Some(parsed_time) = agy_statusline_lib::parse_rfc3339_to_unix(r_time) {
                    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
                    if parsed_time > now {
                        let diff_mins = (parsed_time as u64 - now) / 60;
                        let diff_hours = diff_mins / 60;
                        let diff_days = diff_hours / 24;

                        let raw_time = if diff_days >= 1 {
                            format!("~{}d{}h", diff_days, diff_hours % 24)
                        } else if diff_hours >= 1 {
                            format!("~{}h{}m", diff_hours, diff_mins % 60)
                        } else {
                            format!("~{}m", diff_mins)
                        };
                        time_str = format!(" {}({})\x1b[0m", theme.label, raw_time);
                    }
                }
            }
        }

        let (color, active_color) = if pct <= 20 {
            (format!("{}{}", theme.critical, "\x1b[1m"), theme.critical.as_str())
        } else if pct <= 50 {
            (theme.warning.clone(), theme.warning.as_str())
        } else {
            (theme.success.clone(), theme.success.as_str())
        };

        let icon = get_icon("quota");
        if !hide_time {
            let bar_len = 5;
            let bar = render_progress_bar(item.remainingFraction * 100.0, bar_len, active_color, true);
            format!("{}{}{}{}", theme.label, icon, bar, time_str)
        } else {
            format!("{}{}{}{}%\x1b[0m{}", theme.label, icon, color, pct, time_str)
        }
    } else {
        String::new()
    }
}

fn get_info_widgets(config: &UserConfig, json: &InputJson, cache: &CacheData, step: usize, cols: usize) -> Vec<Widget> {
    let mut list = Vec::new();
    let theme = ResolvedTheme::resolve(config);

    if true {
        let state = json.agent_state.as_deref().unwrap_or("idle");
        let state_text = match state {
            "idle" => &theme.state_ready,
            "thinking" => &theme.state_thinking,
            "working" => &theme.state_working,
            "tool_use" => &theme.state_tool_use,
            _ => &theme.state_default,
        };
        list.push(w(state_text.clone()));
    }

    if let Some(ref agent) = json.agent {
        if let Some(ref name) = agent.name {
            let name_lower = name.to_lowercase();
            let special_mode = if name_lower.contains("grill") {
                Some(format!("{}{}[GRILLME]\x1b[0m", theme.critical, "\x1b[1m"))
            } else if name_lower.contains("plan") {
                Some(format!("{}{}[PLAN]\x1b[0m", theme.accent, "\x1b[1m"))
            } else if name_lower.contains("goal") {
                Some(format!("{}{}[GOAL]\x1b[0m", theme.success, "\x1b[1m"))
            } else if name_lower != "default" && name_lower != "main" && !name_lower.is_empty() {
                Some(format!("{}{}[{}]\x1b[0m", theme.accent, "\x1b[1m", name.to_uppercase()))
            } else {
                None
            };
            if let Some(mode_text) = special_mode {
                list.push(w(mode_text));
            }
        }
        
        if let Some(ref status) = agent.status {
            if !status.is_empty() && status != "idle" {
                list.push(w(format!("{}{}\x1b[0m", theme.accent, status)));
            }
        }
    }

    if true && json.tool_confirmation_pending.unwrap_or(false) {
        list.push(w(format!("{}{}[! PENDING APPROVAL]\x1b[0m", theme.critical, "\x1b[1m")));
    }

    if true {
        let p_input = json.pending_input_count.unwrap_or(0);
        if p_input > 0 {
            list.push(w(format!("{}> {}\x1b[0m", theme.warning, p_input)));
        }
    }

    let raw_model = json.model.as_ref().and_then(|m| m.display_name.as_deref()).unwrap_or("");
    if !raw_model.is_empty() && (true || true) {
        let q_info = if true {
            get_model_quota_string(config, cache, raw_model, step >= 6 || cols < 80)
        } else {
            String::new()
        };
        
        let show_model_name = true;
        let mut model_part = if step >= 4 { get_short_model_name(raw_model) } else { raw_model.to_string() };
        
        if true {
            if let Some(ref tier) = json.plan_tier {
                if !tier.is_empty() {
                    model_part = format!("{} [{}]", model_part, tier);
                }
            }
        }
        
        let text = if show_model_name && !q_info.is_empty() {
            format!("{}{}\x1b[0m {}|{}\x1b[0m {}", theme.accent, model_part, theme.border, theme.border, q_info)
        } else if show_model_name {
            format!("{}{}\x1b[0m", theme.accent, model_part)
        } else {
            q_info
        };
        if !text.is_empty() {
            list.push(w(text));
        }
    }

    let raw_cwd = json.workspace.as_ref().and_then(|w| w.current_dir.as_deref()).or_else(|| json.cwd.as_deref()).unwrap_or("");
    if true && !raw_cwd.is_empty() && step < 5 {
        let path_text = if step >= 3 {
            raw_cwd.replace('\\', "/").split('/').last().unwrap_or(raw_cwd).to_string()
        } else {
            get_shorten_path(raw_cwd)
        };
        let icon = get_icon("path");
        list.push(w(format!("{}{}{}\x1b[0m", theme.label, icon, path_text)));
    }

    if true {
        if let Some(ref vcs) = cache.vcs {
            if vcs.cwd == raw_cwd && !vcs.branch.is_empty() && step < 6 {
                let mut branch_text = vcs.branch.clone();
                if step >= 4 {
                    if branch_text.len() > 10 {
                        branch_text = format!("{}..", &branch_text[..8]);
                    }
                } else if branch_text.len() > 15 {
                    branch_text = format!("{}..", &branch_text[..12]);
                }
                let icon = get_icon("vcs");
                let label = format!("{}{}", icon, branch_text);

                let mut git_extra = String::new();
                if vcs.dirty {
                    if vcs.modified > 0 && step < 4 {
                        git_extra.push_str(&format!("*{}", vcs.modified));
                    } else {
                        git_extra.push('*');
                    }
                }
                if step < 4 {
                    if vcs.ahead > 0 && vcs.behind > 0 {
                        git_extra.push_str(&format!(" ↑{}↓{}", vcs.ahead, vcs.behind));
                    } else if vcs.ahead > 0 {
                        git_extra.push_str(&format!(" ↑{}", vcs.ahead));
                    } else if vcs.behind > 0 {
                        git_extra.push_str(&format!(" ↓{}", vcs.behind));
                    }
                }

                let fmt = if !git_extra.is_empty() {
                    if vcs.dirty {
                        format!("{}{}{}{}\x1b[0m", theme.label, label, theme.warning, git_extra)
                    } else {
                        format!("{}{}{}{}\x1b[0m", theme.label, label, theme.border, git_extra)
                    }
                } else {
                    format!("{}{}\x1b[0m", theme.label, label)
                };
                list.push(w(fmt));
            }
        }
    }

    if step < 2 {
        if true {
            if let Some(ref email) = json.email {
                if !email.is_empty() { list.push(w(email.to_string())); }
            }
        }
        if true {
            if let Some(ref ver) = json.version {
                if !ver.is_empty() { list.push(w(format!("v{}", ver))); }
            }
        }
        if true {
            if let Some(ref cid) = json.conversation_id {
                if !cid.is_empty() {
                    let limit = std::cmp::min(8, cid.len());
                    list.push(w(format!("id:{}", &cid[..limit])));
                }
            }
        }
    }

    list
}

fn get_metric_widgets(config: &UserConfig, json: &InputJson, step: usize) -> Vec<Widget> {
    let mut list = Vec::new();
    let theme = ResolvedTheme::resolve(config);

    if true && step < 11 {
        let (bar_len, detail_mode) = if step >= 10 {
            (0, 3)
        } else if step >= 9 {
            (3, 3)
        } else if step >= 7 {
            (5, 3)
        } else if step >= 6 {
            (6, 2)
        } else if step >= 5 {
            (8, 1)
        } else {
            (10, 0)
        };

        let cw = json.context_window.as_ref();
        let pct = cw.and_then(|c| c.used_percentage).unwrap_or(0.0);
        let input_tok = cw.and_then(|c| c.total_input_tokens).unwrap_or(0);
        let output_tok = cw.and_then(|c| c.total_output_tokens).unwrap_or(0);
        let limit_tok = cw.and_then(|c| c.context_window_size).unwrap_or(0);

        let cu = cw.and_then(|c| c.current_usage.as_ref());
        let cache_read = cu.and_then(|u| u.cache_read_input_tokens).unwrap_or(0);
        let cache_create = cu.and_then(|u| u.cache_creation_input_tokens).unwrap_or(0);

        let total_used = input_tok + output_tok;
        let pct = if limit_tok > 0 {
            (total_used as f64 / limit_tok as f64) * 100.0
        } else {
            pct
        };

        let bar_color = if pct >= 90.0 { theme.critical.as_str() } else if pct >= 60.0 { theme.warning.as_str() } else { theme.accent.as_str() };
        let bar_text = render_progress_bar(pct, bar_len, bar_color, false);

        let mut detail_text = String::new();
        if detail_mode == 0 {
            detail_text = format!(" ({}/{})", get_human_format(total_used), get_human_format(limit_tok));
        } else if detail_mode == 1 {
            if limit_tok > 0 {
                let free_pct = 100.0 - pct;
                let free_tok = limit_tok.saturating_sub(total_used);
                detail_text = format!(" (free:{:.1}%/{})", free_pct, get_human_format(free_tok));
            }
        } else if detail_mode == 2 && total_used > 0 && limit_tok > 0 {
            detail_text = format!(" ({}/{})", get_human_format(total_used), get_human_format(limit_tok));
        }

        let icon = get_icon("context");
        let full_text = if bar_len > 0 {
            format!("{}{}{}ctx\x1b[0m {} {}{}{:.1}%\x1b[0m{}{}\x1b[0m", theme.label, icon, theme.label, bar_text, theme.accent, "\x1b[1m", pct, theme.label, detail_text)
        } else {
            format!("{}{}{}ctx\x1b[0m {}{}{:.1}%\x1b[0m", theme.label, icon, theme.label, theme.accent, "\x1b[1m", pct)
        };
        list.push(w(full_text));

        if true && step < 3 && (cache_read > 0 || cache_create > 0) {
            let rd_fmt = get_human_format(cache_read);
            let icon = get_icon("cache");
            let cache_text = if cache_create > 0 {
                let wr_fmt = get_human_format(cache_create);
                format!("{}{}{}cache\x1b[0m {}{}rd:{}/wr:{}\x1b[0m", theme.label, icon, theme.label, theme.accent, "\x1b[1m", rd_fmt, wr_fmt)
            } else {
                format!("{}{}{}cache\x1b[0m {}{}rd:{}\x1b[0m", theme.label, icon, theme.label, theme.accent, "\x1b[1m", rd_fmt)
            };
            list.push(w(cache_text));
        }
    }

    if true {
        let artifacts = json.artifacts.as_ref().map(|a| a.len()).or(json.artifact_count.map(|c| c as usize)).unwrap_or(0);
        if artifacts > 0 && step < 6 {
            let icon = get_icon("artifacts");
            list.push(w(format!("{}{}{}artifacts\x1b[0m {}{}{}\x1b[0m", theme.label, icon, theme.label, theme.accent, "\x1b[1m", artifacts)));
        }
    }

    if true {
        let subs = json.subagents.as_ref().map(|s| s.len()).unwrap_or(0);
        if subs > 0 && step < 8 {
            let icon = get_icon("subagents");
            list.push(w(format!("{}{}{}subagents\x1b[0m {}{}{}\x1b[0m", theme.label, icon, theme.label, theme.accent, "\x1b[1m", subs)));
        }
    }

    if true {
        let tasks = json.background_tasks.as_ref().map(|t| t.len()).or(json.task_count.map(|c| c as usize)).unwrap_or(0);
        if tasks > 0 && step < 8 {
            let icon = get_icon("tasks");
            list.push(w(format!("{}{}{}tasks\x1b[0m {}{}{}\x1b[0m", theme.label, icon, theme.label, theme.accent, "\x1b[1m", tasks)));
        }
    }

    if true {
        if let Some(ref sb) = json.sandbox {
            if sb.enabled.unwrap_or(false) && step < 4 {
                let icon = get_icon("sandbox");
                let sb_text = if sb.allow_network.unwrap_or(false) {
                    format!("{}{}{}sandbox\x1b[0m {}{}ON(net)\x1b[0m", theme.label, icon, theme.label, theme.success, "\x1b[1m")
                } else {
                    format!("{}{}{}sandbox\x1b[0m {}{}ON(no-net)\x1b[0m", theme.label, icon, theme.label, theme.success, "\x1b[1m")
                };
                list.push(w(sb_text));
            }
        }
    }

    list
}

// --- Get Terminal Title String -----------------------------------------------
fn get_title_string(json: &InputJson) -> String {
    let state = json.agent_state.as_deref().unwrap_or("idle");
    let raw_cwd = json.workspace.as_ref().and_then(|w| w.current_dir.clone()).or_else(|| json.cwd.clone()).unwrap_or_default();

    let mut workspace = "unknown".to_string();
    if !raw_cwd.is_empty() {
        let cwd_norm = raw_cwd.replace('\\', "/");
        if let Some(pos) = cwd_norm.find("/google/src/cloud/") {
            let sub = &cwd_norm[pos + "/google/src/cloud/".len()..];
            let parts: Vec<&str> = sub.split('/').filter(|s| !s.is_empty()).collect();
            if parts.len() >= 2 {
                workspace = parts[1].to_string();
            } else if let Some(last) = parts.first() {
                workspace = last.to_string();
            }
        } else {
            workspace = cwd_norm.split('/').last().unwrap_or("unknown").to_string();
        }
    }

    let emoji = match state {
        "initializing" => "🚀",
        "idle" => "😴",
        "thinking" => "🤔",
        "working" => "🏃",
        "tool_use" => "🛠️",
        _ => "🤖",
    };

    let mut agent_mode = String::new();
    let mut status_str = String::new();
    if let Some(ref agent) = json.agent {
        if let Some(ref name) = agent.name {
            let name_lower = name.to_lowercase();
            if name_lower.contains("grill") {
                agent_mode = " [GRILL]".to_string();
            } else if name_lower.contains("plan") {
                agent_mode = " [PLAN]".to_string();
            } else if name_lower.contains("goal") {
                agent_mode = " [GOAL]".to_string();
            } else if name_lower != "default" && name_lower != "main" && !name_lower.is_empty() {
                agent_mode = format!(" [{}]", name.to_uppercase());
            }
        }
        if let Some(ref status) = agent.status {
            if !status.is_empty() && status != "idle" {
                status_str = format!(" - {}", status);
            }
        }
    }

    format!("{}{} {}{} | {}", emoji, agent_mode, state, status_str, workspace)
}

fn render_tui(config: &UserConfig, json: &InputJson, cache: &CacheData) {
    let cols = json.terminal_width.unwrap_or(80);
    let theme = ResolvedTheme::resolve(config);

    let max_w = if cols >= 80 { cols - 4 } else { cols - 2 };
    let max_metric_w = if cols >= 80 { cols - 5 } else { cols - 2 };

    let get_row_width = |widgets: &[Widget], sep_len: usize| -> usize {
        if widgets.is_empty() { return 0; }
        let total: usize = widgets.iter().map(|w| w.len).sum();
        total + sep_len * (widgets.len() - 1)
    };

    let (min_info_step, min_metric_step) = if cols >= 160 {
        (0, 0)
    } else if cols >= 120 {
        (3, 0)
    } else if cols >= 80 {
        (3, 5)
    } else if cols >= 60 {
        (5, 6)
    } else {
        (6, 6)
    };

    let sep = format!(" {}|{} ", theme.border, theme.border);

    let mut single_line_rendered = None;
    if cols >= 160 {
        for s in min_info_step..=11 {
            let s_info = std::cmp::min(s, 6);
            let s_metric = std::cmp::min(s, 11);
            let mut combined = get_info_widgets(config, json, cache, s_info, cols);
            combined.extend(get_metric_widgets(config, json, s_metric));

            if get_row_width(&combined, 3) <= max_w && s <= 2 {
                let texts: Vec<String> = combined.into_iter().map(|w| w.text).collect();
                single_line_rendered = Some(texts.join(&sep));
                break;
            }
        }
    }

    let mut rendered_rows = Vec::new();
    if let Some(single) = single_line_rendered {
        rendered_rows.push(single);
    } else {
        let mut info_widgets = Vec::new();
        for s in min_info_step..=6 {
            let widgets = get_info_widgets(config, json, cache, s, cols);
            if get_row_width(&widgets, 3) <= max_w {
                info_widgets = widgets;
                break;
            }
            if s == 6 { info_widgets = widgets; }
        }

        let mut metric_widgets = Vec::new();
        for s in min_metric_step..=11 {
            let widgets = get_metric_widgets(config, json, s);
            if get_row_width(&widgets, 3) <= max_metric_w {
                metric_widgets = widgets;
                break;
            }
            if s == 11 { metric_widgets = widgets; }
        }

        let info_row = info_widgets.into_iter().map(|w| w.text).collect::<Vec<String>>().join(&sep);
        rendered_rows.push(info_row);

        if !metric_widgets.is_empty() {
            let metric_row = metric_widgets.into_iter().map(|w| w.text).collect::<Vec<String>>().join(&sep);
            rendered_rows.push(metric_row);
        }
    }

    if cols >= 80 {
        if rendered_rows.len() == 1 {
            println!("{}╭─\x1b[0m {}", theme.border, rendered_rows[0]);
        } else if rendered_rows.len() == 2 {
            println!("{}╭─\x1b[0m {}", theme.border, rendered_rows[0]);
            println!("{}╰─\x1b[0m {}", theme.border, rendered_rows[1]);
        } else if rendered_rows.len() > 2 {
            println!("{}╭─\x1b[0m {}", theme.border, rendered_rows[0]);
            for i in 1..rendered_rows.len() - 1 {
                println!("{}├─\x1b[0m {}", theme.border, rendered_rows[i]);
            }
            println!("{}╰─\x1b[0m {}", theme.border, rendered_rows[rendered_rows.len() - 1]);
        }
    } else {
        for row in rendered_rows {
            println!("{}", row);
        }
    }
}

// --- Background Refresh Logic ------------------------------------------------

fn run_background_refresh(cwd_force: Option<String>) {
    #[cfg(windows)]
    let _mutex = match agy_statusline_lib::NamedMutex::acquire("Local\\AgyStatuslineRefreshMutex") {
        Some(m) => m,
        None => return,
    };

    let status_cache_path = resolve_antigravity_path("statusline-cache.json");
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();

    let mut existing_cache = CacheData::default();
    if let Ok(cache_str) = fs::read_to_string(&status_cache_path) {
        if let Ok(parsed) = serde_json::from_str::<CacheData>(&cache_str) {
            existing_cache = parsed;
        }
    }

    let last_config_update = get_configs_last_modified_time();
    let mut cache_modified_secs = 0u64;
    if let Ok(metadata) = fs::metadata(&status_cache_path) {
        if let Ok(modified) = metadata.modified() {
            if let Ok(duration) = modified.duration_since(UNIX_EPOCH) {
                cache_modified_secs = duration.as_secs();
            }
        }
    }

    let token_opt = get_access_token();
    let current_token_hash = token_opt.as_ref().map(|t| sha256_hex(t));
    let token_changed = current_token_hash != existing_cache.token_hash;

    if token_changed {
        existing_cache.needs_login = None;
    }

    let quota_age = now.saturating_sub(existing_cache.lastRefreshed);
    let need_quota_fetch = (existing_cache.quota.is_empty() 
        || quota_age > 120 
        || last_config_update > cache_modified_secs
        || token_changed)
        && token_opt.is_some()
        && existing_cache.needs_login != Some(true);

    if token_opt.is_none() {
        existing_cache.token_hash = None;
        existing_cache.needs_login = Some(true);
        existing_cache.lastRefreshed = now;
        existing_cache.quota.clear();
    } else if need_quota_fetch {
        if let Some(ref token) = token_opt {
            let endpoints = [
                "https://cloudcode-pa.googleapis.com",
            ];

            use ureq::tls::{TlsConfig, RootCerts};

            let agent: ureq::Agent = ureq::Agent::config_builder()
                .timeout_connect(Some(std::time::Duration::from_secs(3)))
                .timeout_recv_body(Some(std::time::Duration::from_secs(3)))
                .http_status_as_error(false)
                .tls_config(
                    TlsConfig::builder()
                        .root_certs(RootCerts::PlatformVerifier)
                        .build()
                )
                .build()
                .into();

            let mut quota_list: Vec<QuotaItem> = Vec::new();
            let mut auth_error_occurred = false;
            let mut fetch_ok = false;

            for ep in &endpoints {
                let res = agent.post(&format!("{}/v1internal:fetchAvailableModels", ep))
                    .header("Authorization", &format!("Bearer {}", token))
                    .header("Content-Type", "application/json")
                    .header("User-Agent", "antigravity/1.0.0 windows/amd64")
                    .send_json(&serde_json::json!({}));

                match res {
                    Ok(mut resp) => {
                        let status = resp.status();
                        if status == 200 {
                            if let Ok(json_body) = resp.body_mut().read_json::<serde_json::Value>() {
                                if let Some(models) = json_body.get("models").and_then(|m| m.as_object()) {
                                    for (key, model_val) in models {
                                        if let Some(quota_info) = model_val.get("quotaInfo") {
                                            let remaining_fraction = quota_info.get("remainingFraction")
                                                .and_then(|v| v.as_f64())
                                                .unwrap_or(0.0);
                                            let reset_time = quota_info.get("resetTime")
                                                .and_then(|v| v.as_str())
                                                .map(|s| s.to_string());
                                            let display_name = model_val.get("displayName")
                                                .and_then(|v| v.as_str())
                                                .unwrap_or(key)
                                                .to_string();

                                            quota_list.push(QuotaItem {
                                                id: key.clone(),
                                                displayName: display_name,
                                                remainingFraction: remaining_fraction,
                                                resetTime: reset_time,
                                            });
                                        }
                                    }
                                    fetch_ok = true;
                                    break;
                                }
                            }
                        } else if status == 401 || status == 403 {
                            auth_error_occurred = true;
                            break;
                        }
                    }
                    _ => {}
                }
            }

            if auth_error_occurred {
                existing_cache.needs_login = Some(true);
                existing_cache.token_hash = current_token_hash.clone();
                existing_cache.quota.clear();
                existing_cache.lastRefreshed = now;
            } else if fetch_ok {
                existing_cache.needs_login = Some(false);
                existing_cache.token_hash = current_token_hash.clone();
                existing_cache.quota = quota_list;
                existing_cache.lastRefreshed = now;
            } else {
                existing_cache.lastRefreshed = now;
            }
        }
    }

    let mut git_branch = String::new();
    let mut git_dirty = false;
    let mut git_ahead = 0u32;
    let mut git_behind = 0u32;
    let mut git_modified = 0u32;

    if let Some(ref cwd) = cwd_force {
        if Path::new(cwd).exists() {
            if let Some(branch) = agy_statusline_lib::get_git_branch_fast(cwd) {
                git_branch = branch;
                if let Ok(status_out) = Command::new("git")
                    .args(["status", "--porcelain"])
                    .current_dir(cwd)
                    .output()
                {
                    let clean_status = String::from_utf8_lossy(&status_out.stdout);
                    let count = clean_status.lines().filter(|l| !l.trim().is_empty()).count() as u32;
                    git_dirty = count > 0;
                    git_modified = count;
                }

                if let Ok(rev_out) = Command::new("git")
                    .args(["rev-list", "--left-right", "--count", "HEAD...@{u}"])
                    .current_dir(cwd)
                    .output()
                {
                    if rev_out.status.success() {
                        let output_str = String::from_utf8_lossy(&rev_out.stdout).trim().to_string();
                        let parts: Vec<&str> = output_str.split_whitespace().collect();
                        if parts.len() == 2 {
                            if let Ok(a) = parts[0].parse::<u32>() {
                                git_ahead = a;
                            }
                            if let Ok(b) = parts[1].parse::<u32>() {
                                git_behind = b;
                            }
                        }
                    }
                }
            }
        }
    }

    if let Some(cwd) = cwd_force {
        existing_cache.vcs = Some(VcsInfo {
            cwd,
            branch: git_branch,
            dirty: git_dirty,
            ahead: git_ahead,
            behind: git_behind,
            modified: git_modified,
            lastChecked: now,
        });
    }

    let tmp_path = format!("{}.tmp.{}", status_cache_path.to_string_lossy(), std::process::id());
    if let Ok(serialized) = serde_json::to_string(&existing_cache) {
        if fs::write(&tmp_path, serialized).is_ok() {
            let _ = fs::rename(tmp_path, &status_cache_path);
        }
    }

    agy_statusline_lib::write_shared_cache(&existing_cache);
}

// --- App Mode Entrances ------------------------------------------------------

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

        let json = parse_input_json(&input_data);
        let raw_cwd = json.workspace.as_ref().and_then(|w| w.current_dir.clone()).or_else(|| json.cwd.clone()).unwrap_or_default();
        
        let mut cache = agy_statusline_lib::read_shared_cache().unwrap_or_else(|| {
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

        let mut need_refresh = false;
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
        let status_cache_path = resolve_antigravity_path("statusline-cache.json");

        if !status_cache_path.exists() {
            need_refresh = true;
        } else {
            let cached_cwd = cache.vcs.as_ref().map(|v| v.cwd.as_str()).unwrap_or("");
            let vcs_last_checked = cache.vcs.as_ref().map(|v| v.lastChecked).unwrap_or(0);

            if !raw_cwd.is_empty() && raw_cwd != cached_cwd {
                need_refresh = true;
            } else {
                let quota_age = now.saturating_sub(cache.lastRefreshed);
                let vcs_age = now.saturating_sub(vcs_last_checked);
                if quota_age > 120 || vcs_age > 3 {
                    need_refresh = true;
                }
            }
        }

        if need_refresh {
            let mutex_active = agy_statusline_lib::NamedMutex::is_active("Local\\AgyStatuslineRefreshMutex");

            if !mutex_active {
                if let Ok(current_exe) = std::env::current_exe() {
                    let mut cmd = Command::new(current_exe);
                    cmd.arg("--refresh");
                    if !raw_cwd.is_empty() {
                        cmd.arg("--cwd").arg(&raw_cwd);
                    }
                    cmd.stdin(std::process::Stdio::null());
                    cmd.stdout(std::process::Stdio::null());
                    cmd.stderr(std::process::Stdio::null());

                    #[cfg(windows)]
                    {
                        use std::os::windows::process::CommandExt;
                        cmd.creation_flags(0x08000000);
                    }

                    let _ = cmd.spawn();
                }
            }
        }
    }
}



fn set_config_theme(theme_name: &str) {
    let mut config = load_user_config();
    let theme_lower = theme_name.trim().to_lowercase();
    let valid = matches!(theme_lower.as_str(), "frost" | "pastel" | "neon" | "custom");
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
        println!("Unknown theme name: '{}'. Valid options are: 'frost', 'pastel', 'neon', 'custom'.", theme_name);
    }
}

// --- Main Entrance -----------------------------------------------------------

fn main() {
    let args: Vec<String> = std::env::args().collect();



    if let Some(idx) = args.iter().position(|a| a == "--theme") {
        if idx + 1 < args.len() {
            set_config_theme(&args[idx + 1]);
            return;
        }
    }


    
    let is_title = args.contains(&"--title".to_string());

    if is_title {
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
