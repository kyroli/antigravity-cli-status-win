const path = require('path');

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
    } catch (e) {
        // Fallback to defaults
    }

    const state = json.agent_state || "idle";
    let cwd = "";
    if (json.workspace && json.workspace.current_dir) {
        cwd = json.workspace.current_dir;
    } else if (json.cwd) {
        cwd = json.cwd;
    }

    let workspace = "unknown";
    if (cwd) {
        const cwdNormalized = cwd.replace(/\\/g, "/");
        if (cwdNormalized.includes("/google/src/cloud/")) {
            const match = cwdNormalized.match(/\/google\/src\/cloud\/[^/]+\/([^/]+)/);
            workspace = match ? match[1] : path.basename(cwd);
        } else {
            workspace = path.basename(cwd) || cwdNormalized;
        }
    }

    let emoji = "🤖";
    switch (state) {
        case "initializing":
            emoji = "🚀";
            break;
        case "idle":
            emoji = "😴";
            break;
        case "thinking":
            emoji = "🤔";
            break;
        case "working":
            emoji = "🏃";
            break;
        case "tool_use":
            emoji = "🛠️";
            break;
    }

    console.log(`${emoji} ${state} | ${workspace}`);
});
