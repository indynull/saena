//! One Lightning Memory-Mapped Database. The tree is the `node` table.

use crate::node::Node;
use anyhow::{anyhow, Context, Result};
use heed::types::{ByteSlice, Str};
use heed::{Database, Env, EnvOpenOptions};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

const MAP_SIZE: usize = 64 * 1024 * 1024;

pub struct Store {
    env: Env,
    node: Database<Str, ByteSlice>,
    blob: Database<Str, ByteSlice>,
    meta: Database<Str, ByteSlice>,
}

impl Store {
    pub fn open(root: &Path) -> Result<Self> {
        std::fs::create_dir_all(root.join("store.lmdb")).context("store.lmdb")?;
        std::fs::create_dir_all(root.join("bytes")).context("bytes")?;
        let layout = root.join("layout");
        if !layout.exists() {
            std::fs::write(layout, "1\n")?;
        }
        let env = EnvOpenOptions::new()
            .map_size(MAP_SIZE)
            .max_dbs(8)
            .open(root.join("store.lmdb"))
            .map_err(|err| anyhow!("open lmdb: {err}"))?;
        let node = env
            .create_database(Some("node"))
            .map_err(|err| anyhow!("node: {err}"))?;
        let blob = env
            .create_database(Some("blob"))
            .map_err(|err| anyhow!("blob: {err}"))?;
        let meta = env
            .create_database(Some("meta"))
            .map_err(|err| anyhow!("meta: {err}"))?;
        Ok(Self {
            env,
            node,
            blob,
            meta,
        })
    }

    pub fn load(&self) -> Result<Tree> {
        let rtxn = self.env.read_txn().map_err(|err| anyhow!("{err}"))?;
        let mut nodes = BTreeMap::new();
        let iter = self.node.iter(&rtxn).map_err(|err| anyhow!("{err}"))?;
        for item in iter {
            let (key, bytes) = item.map_err(|err| anyhow!("{err}"))?;
            let node: Node = serde_json::from_slice(bytes)?;
            nodes.insert(key.to_string(), node);
        }
        let tip = meta_get(&rtxn, self.meta, "tip")?;
        let gen = meta_get(&rtxn, self.meta, "gen")?
            .and_then(|v| v.parse().ok())
            .unwrap_or(0);
        let mut digests = BTreeMap::new();
        if let Some(raw) = meta_get(&rtxn, self.meta, "digests")? {
            if let Ok(map) = serde_json::from_str::<BTreeMap<String, String>>(&raw) {
                digests = map;
            }
        }
        Ok(Tree {
            nodes,
            tip,
            gen,
            digests,
            blobs: BTreeMap::new(),
            pin: meta_get(&rtxn, self.meta, "pin")?,
        })
    }

    pub fn save(&self, tree: &Tree) -> Result<()> {
        let mut wtxn = self.env.write_txn().map_err(|err| anyhow!("{err}"))?;
        let existing: Vec<String> = self
            .node
            .iter(&wtxn)
            .map_err(|err| anyhow!("{err}"))?
            .filter_map(|item| item.ok().map(|(k, _)| k.to_string()))
            .collect();
        for key in existing {
            if !tree.nodes.contains_key(&key) {
                let _ = self.node.delete(&mut wtxn, &key);
            }
        }
        for (id, node) in &tree.nodes {
            let bytes = serde_json::to_vec(node)?;
            self.node
                .put(&mut wtxn, id, &bytes)
                .map_err(|err| anyhow!("put {id}: {err}"))?;
        }
        match &tree.tip {
            Some(tip) => self
                .meta
                .put(&mut wtxn, "tip", tip.as_bytes())
                .map_err(|err| anyhow!("tip: {err}"))?,
            None => {
                let _ = self.meta.delete(&mut wtxn, "tip");
            }
        }
        let gen = tree.gen.to_string();
        self.meta
            .put(&mut wtxn, "gen", gen.as_bytes())
            .map_err(|err| anyhow!("gen: {err}"))?;
        let digests = serde_json::to_string(&tree.digests)?;
        self.meta
            .put(&mut wtxn, "digests", digests.as_bytes())
            .map_err(|err| anyhow!("digests: {err}"))?;
        for (key, bytes) in &tree.blobs {
            self.blob
                .put(&mut wtxn, key, bytes)
                .map_err(|err| anyhow!("blob {key}: {err}"))?;
        }
        match &tree.pin {
            Some(pin) => self
                .meta
                .put(&mut wtxn, "pin", pin.as_bytes())
                .map_err(|err| anyhow!("pin: {err}"))?,
            None => {
                let _ = self.meta.delete(&mut wtxn, "pin");
            }
        }
        wtxn.commit().map_err(|err| anyhow!("commit: {err}"))?;
        Ok(())
    }

    pub fn blob_get(&self, key: &str) -> Result<Option<Vec<u8>>> {
        let rtxn = self.env.read_txn().map_err(|err| anyhow!("{err}"))?;
        match self.blob.get(&rtxn, key) {
            Ok(Some(bytes)) => Ok(Some(bytes.to_vec())),
            Ok(None) => Ok(None),
            Err(err) => Err(anyhow!("{err}")),
        }
    }
}

fn meta_get(
    rtxn: &heed::RoTxn<'_>,
    db: Database<Str, ByteSlice>,
    key: &str,
) -> Result<Option<String>> {
    match db.get(rtxn, key) {
        Ok(Some(bytes)) => Ok(Some(std::str::from_utf8(bytes)?.to_string())),
        Ok(None) => Ok(None),
        Err(err) => Err(anyhow!("{err}")),
    }
}

#[derive(Clone, Debug, Default)]
pub struct Tree {
    pub nodes: BTreeMap<String, Node>,
    pub tip: Option<String>,
    pub gen: u64,
    pub digests: BTreeMap<String, String>,
    pub blobs: BTreeMap<String, Vec<u8>>,
    pub pin: Option<String>,
}

impl Tree {
    pub fn get(&self, id: &str) -> Option<&Node> {
        self.nodes.get(id)
    }

    pub fn get_mut(&mut self, id: &str) -> Option<&mut Node> {
        self.nodes.get_mut(id)
    }

    pub fn put(&mut self, node: Node) {
        self.nodes.insert(node.id.clone(), node);
    }

    pub fn resolve_deed(&self, id: &str) -> String {
        if let Some(digest) = id.strip_prefix("sha256:") {
            self.digests
                .get(digest)
                .cloned()
                .unwrap_or_else(|| id.to_string())
        } else {
            id.to_string()
        }
    }
}

pub fn lock_path(root: &Path) -> PathBuf {
    root.join(".lock")
}

pub fn acquire(lock: &Path) -> Result<()> {
    for _ in 0..50 {
        match std::fs::create_dir(lock) {
            Ok(()) => return Ok(()),
            Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => {
                if stale(lock) {
                    let _ = std::fs::remove_dir_all(lock);
                }
                std::thread::sleep(std::time::Duration::from_millis(20));
            }
            Err(err) => return Err(err.into()),
        }
    }
    let _ = std::fs::remove_dir_all(lock);
    std::fs::create_dir(lock).context("desk lock timeout")?;
    Ok(())
}

fn stale(lock: &Path) -> bool {
    match std::fs::metadata(lock).and_then(|m| m.modified()) {
        Ok(mtime) => mtime.elapsed().map(|d| d.as_secs() > 2).unwrap_or(true),
        Err(_) => true,
    }
}

pub fn release(lock: &Path) {
    let _ = std::fs::remove_dir_all(lock);
}
