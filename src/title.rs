// Terminal window title string generation from agent state and workspace info.

use crate::types::InputJson;

pub fn get_title_string(json: &InputJson) -> String {
    let state = json.agent_state.as_deref().unwrap_or("idle");
    let raw_cwd = json
        .workspace
        .as_ref()
        .and_then(|w| w.current_dir.clone())
        .or_else(|| json.cwd.clone())
        .unwrap_or_default();

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
        "initializing" => "\u{1f680}",
        "idle" => "\u{1f634}",
        "thinking" => "\u{1f914}",
        "working" => "\u{1f3c3}",
        "tool_use" => "\u{1f6e0}\u{fe0f}",
        _ => "\u{1f916}",
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
