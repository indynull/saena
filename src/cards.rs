//! Human-edited cards next to the desk. Not atoms.

use std::path::{Path, PathBuf};

const NAMES: &[&str] = &["USER.md", "MEMORY.md"];

pub fn paths(tree: &Path) -> [PathBuf; 2] {
    [tree.join("USER.md"), tree.join("MEMORY.md")]
}

pub fn read(tree: &Path) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for name in NAMES {
        let path = tree.join(name);
        if let Ok(body) = std::fs::read_to_string(&path) {
            out.push(((*name).to_string(), body));
        }
    }
    out
}

pub fn dump(tree: &Path) -> String {
    let cards = read(tree);
    if cards.is_empty() {
        return "cards\n  (none)\n".into();
    }
    let mut s = String::from("cards\n");
    for (name, body) in cards {
        s.push_str("  ");
        s.push_str(&name);
        s.push('\n');
        for line in body.lines() {
            s.push_str("    ");
            s.push_str(line);
            s.push('\n');
        }
    }
    s
}
