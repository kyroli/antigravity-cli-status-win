// Background refresh: quota API fetch, git status query, and cache persistence.

use std::fs;
use std::path::Path;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::types::{CacheData, QuotaItem, VcsInfo};
use crate::crypto::sha256_hex;
use crate::path::{resolve_antigravity_path, get_configs_last_modified_time, get_git_branch_fast};
use crate::platform::{get_access_token, NamedMutex, write_shared_cache, CREATE_NO_WINDOW};

/// Minimum seconds between consecutive quota API fetches.
pub const QUOTA_REFRESH_INTERVAL_SECS: u64 = 120;

pub fn run_background_refresh(cwd_force: Option<String>) {
    #[cfg(windows)]
    let _mutex = match NamedMutex::acquire("Local\\AgyStatuslineRefreshMutex") {
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

    let quota_age = now.saturating_sub(existing_cache.last_refreshed);
    let need_quota_fetch = (existing_cache.quota.is_empty()
        || quota_age > QUOTA_REFRESH_INTERVAL_SECS
        || last_config_update > cache_modified_secs
        || token_changed)
        && token_opt.is_some();

    if token_opt.is_none() {
        existing_cache.token_hash = None;
        existing_cache.needs_login = Some(true);
        existing_cache.last_refreshed = now;
        existing_cache.quota.clear();
    } else if need_quota_fetch {
        if let Some(ref token) = token_opt {
            fetch_quota(token, &current_token_hash, now, &mut existing_cache);
        }
    }

    // Git status
    if let Some(ref cwd) = cwd_force {
        collect_git_status(cwd, now, &mut existing_cache);
    }

    // Persist to file
    let tmp_path = format!("{}.tmp.{}", status_cache_path.to_string_lossy(), std::process::id());
    if let Ok(serialized) = serde_json::to_string(&existing_cache) {
        if fs::write(&tmp_path, serialized).is_ok() {
            let _ = fs::rename(tmp_path, &status_cache_path);
        }
    }

    write_shared_cache(&existing_cache);
}

fn fetch_quota(token: &str, current_token_hash: &Option<String>, now: u64, cache: &mut CacheData) {
    use ureq::tls::{TlsConfig, RootCerts};

    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(std::time::Duration::from_secs(6)))
        .http_status_as_error(false)
        .user_agent(&format!("antigravity-statusline-win11-rs/{} windows/11", env!("CARGO_PKG_VERSION")))
        .tls_config(
            TlsConfig::builder()
                .root_certs(RootCerts::PlatformVerifier)
                .build(),
        )
        .build()
        .into();

    let url = "https://cloudcode-pa.googleapis.com/v1internal:fetchAvailableModels";
    let res = agent
        .post(url)
        .header("Authorization", &format!("Bearer {}", token))
        .send_json(&serde_json::json!({}));

    match res {
        Ok(mut resp) => {
            let status = resp.status();
            if status == 200 {
                if let Ok(json_body) = resp.body_mut().read_json::<serde_json::Value>() {
                    if let Some(models) = json_body.get("models").and_then(|m| m.as_object()) {
                        let mut quota_list: Vec<QuotaItem> = Vec::new();
                        for (key, model_val) in models {
                            if let Some(quota_info) = model_val.get("quotaInfo") {
                                let remaining_fraction = quota_info
                                    .get("remainingFraction")
                                    .and_then(|v| v.as_f64())
                                    .unwrap_or(0.0);
                                let reset_time = quota_info
                                    .get("resetTime")
                                    .and_then(|v| v.as_str())
                                    .map(|s| s.to_string());
                                let display_name = model_val
                                    .get("displayName")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or(key)
                                    .to_string();

                                quota_list.push(QuotaItem {
                                    id: key.clone(),
                                    display_name,
                                    remaining_fraction,
                                    reset_time,
                                });
                            }
                        }
                        cache.needs_login = Some(false);
                        cache.token_hash = current_token_hash.clone();
                        cache.quota = quota_list;
                        cache.last_refreshed = now;
                        return;
                    }
                }
            } else if status == 401 || status == 403 {
                cache.needs_login = Some(true);
                cache.token_hash = current_token_hash.clone();
                cache.quota.clear();
                cache.last_refreshed = now;
                return;
            }
        }
        _ => {}
    }

    cache.last_refreshed = now;
}

