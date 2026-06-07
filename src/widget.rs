// Widget abstraction for terminal UI segments: visual length calculation,
// progress bar rendering, and typed widget construction.

use crate::theme;

#[derive(Clone)]
pub struct Widget {
    pub text: String,
    pub visual_width: usize,
}

impl Widget {
    pub fn new(text: String) -> Self {
        let visual_width = get_visual_length(&text);
        Widget { text, visual_width }
    }
}

pub fn get_visual_length(s: &str) -> usize {
    let mut len = 0;
    let mut in_ansi = false;
    let mut chars = s.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\x1b' {
            if chars.peek() == Some(&'[') {
                in_ansi = true;
                chars.next();
                continue;
            }
        }
        if in_ansi {
            if ch.is_ascii_alphabetic() {
                in_ansi = false;
            }
            continue;
        }
        len += 1;
    }
    len
}

pub fn render_progress_bar(pct: f64, bar_len: usize, color: &str, is_quota: bool) -> String {
    if bar_len == 0 {
        return String::new();
    }
    let dim = "\x1b[38;2;80;80;80m";
    let filled = std::cmp::min(bar_len, ((pct / 100.0) * (bar_len as f64)).round() as usize);

    let mut bar = String::with_capacity(bar_len * 4 + 40);
    bar.push_str(dim);
    bar.push('[');

    if is_quota {
        bar.push_str(color);
        for _ in 0..filled {
            bar.push('-');
        }
        bar.push_str(dim);
        for _ in filled..bar_len {
            bar.push('-');
        }
    } else {
        bar.push_str(color);
        for i in 0..bar_len {
            if i < filled {
                bar.push(if i == filled - 1 { '>' } else { '=' });
            } else {
                if i == filled {
                    bar.push_str(dim);
                }
                bar.push('-');
            }
        }
    }

    bar.push_str(dim);
    bar.push(']');
    bar.push_str(theme::RESET);
    bar
}
