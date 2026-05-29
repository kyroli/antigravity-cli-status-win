const fs = require('fs');
const path = require('path');

// ─── Constants & Paths Definition ───────────────────────────────────────────
function getAntigravityRoots() {
    const homeDir = process.env.USERPROFILE || process.env.HOME || "";
    return [
        path.join(homeDir, '.gemini', 'antigravity-cli'),
        process.env.XDG_DATA_HOME ? path.join(process.env.XDG_DATA_HOME, 'antigravity-cli') : null,
        process.env.APPDATA ? path.join(process.env.APPDATA, 'antigravity-cli') : null,
        process.env.LOCALAPPDATA ? path.join(process.env.LOCALAPPDATA, 'antigravity-cli') : null,
    ].filter(Boolean);
}

function resolveAntigravityPath(filename) {
    const root = getAntigravityRoots()[0];
    return path.join(root, filename);
}

const TOKEN_CACHE_PATH = resolveAntigravityPath('statusline-token.json');
const STATUS_CACHE_PATH = resolveAntigravityPath('statusline-cache.json');
const LOCK_FILE_PATH = resolveAntigravityPath('statusline.lock');

// ─── CLI Entry Handler ────────────────────────────────────────────────────────
const args = process.argv;
if (args.includes('--refresh')) {
    const cwdIndex = args.indexOf('--cwd');
    const rawCwdForce = (cwdIndex !== -1 && cwdIndex + 1 < args.length) ? args[cwdIndex + 1] : null;
    const cwdForce = rawCwdForce ? path.resolve(rawCwdForce) : null;
    runBackgroundRefresh(cwdForce).then(() => {
        process.exit(0);
    }).catch(() => {
        process.exit(1);
    });
    return;
}

// ─── Global Status Cache Manager ─────────────────────────────────────────────
let _statusCache = null;
function loadStatusCache() {
    if (_statusCache) return _statusCache;
    try {
        if (fs.existsSync(STATUS_CACHE_PATH)) {
            _statusCache = JSON.parse(fs.readFileSync(STATUS_CACHE_PATH, 'utf8'));
        }
    } catch (e) {}
    if (!_statusCache) {
        _statusCache = { quota: [], vcs: {} };
    }
    return _statusCache;
}

// ─── Config Definitions ──────────────────────────────────────────────────────
const CONFIG = {
    showState: true,
    showModel: true,
    showPath: true,
    showVcs: true,
    showQuota: true,
    showPendingInput: true,
    showApprovalAlert: true,
    showContextBar: true,
    showCacheStats: true,
    showArtifacts: true,
    showSubagents: true,
    showTasks: true,
    showSandbox: true,
    showConversationId: false,
    showVersion: false,
    showPlanTier: false,
    showEmail: false,
    barLength: 15,
};

// ─── ANSI Helpers ────────────────────────────────────────────────────────────
const ESC = '\x1b[';
const R = `${ESC}0m`;
const B = `${ESC}1m`;
const D = `${ESC}2m`;
const I = `${ESC}3m`;

const FG_BRIGHT_GREEN   = `${ESC}92m`;
const FG_BRIGHT_YELLOW  = `${ESC}93m`;
const FG_BRIGHT_CYAN    = `${ESC}96m`;
const FG_BRIGHT_MAGENTA = `${ESC}95m`;
const FG_BRIGHT_RED     = `${ESC}91m`;
const FG_BRIGHT_WHITE   = `${ESC}97m`;
const FG_GRAY           = `${ESC}90m`;
const FG_BRIGHT_BLUE    = `${ESC}94m`;

const NUM_COLOR = `${FG_BRIGHT_WHITE}${B}`;

function getHumanFormat(num) {
    if (num === undefined || num === null || num === 0) return "0";
    if (num >= 1000000) {
        const main = Math.floor(num / 1000000);
        const dec = Math.floor((num % 1000000) / 100000);
        return `${main}.${dec}M`;
    }
    if (num >= 1000) {
        const main = Math.floor(num / 1000);
        const dec = Math.floor((num % 1000) / 100);
        return `${main}.${dec}K`;
    }
    return String(num);
}

function getShortenPath(pathVal) {
    if (!pathVal) return "";
    let pathNorm = pathVal.replace(/\\/g, "/");
    const home = process.env.USERPROFILE || process.env.HOME;
    if (home) {
        const homeNorm = home.replace(/\\/g, "/");
        if (pathNorm.startsWith(homeNorm)) {
            pathNorm = "~" + pathNorm.slice(homeNorm.length);
        }
    }
    if (pathNorm.length > 25) {
        const parts = pathNorm.split("/");
        return ".../" + parts[parts.length - 1];
    }
    return pathNorm;
}