fn collect_git_status(cwd: &str, now: u64, cache: &mut CacheData) {
    if !Path::new(cwd).exists() {
        return;
    }

    // Find the Git directory and check HEAD & index modified times
    let git_dir = crate::path::find_git_dir(cwd);
    let head_mtime = git_dir.as_ref().and_then(|gd| crate::path::get_file_mtime(&gd.join("HEAD")));
    let index_mtime = git_dir.as_ref().and_then(|gd| crate::path::get_file_mtime(&gd.join("index")));

    // If mtime hasn't changed, reuse the cached Git state to avoid launching expensive subprocesses
    if let Some(ref existing_vcs) = cache.vcs {
        if existing_vcs.cwd == cwd
            && existing_vcs.head_mtime == head_mtime
            && existing_vcs.index_mtime == index_mtime
            && head_mtime.is_some()
            && index_mtime.is_some()
        {
            if let Some(ref mut vcs) = cache.vcs {
                vcs.last_checked = now;
            }
            return;
        }
    }

    let mut git_branch = String::new();
    let mut git_dirty = false;
    let mut git_ahead = 0u32;
    let mut git_behind = 0u32;
    let mut git_modified = 0u32;
    let mut remote_web_url = None;
    let mut insertions = 0u32;
    let mut deletions = 0u32;

    if let Some(branch) = get_git_branch_fast(cwd) {
        git_branch = branch;

        // 1. Git Status
        if let Some(status_str) = run_git_cmd(&["--no-optional-locks", "status", "--porcelain"], cwd) {
            let count = status_str.lines().filter(|l| !l.trim().is_empty()).count() as u32;
            git_dirty = count > 0;
            git_modified = count;
        }

        // 2. Ahead/Behind
        if let Some(rev_str) = run_git_cmd(&["--no-optional-locks", "rev-list", "--left-right", "--count", "HEAD...@{u}"], cwd) {
            let parts: Vec<&str> = rev_str.split_whitespace().collect();
            if parts.len() == 2 {
                if let Ok(a) = parts[0].parse::<u32>() {
                    git_ahead = a;
                }
                if let Ok(b) = parts[1].parse::<u32>() {
                    git_behind = b;
                }
            }
        }

        // 3. Remote URL
        if let Some(remote_url_raw) = run_git_cmd(&["--no-optional-locks", "config", "--get", "remote.origin.url"], cwd) {
            remote_web_url = parse_git_remote_url(&remote_url_raw);
        }

        // 4. Insertions & Deletions (Unstaged)
        if let Some(diff_str) = run_git_cmd(&["--no-optional-locks", "diff", "--numstat"], cwd) {
            let (ins, del) = parse_numstat(&diff_str);
            insertions += ins;
            deletions += del;
        }

        // 5. Insertions & Deletions (Staged)
        if let Some(cached_diff_str) = run_git_cmd(&["--no-optional-locks", "diff", "--cached", "--numstat"], cwd) {
            let (ins, del) = parse_numstat(&cached_diff_str);
            insertions += ins;
            deletions += del;
        }
    }

    cache.vcs = Some(VcsInfo {
        cwd: cwd.to_string(),
        branch: git_branch,
        dirty: git_dirty,
        ahead: git_ahead,
        behind: git_behind,
        modified: git_modified,
        last_checked: now,
        head_mtime,
        index_mtime,
        remote_web_url,
        insertions,
        deletions,
    });
}

fn run_git_cmd(args: &[&str], cwd: &str) -> Option<String> {
    let mut cmd = Command::new("git");
    cmd.args(args).current_dir(cwd);

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }

    let output = cmd.output().ok()?;
    if output.status.success() {
        Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        None
    }
}

fn parse_git_remote_url(url: &str) -> Option<String> {
    let url = url.trim();
    if url.is_empty() {
        return None;
    }

    if url.starts_with("https://") || url.starts_with("http://") {
        let mut clean = url;
        if clean.ends_with(".git") {
            clean = &clean[..clean.len() - 4];
        }
        return Some(clean.to_string());
    }

    if url.contains('@') && url.contains(':') {
        if let Some(colon_idx) = url.find(':') {
            let host_part = &url[..colon_idx];
            let path_part = &url[colon_idx + 1..];
            let host = host_part.split('@').last().unwrap_or(host_part);
            let mut path = path_part;
            if path.ends_with(".git") {
                path = &path[..path.len() - 4];
            }
            return Some(format!("https://{}/{}", host, path.replace('\\', "/")));
        }
    }

    None
}

fn parse_numstat(output: &str) -> (u32, u32) {
    let mut insertions = 0;
    let mut deletions = 0;
    for line in output.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 2 {
            if let Ok(ins) = parts[0].parse::<u32>() {
                insertions += ins;
            }
            if let Ok(del) = parts[1].parse::<u32>() {
                deletions += del;
            }
        }
    }
    (insertions, deletions)
}
