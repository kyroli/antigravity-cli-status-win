// Filesystem path utilities, human-readable formatting, date parsing,
// percent-encoding, and fast git branch detection.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

pub fn get_antigravity_dir() -> PathBuf {
    let home = std::env::var("USERPROFILE")
        .unwrap_or_else(|_| std::env::var("HOME").unwrap_or_default());
    PathBuf::from(&home).join(".gemini").join("antigravity-cli")
}

pub fn resolve_antigravity_path(filename: &str) -> PathBuf {
    let dir = get_antigravity_dir().join("statusline");
    if !dir.exists() {
        let _ = std::fs::create_dir_all(&dir);
    }
    dir.join(filename)
}

pub fn get_human_format(num: u64) -> String {
    if num >= 1_000_000 {
        let main = num / 1_000_000;
        let dec = (num % 1_000_000) / 100_000;
        format!("{}.{}M", main, dec)
    } else if num >= 1_000 {
        let main = num / 1_000;
        let dec = (num % 1_000) / 100;
        format!("{}.{}K", main, dec)
    } else {
        num.to_string()
    }
}

fn days_since_epoch(y: u64, m: u64, d: u64) -> u64 {
    let y = y - if m <= 2 { 1 } else { 0 };
    let era = y / 400;
    let y_of_era = y % 400;
    let doy = ((153 * (m as i64 + if m <= 2 { 9 } else { -3 }) + 2) / 5 + d as i64 - 1) as u64;
    let doe = y_of_era * 365 + y_of_era / 4 - y_of_era / 100 + doy;
    era * 146097 + doe - 719468
}

pub fn parse_rfc3339_to_unix(s: &str) -> Option<u64> {
    if s.len() < 19 {
        return None;
    }
    let year: u64 = s[0..4].parse().ok()?;
    let month: u64 = s[5..7].parse().ok()?;
    let day: u64 = s[8..10].parse().ok()?;
    let hour: u64 = s[11..13].parse().ok()?;
    let min: u64 = s[14..16].parse().ok()?;
    let sec: u64 = s[17..19].parse().ok()?;

    let days = days_since_epoch(year, month, day);
    let mut total_secs = days * 86400 + hour * 3600 + min * 60 + sec;

    let mut tz_part = &s[19..];
    if tz_part.starts_with('.') {
        if let Some(non_digit_idx) = tz_part.find(|c: char| !c.is_ascii_digit() && c != '.') {
            tz_part = &tz_part[non_digit_idx..];
        } else {
            tz_part = "";
        }
    }

    if tz_part.starts_with('+') || tz_part.starts_with('-') {
        let sign = if tz_part.starts_with('+') { 1i64 } else { -1i64 };
        let tz_digits = &tz_part[1..];
        if tz_digits.len() >= 5 {
            let tz_hour: i64 = tz_digits[0..2].parse().unwrap_or(0);
            let tz_min: i64 = tz_digits[3..5].parse().unwrap_or(0);
            let offset_secs = (tz_hour * 3600 + tz_min * 60) * sign;
            total_secs = (total_secs as i64 - offset_secs) as u64;
        }
    }

    Some(total_secs)
}

pub fn get_shorten_path(path_val: &str) -> String {
    let path_norm = path_val.replace('\\', "/");
    let home = std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .unwrap_or_default()
        .replace('\\', "/");

    let mut display_path = path_norm.clone();
    if !home.is_empty() && path_norm.starts_with(&home) {
        display_path = format!("~{}", &path_norm[home.len()..]);
    }

    if display_path.len() > 25 {
        let parts: Vec<&str> = display_path.split('/').collect();
        if !parts.is_empty() {
            return format!(".../{}", parts[parts.len() - 1]);
        }
    }
    display_path
}

pub fn get_file_mtime(path: &Path) -> Option<u64> {
    if !path.exists() {
        return None;
    }
    let metadata = fs::metadata(path).ok()?;
    let modified = metadata.modified().ok()?;
    let duration = modified.duration_since(UNIX_EPOCH).ok()?;
    Some(duration.as_secs())
}

pub fn get_configs_last_modified_time() -> u64 {
    let mut max_mtime = 0;
    let dir = get_antigravity_dir();
    if let Some(mtime) = get_file_mtime(&dir.join("antigravity-oauth-token")) {
        max_mtime = max_mtime.max(mtime);
    }
    if let Some(parent) = dir.parent() {
        if let Some(mtime) = get_file_mtime(&parent.join("oauth_creds.json")) {
            max_mtime = max_mtime.max(mtime);
        }
    }
    max_mtime
}


// --- Fast git branch detection -----------------------------------------------

pub fn find_git_dir(cwd: &str) -> Option<PathBuf> {
    let mut current = Path::new(cwd);
    loop {
        let git_dir = current.join(".git");
        if git_dir.exists() {
            if git_dir.is_dir() {
                return Some(git_dir);
            } else if git_dir.is_file() {
                if let Ok(content) = fs::read_to_string(&git_dir) {
                    if let Some(gitdir_line) = content.lines().next() {
                        if let Some(gitdir_path) = gitdir_line.trim().strip_prefix("gitdir: ") {
                            let path = Path::new(gitdir_path.trim());
                            if path.is_absolute() {
                                return Some(path.to_path_buf());
                            } else {
                                return Some(current.join(path));
                            }
                        }
                    }
                }
            }
        }
        current = current.parent()?;
    }
}

pub fn get_git_branch_fast(cwd: &str) -> Option<String> {
    let git_dir = find_git_dir(cwd)?;
    let head_path = git_dir.join("HEAD");
    if let Ok(content) = fs::read_to_string(head_path) {
        let line = content.lines().next()?.trim();
        if let Some(ref_path) = line.strip_prefix("ref: refs/heads/") {
            return Some(ref_path.to_string());
        } else {
            return Some(line.chars().take(8).collect::<String>());
        }
    }
    None
}

// --- Percent-encoding for file:// URIs ---------------------------------------

pub fn percent_encode_path(path: &str) -> String {
    let mut encoded = String::with_capacity(path.len() * 3);
    for byte in path.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' | b'/' | b':' => {
                encoded.push(byte as char);
            }
            _ => {
                encoded.push_str(&format!("%{:02X}", byte));
            }
        }
    }
    encoded
}