function getVisualLength(str) {
    return str.replace(/\x1B\[[0-9;]*m/g, '').length;
}

function w(text, len) {
    return { text, len: len !== undefined ? len : getVisualLength(text) };
}

// ─── Git Fast Resolver Module (Moved to Background SWR) ──────────────────────

// ─── Background Refresh Module (SWR) ──────────────────────────────────────────
function buildWindowsCredentialScript() {
    const targets = [
        'gemini:antigravity',
        'LegacyGeneric:target=gemini:antigravity',
    ].map(target => `'${target.replace(/'/g, "''")}'`).join(',');

    return [
        '$ErrorActionPreference = "SilentlyContinue"',
        'Add-Type -Language CSharp -TypeDefinition @"',
        'using System; using System.Runtime.InteropServices; using System.Text;',
        'public class WC {',
        '  [StructLayout(LayoutKind.Sequential,CharSet=CharSet.Unicode)]',
        '  public struct CRED { public uint Flags,Type; public string Target,Comment;',
        '    public long LastWritten; public uint BlobSize; public IntPtr Blob;',
        '    public uint Persist,AttrCount; public IntPtr Attrs; public string Alias,User; }',
        '  [DllImport("advapi32.dll",CharSet=CharSet.Unicode,SetLastError=true)]',
        '  public static extern bool CredRead(string target,uint type,int reservedFlag,out IntPtr credentialPtr);',
        '  [DllImport("advapi32.dll")] public static extern void CredFree(IntPtr p);',
        '}',
        '"@',
        `$targets=@(${targets})`,
        '$tokens=@()',
        'foreach($target in $targets){',
        '  $p=[IntPtr]::Zero',
        '  if([WC]::CredRead($target,1,0,[ref]$p)){',
        '    try {',
        '      $c=[Runtime.InteropServices.Marshal]::PtrToStructure($p,[type][WC+CRED])',
        '      if($c.BlobSize -gt 0){',
        '        $bytes=New-Object byte[] $c.BlobSize',
        '        [Runtime.InteropServices.Marshal]::Copy($c.Blob,$bytes,0,$c.BlobSize)',
        '        foreach($enc in @([Text.Encoding]::UTF8,[Text.Encoding]::Unicode)){',
        '          try {',
        '            $o=$enc.GetString($bytes)|ConvertFrom-Json',
        '            $tok=$o.token',
        '            if($tok -and $tok.access_token){',
        '              $tokens += [pscustomobject]@{ accessToken=$tok.access_token; expiry=$tok.expiry }',
        '              break',
        '            } elseif($o.access_token){',
        '              $tokens += [pscustomobject]@{ accessToken=$o.access_token; expiry=$o.expiry }',
        '              break',
        '            }',
        '          } catch {}',
        '        }',
        '      }',
        '    } finally { [WC]::CredFree($p) }',
        '  }',
        '}',
        '$dedup=@{}',
        'foreach($t in $tokens){ if($t.accessToken -and -not $dedup.ContainsKey($t.accessToken)){ $dedup[$t.accessToken]=$t } }',
        'if($dedup.Count -gt 0){ [pscustomobject]@{ tokens=@($dedup.Values) } | ConvertTo-Json -Compress -Depth 4 }',
        'else { Write-Output "{`"tokens`":[]}" }',
    ].join('\n');
}

function getConfigsLastModifiedTime() {
    let maxMtime = 0;
    try {
        const roots = getAntigravityRoots();
        for (const root of roots) {
            try {
                const tokenPath = path.join(root, 'antigravity-oauth-token');
                if (fs.existsSync(tokenPath)) {
                    const stat = fs.statSync(tokenPath);
                    if (stat.mtimeMs > maxMtime) maxMtime = stat.mtimeMs;
                }
            } catch (e) {}

            try {
                const parentDir = path.dirname(root);
                const credsPath = path.join(parentDir, 'oauth_creds.json');
                if (fs.existsSync(credsPath)) {
                    const stat = fs.statSync(credsPath);
                    if (stat.mtimeMs > maxMtime) maxMtime = stat.mtimeMs;
                }
            } catch (e) {}
        }
    } catch (e) {}
    return maxMtime;
}

function getAccessToken() {
    try {
        if (fs.existsSync(TOKEN_CACHE_PATH)) {
            const stat = fs.statSync(TOKEN_CACHE_PATH);
            const now = Date.now();
            const lastConfigUpdate = getConfigsLastModifiedTime();
            if (lastConfigUpdate < stat.mtimeMs) {
                const raw = JSON.parse(fs.readFileSync(TOKEN_CACHE_PATH, 'utf8'));
                if (raw.accessToken) {
                    if (raw.expiry) {
                        const expiresAt = new Date(raw.expiry).getTime();
                        if (Number.isFinite(expiresAt) && expiresAt - 300000 > now) {
                            return raw.accessToken;
                        }
                    } else {
                        if (now - stat.mtimeMs < 10 * 60 * 1000) {
                            return raw.accessToken;
                        }
                    }
                }
            }
        }
    } catch (e) {}

    let tokenObj = null;

    if (process.platform === 'win32') {
        const script = buildWindowsCredentialScript();
        const { execFileSync } = require('child_process');
        
        const shells = ['pwsh.exe', 'powershell.exe'];
        for (const shell of shells) {
            try {
                const raw = execFileSync(shell, [
                    '-NoProfile',
                    '-NonInteractive',
                    '-ExecutionPolicy',
                    'Bypass',
                    '-Command',
                    script,
                ], {
                    encoding: 'utf8',
                    timeout: 5000,
                    windowsHide: true,
                });
                const parsed = JSON.parse(raw.trim() || '{"tokens":[]}');
                if (parsed.tokens && parsed.tokens.length > 0) {
                    tokenObj = parsed.tokens[0];
                    break;
                }
            } catch (err) {}
        }
    }

    if (!tokenObj) {
        const roots = getAntigravityRoots();
        for (const root of roots) {
            try {
                const filePath = path.join(root, 'antigravity-oauth-token');
                if (fs.existsSync(filePath)) {
                    const raw = JSON.parse(fs.readFileSync(filePath, 'utf8'));
                    if (raw.token && raw.token.access_token) {
                        tokenObj = {
                            accessToken: raw.token.access_token,
                            expiry: raw.token.expiry || null
                        };
                        break;
                    }
                }
            } catch (e) {}

            try {
                const parentDir = path.dirname(root);
                const filePath = path.join(parentDir, 'oauth_creds.json');
                if (fs.existsSync(filePath)) {
                    const raw = JSON.parse(fs.readFileSync(filePath, 'utf8'));
                    if (raw.access_token) {
                        tokenObj = {
                            accessToken: raw.access_token,
                            expiry: raw.expiry || null
                        };
                        break;
                    }
                }
            } catch (e) {}
        }
    }

    if (tokenObj && tokenObj.accessToken) {
        try {
            fs.mkdirSync(path.dirname(TOKEN_CACHE_PATH), { recursive: true });
            fs.writeFileSync(TOKEN_CACHE_PATH, JSON.stringify({
                accessToken: tokenObj.accessToken,
                expiry: tokenObj.expiry
            }), { mode: 0o600 });
        } catch (e) {}
        return tokenObj.accessToken;
    }

    return null;
}

async function runBackgroundRefresh(cwdForce) {
    // Acquire Lock with self-healing mechanism
    try {
        let shouldBreakLock = false;
        if (fs.existsSync(LOCK_FILE_PATH)) {
            const stat = fs.statSync(LOCK_FILE_PATH);
            const lockAge = Date.now() - stat.mtimeMs;
            
            if (lockAge > 30000) {
                shouldBreakLock = true;
            } else {
                try {
                    const lockContent = JSON.parse(fs.readFileSync(LOCK_FILE_PATH, 'utf8'));
                    let isPidRunning = false;
                    if (lockContent.pid) {
                        try {
                            process.kill(lockContent.pid, 0);
                            isPidRunning = true;
                        } catch (err) {
                            if (err.code === 'EPERM') {
                                isPidRunning = true;
                            }
                        }
                    }
                    if (!isPidRunning) {
                        shouldBreakLock = true;
                    } else if (cwdForce && lockContent.cwd !== cwdForce) {
                        shouldBreakLock = true;
                    }
                } catch (err) {
                    shouldBreakLock = true;
                }
            }

            if (shouldBreakLock) {
                try {
                    fs.unlinkSync(LOCK_FILE_PATH);
                } catch (e) {}
            } else {
                return;
            }
        }
        fs.writeFileSync(LOCK_FILE_PATH, JSON.stringify({
            pid: process.pid,
            time: Date.now(),
            cwd: cwdForce || ""
        }), { flag: 'w' });
    } catch (e) {
        return;
    }

    let success = false;
    let quotaData = null;
    let gitDirty = false;

    try {
        // 1. Fetch Quota Info
        const token = getAccessToken();
        if (token) {
            const endpoints = [
                'https://daily-cloudcode-pa.googleapis.com',
                'https://cloudcode-pa.googleapis.com',
            ];

            for (const endpoint of endpoints) {
                const controller = new AbortController();
                const timeoutId = setTimeout(() => controller.abort(), 3000);
                try {
                    const response = await fetch(`${endpoint}/v1internal:fetchAvailableModels`, {
                        method: 'POST',
                        headers: {
                            'Authorization': `Bearer ${token}`,
                            'Content-Type': 'application/json',
                            'User-Agent': 'antigravity/1.0.0 windows/amd64',
                        },
                        body: JSON.stringify({}),
                        signal: controller.signal,
                    });

                    if (response.ok) {
                        const data = await response.json();
                        quotaData = data.models || {};
                        success = true;
                        clearTimeout(timeoutId);
                        break;
                    } else if (response.status === 401 || response.status === 403) {
                        try {
                            if (fs.existsSync(TOKEN_CACHE_PATH)) {
                                fs.unlinkSync(TOKEN_CACHE_PATH);
                            }
                        } catch (e) {}
                        quotaData = {};
                        success = true;
                        clearTimeout(timeoutId);
                        break;
                    }
                } catch (err) {
                } finally {
                    clearTimeout(timeoutId);
                }
            }
        }

        // 2. Fetch Git Info (Branch & Dirty - in background SWR only)
        let gitBranch = "";
        if (cwdForce) {
            try {
                const { execSync } = require('child_process');
                try {
                    gitBranch = execSync('git rev-parse --abbrev-ref HEAD', { cwd: cwdForce, encoding: 'utf8', timeout: 800, windowsHide: true }).trim();
                } catch (err) {}
                try {
                    const statusOut = execSync('git status --porcelain -uno', { cwd: cwdForce, encoding: 'utf8', timeout: 800, windowsHide: true }).trim();
                    gitDirty = statusOut.length > 0;
                } catch (err) {}
            } catch (err) {}
        }

        // 3. Write to Cache
        if (success || cwdForce || token === null) {
            let existingCache = { quota: [], vcs: {} };
            try {
                if (fs.existsSync(STATUS_CACHE_PATH)) {
                    existingCache = JSON.parse(fs.readFileSync(STATUS_CACHE_PATH, 'utf8'));
                }
            } catch (e) {}

            if (success && quotaData) {
                const simplifiedQuota = [];
                for (const key of Object.keys(quotaData)) {
                    const model = quotaData[key];
                    if (model && model.quotaInfo) {
                        simplifiedQuota.push({
                            id: key,
                            displayName: model.displayName || key,
                            remainingFraction: model.quotaInfo.remainingFraction !== undefined ? model.quotaInfo.remainingFraction : 0.0,
                            resetTime: model.quotaInfo.resetTime || null,
                        });
                    }
                }
                existingCache.quota = simplifiedQuota;
                existingCache.lastRefreshed = Date.now();
            } else if (token === null) {
                existingCache.quota = [];
                existingCache.lastRefreshed = Date.now();
                try {
                    if (fs.existsSync(TOKEN_CACHE_PATH)) {
                        fs.unlinkSync(TOKEN_CACHE_PATH);
                    }
                } catch (e) {}
            }

            if (cwdForce) {
                existingCache.vcs = {
                    cwd: cwdForce,
                    branch: gitBranch,
                    dirty: gitDirty,
                    lastChecked: Date.now()
                };
            }

            const tmpPath = `${STATUS_CACHE_PATH}.tmp.${process.pid}`;
            fs.mkdirSync(path.dirname(STATUS_CACHE_PATH), { recursive: true });
            fs.writeFileSync(tmpPath, JSON.stringify(existingCache), { mode: 0o600 });
            fs.renameSync(tmpPath, STATUS_CACHE_PATH);
        }
    } catch (e) {
    } finally {
        // Release Lock and Clean up tmp files
        try {
            if (fs.existsSync(LOCK_FILE_PATH)) {
                const lockContent = JSON.parse(fs.readFileSync(LOCK_FILE_PATH, 'utf8'));
                if (lockContent.pid === process.pid) {
                    fs.unlinkSync(LOCK_FILE_PATH);
                }
            }
        } catch (e) {}
        try {
            const tmpPath = `${STATUS_CACHE_PATH}.tmp.${process.pid}`;
            if (fs.existsSync(tmpPath)) {
                fs.unlinkSync(tmpPath);
            }
        } catch (e) {}
    }
}

// ─── Reading UI state from stdin ─────────────────────────────────────────────
let inputData = '';
process.stdin.on('data', chunk => {
    inputData += chunk;
});

process.stdin.on('end', () => {
    let json = {};
    try {
        let cleanInput = inputData.trim();
        if (cleanInput.charCodeAt(0) === 0xFEFF) {
            cleanInput = cleanInput.slice(1);
        }
        if (cleanInput) {
            json = JSON.parse(cleanInput);
        }
    } catch (e) {}

    const rawCwd = (json.workspace && json.workspace.current_dir) ? json.workspace.current_dir : (json.cwd || "");
    const cwd = rawCwd ? path.resolve(rawCwd) : "";

    // (SWR Background Refresher check is deferred to post-rendering)

    // ─── Extraction and Rendering Logic ──────────────────────────────────────
    const state = json.agent_state || "idle";
    const model = (json.model && json.model.display_name) ? json.model.display_name : "";

    let usedPct = 0, inputTok = 0, outputTok = 0, limitTok = 0;
    let cacheRead = 0, cacheCreate = 0, curIn = 0, curOut = 0;

    if (json.context_window) {
        const cw = json.context_window;
        if (cw.used_percentage !== undefined) usedPct = cw.used_percentage;
        if (cw.total_input_tokens !== undefined) inputTok = cw.total_input_tokens;
        if (cw.total_output_tokens !== undefined) outputTok = cw.total_output_tokens;
        if (cw.context_window_size !== undefined) limitTok = cw.context_window_size;
        
        if (cw.current_usage) {
            const cu = cw.current_usage;
            if (cu.cache_read_input_tokens !== undefined) cacheRead = cu.cache_read_input_tokens;
            if (cu.cache_creation_input_tokens !== undefined) cacheCreate = cu.cache_creation_input_tokens;
            if (cu.input_tokens !== undefined) curIn = cu.input_tokens;
            if (cu.output_tokens !== undefined) curOut = cu.output_tokens;
        }
    }

    const cached = loadStatusCache();
    let vcsBranch = "";
    let vcsDirty = false;
    if (cached.vcs && cached.vcs.cwd === cwd) {
        vcsBranch = cached.vcs.branch || "";
        vcsDirty = !!cached.vcs.dirty;
    }

    let sandbox = false, sandboxAllowNetwork = false;
    if (json.sandbox) {
        if (json.sandbox.enabled !== undefined) sandbox = json.sandbox.enabled;
        if (json.sandbox.allow_network !== undefined) sandboxAllowNetwork = json.sandbox.allow_network;
    }

    let artifacts = 0;
    if (json.artifacts) {
        artifacts = json.artifacts.length;
    } else if (json.artifact_count !== undefined) {
        artifacts = json.artifact_count;
    }

    let subagentsCount = json.subagents ? json.subagents.length : 0;
    let bgTasks = json.background_tasks ? json.background_tasks.length : (json.task_count || 0);

    const toolConfirmationPending = !!json.tool_confirmation_pending;
    const pendingInputCount = json.pending_input_count || 0;
    const planTier = json.plan_tier || "";
    const email = json.email || "";
    const version = json.version || "";
    const convId = json.conversation_id || "";

    const cols = json.terminal_width || process.stderr.columns || 80;
    const pctFmt = usedPct.toFixed(1);
    const pctInt = Math.floor(usedPct);

    // Shorten Model Name
    function getShortModelName(rawName) {
        if (!rawName) return "";
        let clean = rawName.replace(/-preview(-\d+)?/gi, '')
                             .replace(/-experimental/gi, '-exp')
                             .replace(/-latest/gi, '')
                             .replace(/cloudcode-pa-internal/gi, 'cc-pa')
                             .replace(/\(medium\)/gi, '(M)')
                             .replace(/\(high\)/gi, '(H)')
                             .replace(/\(low\)/gi, '(L)')
                             .replace(/\(thinking\)/gi, '(Th)');

        clean = clean.replace(/Gemini\s*(\d+\.\d+)\s*Flash/gi, 'Gem $1F')
                     .replace(/Gemini\s*(\d+\.\d+)\s*Pro/gi, 'Gem $1P')
                     .replace(/Claude\s*(\d+\.\d+)\s*Sonnet/gi, 'Sonnet $1')
                     .replace(/Claude\s*(\d+\.\d+)\s*Haiku/gi, 'Haiku $1')
                     .replace(/Claude\s*(\d+)\s*Opus/gi, 'Opus $1')
                     .replace(/Claude\s*Sonnet\s*(\d+\.\d+)/gi, 'Sonnet $1');

        return clean.length > 15 ? clean.slice(0, 12) + ".." : clean;
    }

    // Read Model Quota from Cache
    function getModelQuotaString(currentModelName, hideTime = false) {
        if (!currentModelName) return "";
        try {
            const payload = loadStatusCache();
            const list = payload.quota || [];

            const cleanName = (name) => name.toLowerCase().replace(/[^a-z0-9]/g, '');
            const targetClean = cleanName(currentModelName);

            let matched = list.find(item => cleanName(item.displayName) === targetClean || cleanName(item.id) === targetClean);
            if (!matched) {
                matched = list.find(item => targetClean.includes(cleanName(item.displayName)) || cleanName(item.displayName).includes(targetClean));
            }

            if (matched) {
                const pct = Math.floor(matched.remainingFraction * 100);
                let timeStr = "";
                if (matched.resetTime && !hideTime) {
                    const diffMs = new Date(matched.resetTime).getTime() - Date.now();
                    if (diffMs > 0) {
                        const diffMins = Math.floor(diffMs / (60 * 1000));
                        const diffHours = Math.floor(diffMins / 60);
                        const diffDays = Math.floor(diffHours / 24);

                        if (diffDays >= 1) {
                            timeStr = ` ~${diffDays}d${diffHours % 24}h`;
                        } else if (diffHours >= 1) {
                            timeStr = ` ~${diffHours}h${diffMins % 60}m`;
                        } else if (diffMins >= 1) {
                            timeStr = ` ~${diffMins}m`;
                        }
                    }
                }

                let color = FG_GRAY;
                if (pct <= 20) color = `${FG_BRIGHT_RED}${B}`;
                else if (pct <= 50) color = FG_BRIGHT_YELLOW;

                return `${color}q:${pct}%${timeStr}${R}`;
            }
        } catch (e) {}
        return "";
    }

    // Adaptive Widget Builders
    function getInfoWidgetsAtStep(step) {
        const widgets = [];

        // 1. State
        if (CONFIG.showState) {
            let text = "";
            let len = 0;
            if (step >= 7) {
                switch (state) {
                    case "idle": text = `${FG_BRIGHT_GREEN}${B}[R]${R}`; len = 3; break;
                    case "thinking": text = `${FG_BRIGHT_YELLOW}${B}[T]${R}`; len = 3; break;
                    case "working": text = `${FG_BRIGHT_CYAN}${B}[W]${R}`; len = 3; break;
                    case "tool_use": text = `${FG_BRIGHT_MAGENTA}${B}[L]${R}`; len = 3; break;
                    default: text = `${FG_BRIGHT_WHITE}${B}[S]${R}`; len = 3;
                }
            } else {
                switch (state) {
                    case "idle": text = `${FG_BRIGHT_GREEN}${B}[READY]${R}`; len = 7; break;
                    case "thinking": text = `${FG_BRIGHT_YELLOW}${B}[THINKING]${R}`; len = 10; break;
                    case "working": text = `${FG_BRIGHT_CYAN}${B}[WORKING]${R}`; len = 9; break;
                    case "tool_use": text = `${FG_BRIGHT_MAGENTA}${B}[TOOL]${R}`; len = 6; break;
                    default: text = `${FG_BRIGHT_WHITE}${B}[${state.toUpperCase()}]${R}`; len = 2 + state.length;
                }
            }
            widgets.push(w(text, len));
        }

        // 2. Approval Pending Alert
        if (CONFIG.showApprovalAlert && toolConfirmationPending) {
            widgets.push(w(`${FG_BRIGHT_RED}${B}[! PENDING APPROVAL]${R}`, 20));
        }

        // 3. Pending Input Count
        if (CONFIG.showPendingInput && pendingInputCount > 0) {
            widgets.push(w(`${FG_BRIGHT_YELLOW}> ${pendingInputCount}${R}`, 2 + String(pendingInputCount).length));
        }

        // 4. Model & Quota
        if (model && (CONFIG.showModel || CONFIG.showQuota)) {
            const qInfo = CONFIG.showQuota ? getModelQuotaString(model, step >= 6 || cols < 80) : "";
            const showModelName = CONFIG.showModel;
            
            if (showModelName || qInfo) {
                let text = "";
                if (showModelName && qInfo) {
                    const modelName = step >= 4 ? getShortModelName(model) : model;
                    const tierTag = (CONFIG.showPlanTier && planTier) ? ` [${planTier}]` : "";
                    text = `${FG_GRAY}${I}${modelName}${tierTag}${R} ${FG_GRAY}|${R} ${qInfo}`;
                } else if (showModelName) {
                    const modelName = step >= 4 ? getShortModelName(model) : model;
                    const tierTag = (CONFIG.showPlanTier && planTier) ? ` [${planTier}]` : "";
                    text = `${FG_GRAY}${I}${modelName}${tierTag}${R}`;
                } else {
                    text = qInfo;
                }
                widgets.push(w(text, getVisualLength(text)));
            }
        }

        // 5. Path
        if (CONFIG.showPath && cwd && step < 5) {
            let pathText = "";
            if (step >= 3) {
                let pathNorm = cwd.replace(/\\/g, "/");
                const parts = pathNorm.split("/");
                pathText = parts[parts.length - 1] || pathNorm;
            } else {
                pathText = getShortenPath(cwd);
            }
            if (pathText) {
                widgets.push(w(`${FG_BRIGHT_BLUE}${pathText}${R}`, pathText.length));
            }
        }

        // 6. VCS (Git Branch)
        if (CONFIG.showVcs && vcsBranch && step < 6) {
            let branchText = vcsBranch;
            if (step >= 4) {
                branchText = vcsBranch.length > 10 ? vcsBranch.slice(0, 8) + ".." : vcsBranch;
            } else {
                branchText = vcsBranch.length > 15 ? vcsBranch.slice(0, 12) + ".." : vcsBranch;
            }
            const vcsLabel = `@${branchText}`;
            const vcsFmt = vcsDirty ? `${FG_BRIGHT_BLUE}${vcsLabel}${FG_BRIGHT_YELLOW}*${R}` : `${FG_BRIGHT_BLUE}${vcsLabel}${R}`;
            widgets.push(w(vcsFmt, vcsLabel.length + (vcsDirty ? 1 : 0)));
        }

        // 7. Metadata (Email, Version, ConvID)
        if (step < 2) {
            if (CONFIG.showEmail && email) {
                widgets.push(w(email, email.length));
            }
            if (CONFIG.showVersion && version) {
                widgets.push(w(`v${version}`, 1 + version.length));
            }
            if (CONFIG.showConversationId && convId) {
                widgets.push(w(`id:${convId.slice(0, 8)}`, 3 + Math.min(8, convId.length)));
            }
        }

        return widgets;
    }

    function getMetricWidgetsAtStep(step) {
        const widgets = [];

        // 1. Context Token Bar
        if (CONFIG.showContextBar && step < 11) {
            let barLen = 15;
            let detailMode = 0;

            if (step >= 10) {
                barLen = 0;
                detailMode = 3;
            } else if (step >= 9) {
                barLen = 4;
                detailMode = 3;
            } else if (step >= 7) {
                barLen = 8;
                detailMode = 3;
            } else if (step >= 6) {
                barLen = 10;
                detailMode = 2;
            } else if (step >= 5) {
                barLen = 12;
                detailMode = 1;
            }

            let barText = "";
            let barVisualLen = 0;

            if (barLen > 0) {
                let barColor = FG_BRIGHT_CYAN;
                if (pctInt >= 90) barColor = FG_BRIGHT_RED;
                else if (pctInt >= 60) barColor = FG_BRIGHT_YELLOW;

                if (barLen > 2) {
                    let innerLen = barLen - 2;
                    let filled = Math.min(innerLen, Math.max(0, Math.floor(pctInt * innerLen / 100)));
                    let barFilled = "=".repeat(filled);
                    let barEmpty = "";
                    if (filled < innerLen) {
                        barFilled += ">";
                        barEmpty = "-".repeat(innerLen - filled - 1);
                    }
                    barText = `${FG_GRAY}[${R}${barColor}${barFilled}${FG_GRAY}${barEmpty}]${R}`;
                    barVisualLen = barLen;
                } else {
                    barText = `${barColor}${pctInt}%${R}`;
                    barVisualLen = String(pctInt).length + 1;
                }
            }

            const ctxUsed = inputTok + outputTok;
            let detailText = "";
            
            if (detailMode === 0) {
                let curTokFmt = "";
                if (curIn > 0 || curOut > 0) {
                    curTokFmt = `cur:${getHumanFormat(curIn)}/${getHumanFormat(curOut)}`;
                }
                if (ctxUsed > 0 && limitTok > 0) {
                    const usedFmt = getHumanFormat(ctxUsed);
                    const limitFmt = getHumanFormat(limitTok);
                    if (outputTok > 0 || curTokFmt) {
                        const inFmt = getHumanFormat(inputTok);
                        const outFmt = getHumanFormat(outputTok);
                        detailText = ` (${usedFmt}/${limitFmt} | in:${inFmt}/out:${outFmt}${curTokFmt ? ` | ${curTokFmt}` : ""})`;
                    } else {
                        detailText = ` (${usedFmt}/${limitFmt})`;
                    }
                }
            } else if (detailMode === 1) {
                if (ctxUsed > 0 && limitTok > 0) {
                    const usedFmt = getHumanFormat(ctxUsed);
                    const limitFmt = getHumanFormat(limitTok);
                    const inFmt = getHumanFormat(inputTok);
                    const outFmt = getHumanFormat(outputTok);
                    detailText = ` (${usedFmt}/${limitFmt} | in:${inFmt}/out:${outFmt})`;
                }
            } else if (detailMode === 2) {
                if (ctxUsed > 0 && limitTok > 0) {
                    const usedFmt = getHumanFormat(ctxUsed);
                    const limitFmt = getHumanFormat(limitTok);
                    detailText = ` (${usedFmt}/${limitFmt})`;
                }
            }

            let fullCtxStr = "";
            let fullCtxLen = 0;
            if (barLen > 0) {
                fullCtxStr = `${FG_GRAY}ctx${R} ${barText} ${NUM_COLOR}${pctFmt}%${R}${FG_GRAY}${detailText}${R}`;
                fullCtxLen = 4 + barVisualLen + 1 + pctFmt.length + 1 + detailText.length;
            } else {
                fullCtxStr = `${FG_GRAY}ctx${R} ${NUM_COLOR}${pctFmt}%${R}`;
                fullCtxLen = 4 + pctFmt.length + 1;
            }
            widgets.push(w(fullCtxStr, fullCtxLen));
        }

        // 2. Cache
        if (CONFIG.showCacheStats && (cacheRead > 0 || cacheCreate > 0) && step < 3) {
            const rdFmt = getHumanFormat(cacheRead);
            const wrFmt = getHumanFormat(cacheCreate);
            widgets.push(w(`${FG_GRAY}cache${R} ${NUM_COLOR}rd:${rdFmt}/wr:${wrFmt}${R}`, 13 + rdFmt.length + wrFmt.length));
        }

        // 3. Artifacts
        if (CONFIG.showArtifacts && artifacts > 0 && step < 6) {
            widgets.push(w(`${FG_GRAY}artifacts${R} ${NUM_COLOR}${artifacts}${R}`, 10 + String(artifacts).length));
        }

        // 4. Subagents
        if (CONFIG.showSubagents && subagentsCount > 0 && step < 8) {
            widgets.push(w(`${FG_GRAY}subagents${R} ${NUM_COLOR}${subagentsCount}${R}`, 10 + String(subagentsCount).length));
        }

        // 5. Tasks
        if (CONFIG.showTasks && bgTasks > 0 && step < 8) {
            widgets.push(w(`${FG_GRAY}tasks${R} ${NUM_COLOR}${bgTasks}${R}`, 6 + String(bgTasks).length));
        }

        // 6. Sandbox
        if (CONFIG.showSandbox && sandbox && step < 4) {
            const sbFmt = sandboxAllowNetwork ? 
                `${FG_GRAY}sandbox${R} ${FG_BRIGHT_GREEN}${B}ON(net)${R}` : 
                `${FG_GRAY}sandbox${R} ${FG_BRIGHT_GREEN}${B}ON(no-net)${R}`;
            const sbLen = sandboxAllowNetwork ? 15 : 18;
            widgets.push(w(sbFmt, sbLen));
        }

        return widgets;
    }

    const maxInfoStep = 6;
    const maxMetricStep = 11;

    const maxW = (cols >= 80) ? (cols - 4) : (cols - 2);
    const maxMetricW = (cols >= 80) ? (cols - 5) : (cols - 2);

    const getRowWidth = (list, sepLen) => list.reduce((sum, wItem) => sum + wItem.len, 0) + (list.length > 0 ? sepLen * (list.length - 1) : 0);

    // Determine active min/max steps dynamically based on terminal width
    let minInfoStep = 0;
    let minMetricStep = 0;

    if (cols >= 160) {
        minInfoStep = 0;
        minMetricStep = 0;
    } else if (cols >= 120) {
        minInfoStep = 3;   // Double-line: path shortened
        minMetricStep = 0; // Double-line: show full cache/artifacts, bar not shortened
    } else if (cols >= 80) {
        minInfoStep = 3;   // Double-line: path shortened
        minMetricStep = 5; // Double-line: ctx bar shortened to 12, keep artifacts
    } else if (cols >= 60) {
        minInfoStep = 5;   // Double-line: hide path, keep VCS (shortened), hide quota time
        minMetricStep = 6; // Double-line: ctx bar shortened to 10 and no in/out detail
    } else {
        minInfoStep = 6;   // Double-line: hide path, hide VCS, hide quota time
        minMetricStep = 6; // Double-line: ctx bar shortened to 10 and no in/out detail
    }

    // Try single-line rendering
    let singleLineWidgets = null;
    let foundSingleLine = false;

    if (cols >= 160) {
        for (let s = minInfoStep; s <= Math.max(maxInfoStep, maxMetricStep); s++) {
            const sInfo = Math.min(s, maxInfoStep);
            const sMetric = Math.min(s, maxMetricStep);
            const infoW = getInfoWidgetsAtStep(sInfo);
            const metricW = getMetricWidgetsAtStep(sMetric);
            const combined = [...infoW, ...metricW];
            const totalW = getRowWidth(combined, 3);
            if (totalW <= maxW) {
                if (s <= 2) {
                    singleLineWidgets = combined;
                    foundSingleLine = true;
                    break;
                }
            }
        }
    }

    let allRenderedRows;
    if (foundSingleLine) {
        allRenderedRows = [singleLineWidgets.map(wItem => wItem.text).join(`${FG_GRAY} | ${R}`)];
    } else {
        // Double-line layout
        let infoWidgets = null;
        for (let s = minInfoStep; s <= maxInfoStep; s++) {
            const widgets = getInfoWidgetsAtStep(s);
            if (getRowWidth(widgets, 3) <= maxW) {
                infoWidgets = widgets;
                break;
            }
            if (s === maxInfoStep) infoWidgets = widgets;
        }

        let metricWidgets = null;
        for (let s = minMetricStep; s <= maxMetricStep; s++) {
            const widgets = getMetricWidgetsAtStep(s);
            if (getRowWidth(widgets, 3) <= maxMetricW) {
                metricWidgets = widgets;
                break;
            }
            if (s === maxMetricStep) metricWidgets = widgets;
        }

        const infoRowText = infoWidgets.map(wItem => wItem.text).join(`${FG_GRAY} | ${R}`);
        if (metricWidgets.length > 0) {
            const metricRowText = metricWidgets.map(wItem => wItem.text).join(`${FG_GRAY} | ${R}`);
            allRenderedRows = [infoRowText, metricRowText];
        } else {
            allRenderedRows = [infoRowText];
        }
    }

    // ─── Rendering output ───────────────────────────────────────────────────
    if (cols >= 80) {
        if (allRenderedRows.length === 1) {
            console.log(`${FG_GRAY}╭─${R} ${allRenderedRows[0]}`);
        } else if (allRenderedRows.length === 2) {
            console.log(`${FG_GRAY}╭─${R} ${allRenderedRows[0]}`);
            console.log(`${FG_GRAY}╰─${R} ${allRenderedRows[1]}`);
        } else {
            console.log(`${FG_GRAY}╭─${R} ${allRenderedRows[0]}`);
            for (let i = 1; i < allRenderedRows.length - 1; i++) {
                console.log(`${FG_GRAY}├─${R} ${allRenderedRows[i]}`);
            }
            console.log(`${FG_GRAY}╰─${R} ${allRenderedRows[allRenderedRows.length - 1]}`);
        }
    } else {
        console.log(allRenderedRows.join('\n'));
    }

    // ─── Deferred SWR Background Refresher ──────────────────────────────────
    if (CONFIG.showQuota || CONFIG.showVcs) {
        try {
            let needRefresh = false;
            const cached = loadStatusCache();
            const cachedCwd = (cached.vcs && cached.vcs.cwd) || "";

            if (!fs.existsSync(STATUS_CACHE_PATH)) {
                needRefresh = true;
            } else {
                const stat = fs.statSync(STATUS_CACHE_PATH);
                const lastConfigUpdate = getConfigsLastModifiedTime();

                // 1. Trigger immediately on CWD change
                if (cwd && cwd !== cachedCwd) {
                    needRefresh = true;
                } else if (lastConfigUpdate > stat.mtimeMs) {
                    needRefresh = true;
                } else {
                    // 2. Trigger on expiration (2 minutes)
                    const age = Date.now() - stat.mtimeMs;
                    if (age > 2 * 60 * 1000) {
                        needRefresh = true;
                    }
                }
            }

            if (needRefresh) {
                let lockActive = false;
                try {
                    if (fs.existsSync(LOCK_FILE_PATH)) {
                        const lockStat = fs.statSync(LOCK_FILE_PATH);
                        const lockAge = Date.now() - lockStat.mtimeMs;
                        if (lockAge < 30000) {
                            const lockContent = JSON.parse(fs.readFileSync(LOCK_FILE_PATH, 'utf8'));
                            if (path.resolve(lockContent.cwd) === path.resolve(cwd)) {
                                lockActive = true;
                            }
                        }
                    }
                } catch (e) {}

                if (!lockActive) {
                    const { spawn } = require('child_process');
                    const subprocess = spawn(process.execPath, [
                        __filename,
                        '--refresh',
                        ...(cwd ? ['--cwd', cwd] : [])
                    ], {
                        detached: true,
                        stdio: 'ignore',
                        windowsHide: true,
                    });
                    subprocess.unref();
                }
            }
        } catch (e) {}
    }
});
