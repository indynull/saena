//! A checkout names itself to the home desk.
//!
//! `saena init` writes `.saena/project`. `Desk::sit` walks up
//! from the workspace and uses that name as the inbox and the
//! snapshot root.

use crate::error::{Error, Result};
use std::path::{Path, PathBuf};

pub struct Project {
    pub name: String,
    pub root: PathBuf,
}

impl Project {
    pub fn file(root: &Path) -> PathBuf {
        root.join(".saena").join("project")
    }

    pub fn find(start: &Path) -> Option<Self> {
        let mut dir = start.to_path_buf();
        loop {
            if let Some(project) = Self::read(&dir) {
                return Some(project);
            }
            if !dir.pop() {
                return None;
            }
        }
    }

    pub fn read(root: &Path) -> Option<Self> {
        let name = std::fs::read_to_string(Self::file(root)).ok()?;
        let name = name.trim();
        if name.is_empty() {
            return None;
        }
        Some(Self {
            name: name.to_string(),
            root: root.to_path_buf(),
        })
    }

    pub fn write(root: &Path, name: &str) -> Result<Self> {
        let name = name.trim();
        if name.is_empty() {
            return Err(Error::Usage);
        }
        let path = Self::file(root);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|_| Error::Usage)?;
        }
        std::fs::write(&path, format!("{name}\n")).map_err(|_| Error::Usage)?;
        Ok(Self {
            name: name.to_string(),
            root: root.to_path_buf(),
        })
    }
}
