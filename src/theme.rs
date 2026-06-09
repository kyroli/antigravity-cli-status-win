// Terminal ANSI escape sequences, color palettes, and state label definitions.

pub const RESET: &str = "\x1b[0m";
pub const BOLD: &str = "\x1b[1m";

pub struct Palette {
    pub border: &'static str,
    pub label: &'static str,
    pub accent: &'static str,
    pub success: &'static str,
    pub warning: &'static str,
    pub critical: &'static str,
}

pub struct StateLabels {
    pub ready: &'static str,
    pub thinking: &'static str,
    pub working: &'static str,
    pub tool_use: &'static str,
    pub default_state: &'static str,
}

pub struct Theme {
    pub palette: &'static Palette,
    pub states: &'static StateLabels,
}

// --- Frost (default) ---------------------------------------------------------

static FROST_PALETTE: Palette = Palette {
    border:   "\x1b[38;2;76;86;106m",
    label:    "\x1b[38;2;216;222;233m",
    accent:   "\x1b[38;2;136;192;208m",
    success:  "\x1b[38;2;163;190;140m",
    warning:  "\x1b[38;2;235;203;139m",
    critical: "\x1b[38;2;191;97;106m",
};

static FROST_STATES: StateLabels = StateLabels {
    ready:         "\x1b[38;2;163;190;140m\x1b[1m[READY]\x1b[0m",
    thinking:      "\x1b[38;2;235;203;139m\x1b[1m[THINKING]\x1b[0m",
    working:       "\x1b[38;2;136;192;208m\x1b[1m[WORKING]\x1b[0m",
    tool_use:      "\x1b[38;2;136;192;208m\x1b[1m[TOOL]\x1b[0m",
    default_state: "\x1b[38;2;216;222;233m\x1b[1m[STATE]\x1b[0m",
};

// --- Pastel ------------------------------------------------------------------

static PASTEL_PALETTE: Palette = Palette {
    border:   "\x1b[38;2;88;91;112m",
    label:    "\x1b[38;2;205;214;244m",
    accent:   "\x1b[38;2;203;166;247m",
    success:  "\x1b[38;2;166;227;161m",
    warning:  "\x1b[38;2;249;226;175m",
    critical: "\x1b[38;2;243;139;168m",
};

static PASTEL_STATES: StateLabels = StateLabels {
    ready:         "\x1b[38;2;166;227;161m\x1b[1m[READY]\x1b[0m",
    thinking:      "\x1b[38;2;249;226;175m\x1b[1m[THINKING]\x1b[0m",
    working:       "\x1b[38;2;203;166;247m\x1b[1m[WORKING]\x1b[0m",
    tool_use:      "\x1b[38;2;203;166;247m\x1b[1m[TOOL]\x1b[0m",
    default_state: "\x1b[38;2;205;214;244m\x1b[1m[STATE]\x1b[0m",
};

// --- Neon --------------------------------------------------------------------

static NEON_PALETTE: Palette = Palette {
    border:   "\x1b[38;2;59;66;97m",
    label:    "\x1b[38;2;169;177;214m",
    accent:   "\x1b[38;2;125;207;255m",
    success:  "\x1b[38;2;115;218;202m",
    warning:  "\x1b[38;2;255;158;100m",
    critical: "\x1b[38;2;247;118;142m",
};

static NEON_STATES: StateLabels = StateLabels {
    ready:         "\x1b[38;2;115;218;202m\x1b[1m[READY]\x1b[0m",
    thinking:      "\x1b[38;2;255;158;100m\x1b[1m[THINKING]\x1b[0m",
    working:       "\x1b[38;2;125;207;255m\x1b[1m[WORKING]\x1b[0m",
    tool_use:      "\x1b[38;2;125;207;255m\x1b[1m[TOOL]\x1b[0m",
    default_state: "\x1b[38;2;169;177;214m\x1b[1m[STATE]\x1b[0m",
};

// --- Fallback (ANSI 16-color) ------------------------------------------------

static FALLBACK_PALETTE: Palette = Palette {
    border:   "\x1b[90m",
    label:    "\x1b[37m",
    accent:   "\x1b[94m",
    success:  "\x1b[96m",
    warning:  "\x1b[93m",
    critical: "\x1b[91m",
};

static FALLBACK_STATES: StateLabels = StateLabels {
    ready:         "\x1b[92m\x1b[1m[READY]\x1b[0m",
    thinking:      "\x1b[93m\x1b[1m[THINKING]\x1b[0m",
    working:       "\x1b[96m\x1b[1m[WORKING]\x1b[0m",
    tool_use:      "\x1b[95m\x1b[1m[TOOL]\x1b[0m",
    default_state: "\x1b[97m\x1b[1m[STATE]\x1b[0m",
};

// --- Resolution --------------------------------------------------------------

pub fn resolve(name: &str) -> Theme {
    match name.to_lowercase().as_str() {
        "frost" => Theme { palette: &FROST_PALETTE, states: &FROST_STATES },
        "pastel" => Theme { palette: &PASTEL_PALETTE, states: &PASTEL_STATES },
        "neon" => Theme { palette: &NEON_PALETTE, states: &NEON_STATES },
        _ => Theme { palette: &FALLBACK_PALETTE, states: &FALLBACK_STATES },
    }
}

pub fn get_icon(widget: &str) -> &'static str {
    match widget {
        "vcs" => "\u{e0a0}",
        "path" => "\u{f07c}",
        "quota" => "\u{f0e7}",
        "context" => "\u{f061a}",
        "cache" => "\u{f0a0}",
        "artifacts" => "\u{f09d1}",
        "subagents" => "\u{f06a9}",
        "tasks" => "\u{f051b}",
        "sandbox" => "\u{f132}",
        "settings" => "\u{f013}",
        _ => "",
    }
}
