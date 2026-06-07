// InputJson merge logic for preserving context window data across status updates.

use crate::types::{InputJson, ContextWindow, CurrentUsage};

fn is_same_session(current: &InputJson, last: &InputJson) -> bool {
    let cur_cid = current.conversation_id.as_deref().unwrap_or("").trim();
    let last_cid = last.conversation_id.as_deref().unwrap_or("").trim();
    if cur_cid != last_cid {
        return false;
    }
    let cur_mid = current.model.as_ref().and_then(|m| m.id.as_deref()).unwrap_or("").trim();
    let last_mid = last.model.as_ref().and_then(|m| m.id.as_deref()).unwrap_or("").trim();
    if !cur_mid.is_empty() && !last_mid.is_empty() && cur_mid != last_mid {
        return false;
    }
    true
}

fn merge_field<T: PartialEq + Copy>(cur: &mut Option<T>, last_val: Option<T>, empty: T) {
    if cur.unwrap_or(empty) == empty {
        *cur = last_val;
    }
}

pub fn merge_input_json(mut current: InputJson, last: Option<InputJson>) -> InputJson {
    current.plan_tier = None;

    let last = match last {
        Some(l) => l,
        None => return current,
    };

    if !is_same_session(&current, &last) {
        return current;
    }

    let is_idle = current.agent_state.as_deref().unwrap_or("idle") == "idle";

    if current.context_window.is_none() {
        if !is_idle {
            current.context_window = last.context_window.clone();
        }
    } else if let Some(ref mut cur_cw) = current.context_window {
        if let Some(ref last_cw) = last.context_window {
            merge_field(&mut cur_cw.context_window_size, last_cw.context_window_size, 0);
            merge_field(&mut cur_cw.total_input_tokens, last_cw.total_input_tokens, 0);
            merge_field(&mut cur_cw.total_output_tokens, last_cw.total_output_tokens, 0);
            merge_field(&mut cur_cw.used_percentage, last_cw.used_percentage, 0.0);

            match (&mut cur_cw.current_usage, &last_cw.current_usage) {
                (cur_opt @ None, Some(last_cu)) => {
                    *cur_opt = Some(last_cu.clone());
                }
                (Some(cur_cu), Some(last_cu)) => {
                    if !is_idle {
                        merge_field(&mut cur_cu.cache_read_input_tokens, last_cu.cache_read_input_tokens, 0);
                        merge_field(&mut cur_cu.cache_creation_input_tokens, last_cu.cache_creation_input_tokens, 0);
                        merge_field(&mut cur_cu.input_tokens, last_cu.input_tokens, 0);
                        merge_field(&mut cur_cu.output_tokens, last_cu.output_tokens, 0);
                    }
                }
                _ => {}
            }
        }
    }

    current
}
