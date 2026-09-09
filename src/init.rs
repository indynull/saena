//! Opt this directory into saena. The machine-wide server stays off.

use crate::desk::Desk;
use crate::error::{Error, Result};
use crate::project::Project;
use serde_json::json;
use std::path::{Path, PathBuf};

pub struct Init {
    pub root: PathBuf,
    pub desk: PathBuf,
    pub project: String,
}

pub fn run(root: Option<&Path>, name: Option<&str>, store: Option<&Path>) -> Result<Init> {
    let root = root
        .map(Path::to_path_buf)
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
    std::fs::create_dir_all(&root).ok();
    let name = name
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| {
            root.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("project")
                .to_string()
        });
    Project::write(&root, &name)?;
    write_grok(&root);
    write_mcp(&root);
    write_skill(&root);
    write_agents(&root);
    let desk = store.map(Path::to_path_buf).unwrap_or_else(Desk::home_root);
    let opened = Desk::open(&desk)?;
    opened.ensure_project(&name, &root)?;
    Ok(Init {
        root,
        desk,
        project: name,
    })
}

fn write_grok(root: &Path) {
    let path = root.join(".grok/config.toml");
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(
        path,
        "[mcp_servers.saena]\n\
         command = \"saena\"\n\
         args = [\"mcp\"]\n\
         enabled = true\n\
         startup_timeout_sec = 10\n",
    );
}

fn write_mcp(root: &Path) {
    let body = json!({
        "mcpServers": {
            "saena": {
                "command": "saena",
                "args": ["mcp"],
                "enabled": true
            }
        }
    });
    let _ = std::fs::write(
        root.join(".mcp.json"),
        serde_json::to_string_pretty(&body).unwrap_or_default() + "\n",
    );
}

fn write_skill(root: &Path) {
    let path = root.join(".grok/skills/saena/SKILL.md");
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(path, crate::install::skill());
}

fn write_agents(root: &Path) {
    let path = root.join("AGENTS.md");
    if !path.exists() {
        let _ = std::fs::write(
            path,
            "This project sits at a saena desk. When the saena server is\n\
             connected, call `status` first. Claim work. Complete when done.\n\
             The host snapshots what changed. File work for another project\n\
             with `--project`, then `wait` for the deed.\n",
        );
    }
}

#[allow(dead_code)]
pub fn fail() -> Error {
    Error::Usage
}
