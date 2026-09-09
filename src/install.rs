//! Put `saena` on the path and the Model Context Protocol block in harness configs.

use crate::error::{Error, Result};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

pub const SKILL: &str = "\
---
name: saena
description: Desk for a sitting when the saena server is enabled. Claim work, complete deeds, remember. Ignore this skill unless the saena server is connected.
---

Use only when the saena server is enabled for this session. Call
`status` first. Claim ready work, or `work` then claim. File work
for another project with `--project`, mint work there, `depend`,
then `wait` for the deed. When the work is done, `complete` —
the host snapshots what changed. Do not ask for SAENA_DESK or
file paths. The next sitting opens the tip deed.
";

pub struct Install {
    pub bin: PathBuf,
    pub block: Value,
    pub home: PathBuf,
}

pub fn skill() -> &'static str {
    SKILL
}

pub fn run(bin: Option<&Path>, home: Option<&Path>, source: Option<&Path>) -> Result<Install> {
    let home = home
        .map(Path::to_path_buf)
        .or_else(|| std::env::var("HOME").ok().map(PathBuf::from))
        .ok_or(Error::NoBinary)?;
    let bin_dir = bin
        .map(Path::to_path_buf)
        .unwrap_or_else(|| home.join(".local/bin"));
    let source = source
        .map(Path::to_path_buf)
        .or_else(self_binary)
        .ok_or(Error::NoBinary)?;
    if !source.is_file() {
        return Err(Error::NoBinary);
    }
    let dest = bin_dir.join("saena");
    let _ = std::fs::create_dir_all(&bin_dir);
    std::fs::copy(&source, &dest).map_err(|_| Error::NoBinary)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&dest, std::fs::Permissions::from_mode(0o755));
    }
    let block = mcp_block(&dest);
    write_configs(&home, &dest, &block);
    write_skills(&home);
    Ok(Install {
        bin: dest,
        block,
        home,
    })
}

pub fn mcp_block(command: &Path) -> Value {
    json!({
        "mcpServers": {
            "saena": {
                "command": command,
                "args": ["mcp"],
                "enabled": false
            }
        }
    })
}

fn self_binary() -> Option<PathBuf> {
    std::env::current_exe().ok().filter(|p| p.is_file())
}

fn write_skills(home: &Path) {
    for rel in [
        ".grok/skills/saena/SKILL.md",
        ".claude/skills/saena/SKILL.md",
        ".cursor/skills/saena/SKILL.md",
        ".agents/skills/saena/SKILL.md",
    ] {
        let path = home.join(rel);
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::write(path, SKILL);
    }
}

fn write_configs(home: &Path, dest: &Path, block: &Value) {
    let server = block["mcpServers"]["saena"].clone();
    write_json(&home.join(".config/saena/mcp.json"), |_| block.clone());
    merge_mcp(&home.join(".cursor/mcp.json"), &server);
    merge_mcp(&home.join(".claude.json"), &server);
    merge_mcp(&home.join(".omp/agent/mcp.json"), &server);
    merge_mcp(&home.join(".gemini/settings.json"), &server);
    merge_opencode(&home.join(".config/opencode/opencode.json"), dest);
    append_toml(&home.join(".grok/config.toml"), dest);
    append_toml(&home.join(".codex/config.toml"), dest);
}

fn merge_mcp(path: &Path, server: &Value) {
    write_json(path, |mut current| {
        let servers = current
            .as_object_mut()
            .and_then(|o| o.get_mut("mcpServers"))
            .and_then(Value::as_object_mut);
        if let Some(servers) = servers {
            servers.insert("saena".into(), server.clone());
            current
        } else if let Some(obj) = current.as_object_mut() {
            obj.insert("mcpServers".into(), json!({"saena": server}));
            current
        } else {
            json!({"mcpServers": {"saena": server}})
        }
    });
}

fn merge_opencode(path: &Path, dest: &Path) {
    write_json(path, |mut current| {
        if !current.is_object() {
            current = json!({});
        }
        current["mcp"]["saena"] = json!({
            "type": "local",
            "command": [dest],
            "args": ["mcp"],
            "enabled": false
        });
        current
    });
}

fn append_toml(path: &Path, dest: &Path) {
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let existing = std::fs::read_to_string(path).unwrap_or_default();
    if existing.contains("[mcp_servers.saena]") {
        return;
    }
    let block = format!(
        "\n[mcp_servers.saena]\ncommand = \"{}\"\nargs = [\"mcp\"]\nenabled = false\n",
        dest.display()
    );
    let _ = std::fs::write(path, existing + block.as_str());
}

fn write_json(path: &Path, edit: impl FnOnce(Value) -> Value) {
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let current = std::fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_else(|| json!({}));
    let next = edit(current);
    let _ = std::fs::write(
        path,
        serde_json::to_string_pretty(&next).unwrap_or_default() + "\n",
    );
}
