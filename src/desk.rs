//! One desk. One tree of nodes. The host is the sole mutator.

use crate::error::{Error, Result};
use crate::extract::{claim_from_user, is_tool_dump};
use crate::id;
use crate::node::{today, Kind, Node, Status};
use crate::panel::{self, Hit};
use crate::project::Project;
use crate::stamp;
use crate::store::{self, Store, Tree};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};

const SKIP: &[&str] = &[
    "desk",
    ".saena",
    "pack",
    "bytes",
    ".lock",
    ".git",
    "_build",
    "deps",
    "node_modules",
    ".elixir_ls",
    "target",
    "store.lmdb",
    "search.milli",
    "layout",
    "tsq",
    ".txn.json",
    ".dirty",
];

pub struct Desk {
    pub root: PathBuf,
    pub tree: PathBuf,
    pub project: Option<String>,
}

pub struct Glance {
    pub desk: PathBuf,
    pub ready: Vec<Node>,
    pub tip: Option<String>,
    pub last: Option<Node>,
    pub gen: u64,
}

pub struct Complete {
    pub work: Node,
    pub deed: Option<Node>,
}

pub struct Related {
    pub id: String,
    pub score: f64,
    pub summary: String,
}

pub struct PlanTree {
    pub id: String,
    pub children: Vec<PlanTree>,
    pub blocked_by: Vec<String>,
}

impl Desk {
    pub fn default_root() -> PathBuf {
        Self::home_root()
    }

    pub fn home_root() -> PathBuf {
        if let Ok(env) = std::env::var("SAENA_DESK") {
            if !env.is_empty() {
                return PathBuf::from(env);
            }
        }
        data_home().join("saena").join("desk")
    }

    /// Open the home desk and the nearest `.saena/project`.
    ///
    /// Call this when the operator did not pass `--desk`. Ready
    /// work is that project's inbox. `complete` snapshots that
    /// folder.
    pub fn sit() -> Result<Self> {
        let mut desk = Self::open(Self::home_root())?;
        if let Some(found) = Project::find(&workspace()) {
            desk.tree = found.root;
            desk.project = Some(found.name);
        }
        Ok(desk)
    }

    pub fn for_session(explicit: Option<&Path>) -> Result<Self> {
        match explicit {
            Some(path) => Self::open(path),
            None => Self::sit(),
        }
    }

    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let root = path.as_ref().to_path_buf();
        std::fs::create_dir_all(&root).ok();
        std::fs::create_dir_all(root.join("store.lmdb")).ok();
        std::fs::create_dir_all(root.join("bytes")).ok();
        let layout = root.join("layout");
        if !layout.exists() {
            let _ = std::fs::write(layout, "1\n");
        }
        let tree = infer_tree(&root);
        Ok(Self {
            root,
            tree,
            project: None,
        })
    }

    pub fn format_status(&self) -> String {
        let glance = self.status();
        let ready = if glance.ready.is_empty() {
            "(none)".into()
        } else {
            glance
                .ready
                .iter()
                .map(|n| format!("{}  {}", n.id, n.summary))
                .collect::<Vec<_>>()
                .join(", ")
        };
        let tip = glance.tip.unwrap_or_else(|| "(none)".into());
        let last = glance
            .last
            .map(|a| format!("{}  {}", a.id, a.text))
            .unwrap_or_else(|| "(none)".into());
        format!(
            "desk  {}\nproject {}\nready {}\ntip   {}\nlast  {}",
            glance.desk.display(),
            self.project.as_deref().unwrap_or("(none)"),
            ready,
            tip,
            last
        )
    }

    pub fn status(&self) -> Glance {
        let state = self.load();
        Glance {
            desk: self.root.clone(),
            ready: self.ready_in(&state),
            tip: state.tip.clone(),
            last: state
                .nodes
                .values()
                .filter(|n| n.kind == Kind::Atom)
                .max_by(|a, b| a.ts.cmp(&b.ts))
                .cloned(),
            gen: state.gen,
        }
    }

    pub fn ready(&self) -> Vec<Node> {
        self.ready_in(&self.load())
    }

    fn ready_in(&self, state: &Tree) -> Vec<Node> {
        let mut out: Vec<Node> = state
            .nodes
            .values()
            .filter(|n| n.kind == Kind::Work && work_claimable(n, state) && self.in_project(n))
            .cloned()
            .collect();
        out.sort_by(|a, b| a.id.cmp(&b.id));
        out
    }

    pub fn upsert(&self, summary: &str) -> Result<Node> {
        self.transact(|state| {
            let mut node = Node::work(id::mint("w"), summary.to_string());
            node.project = self.project.clone();
            state.put(node.clone());
            Ok(node)
        })
    }

    pub fn claim(&self, id: &str, actor: &str, gen: Option<u64>) -> Result<Node> {
        self.transact(|state| {
            let node = state.get(id).cloned().ok_or(Error::NotFound)?;
            if node.kind != Kind::Work {
                return Err(Error::NotWork);
            }
            if node.status.terminal() {
                return Err(Error::Terminal);
            }
            if node.status != Status::Ready || !deps_met(&node, state) {
                return Err(Error::NotReady);
            }
            if let Some(expected) = gen {
                if expected != node.gen {
                    return Err(Error::Conflict);
                }
            }
            let mut node = node;
            node.status = Status::Claimed;
            node.assignee = Some(actor.to_string());
            node.gen += 1;
            node.snap = take_snap(&self.tree);
            state.put(node.clone());
            Ok(node)
        })
    }

    pub fn spawn(&self, parent_id: &str, summary: &str) -> Result<Node> {
        self.transact(|state| {
            let mut parent = state.get(parent_id).cloned().ok_or(Error::NotFound)?;
            if parent.kind != Kind::Work {
                return Err(Error::NotFound);
            }
            let mut child = Node::work(id::mint("w"), summary.to_string());
            child.parent = Some(parent_id.to_string());
            child.project = parent.project.clone();
            parent.children.push(child.id.clone());
            state.put(parent);
            state.put(child.clone());
            Ok(child)
        })
    }

    pub fn link(&self, parent_id: &str, child_id: &str) -> Result<Node> {
        self.transact(|state| {
            let mut parent = state.get(parent_id).cloned().ok_or(Error::NotFound)?;
            let mut child = state.get(child_id).cloned().ok_or(Error::NotFound)?;
            if !parent.succs.contains(&child_id.to_string()) {
                parent.succs.push(child_id.to_string());
            }
            if !child.deps.contains(&parent_id.to_string()) {
                child.deps.push(parent_id.to_string());
            }
            state.put(parent);
            state.put(child.clone());
            Ok(child)
        })
    }

    pub fn depend(&self, id: &str, other: &str) -> Result<Node> {
        self.link(other, id)
    }

    pub fn wait(&self, id: &str, timeout_ms: Option<u64>) -> Result<Node> {
        let start = std::time::Instant::now();
        loop {
            let node = self.load().get(id).cloned().ok_or(Error::NotFound)?;
            let done = match node.kind {
                Kind::Ticket => node.status.ticket_closed(),
                _ => node.status.terminal(),
            };
            if done {
                return Ok(node);
            }
            if let Some(ms) = timeout_ms {
                if start.elapsed().as_millis() as u64 >= ms {
                    return Err(Error::Timeout);
                }
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
    }

    pub fn unlink(&self, parent_id: &str, child_id: &str) -> Result<Node> {
        self.transact(|state| {
            let mut parent = state.get(parent_id).cloned().ok_or(Error::NotFound)?;
            let mut child = state.get(child_id).cloned().ok_or(Error::NotFound)?;
            parent.succs.retain(|id| id != child_id);
            child.deps.retain(|id| id != parent_id);
            state.put(parent);
            state.put(child.clone());
            Ok(child)
        })
    }

    pub fn archive(&self, id: &str) -> Result<Node> {
        self.transact(|state| {
            let mut node = state.get(id).cloned().ok_or(Error::NotFound)?;
            if !node.status.terminal() {
                return Err(Error::NotTerminal);
            }
            node.archived = true;
            state.put(node.clone());
            Ok(node)
        })
    }

    pub fn unarchive(&self, id: &str) -> Result<Node> {
        self.transact(|state| {
            let mut node = state.get(id).cloned().ok_or(Error::NotFound)?;
            node.archived = false;
            state.put(node.clone());
            Ok(node)
        })
    }

    pub fn complete(
        &self,
        id: &str,
        file: Option<&Path>,
        sources: Vec<String>,
        supersedes: Option<String>,
        ticket: Option<String>,
        status: Status,
    ) -> Result<Complete> {
        let product = match file {
            Some(path) => Some(std::fs::read(path).map_err(|_| Error::MissingBytes)?),
            None => None,
        };
        self.transact(|state| {
            let mut node = state.get(id).cloned().ok_or(Error::NotFound)?;
            if node.kind != Kind::Work {
                return Err(Error::NotWork);
            }
            if node.status.terminal() {
                return Err(Error::Terminal);
            }
            if node.status != Status::Claimed {
                return Err(Error::NotClaimed);
            }
            if let Some(t) = ticket {
                node.ticket = Some(t);
            }
            node.status = status;
            let body = match product.as_deref() {
                Some(bytes) => Some(bytes.to_vec()),
                None => {
                    let files = changed_files(&self.tree, &node.snap);
                    if files.is_empty() {
                        None
                    } else {
                        Some(encode_snap(&files))
                    }
                }
            };
            match body {
                None => {
                    state.put(node.clone());
                    Ok(Complete {
                        work: node,
                        deed: None,
                    })
                }
                Some(body) => {
                    let digest = hash(&body);
                    let mut deed = Node::deed(id::mint("d"), node.summary.clone(), digest.clone());
                    deed.produced_by = node.assignee.clone();
                    deed.work = Some(node.id.clone());
                    deed.ticket = node.ticket.clone();
                    deed.sources = sources;
                    deed.supersedes = supersedes.clone();
                    node.deed = Some(deed.id.clone());
                    node.snap.clear();
                    let bytes_path = self.root.join("bytes").join(&deed.id);
                    let _ = std::fs::create_dir_all(self.root.join("bytes"));
                    let _ = std::fs::write(bytes_path, &body);
                    state.digests.insert(digest, deed.id.clone());
                    if let Some(prior) = supersedes {
                        if let Some(old) = state.get_mut(&prior) {
                            old.successor = Some(deed.id.clone());
                        }
                    }
                    state.tip = Some(deed.id.clone());
                    state.blobs.insert(deed.id.clone(), body);
                    state.put(node.clone());
                    state.put(deed.clone());
                    write_host(self, &deed);
                    Ok(Complete {
                        work: node,
                        deed: Some(deed),
                    })
                }
            }
        })
    }

    pub fn get_work(&self, id: &str) -> Result<Node> {
        let node = self.load().get(id).cloned().ok_or(Error::NotFound)?;
        if node.kind != Kind::Work {
            return Err(Error::NotFound);
        }
        Ok(node)
    }

    pub fn deed(&self, id: &str) -> Result<Node> {
        let state = self.load();
        let id = state.resolve_deed(id);
        let node = state.get(&id).cloned().ok_or(Error::NotFound)?;
        if node.kind != Kind::Deed {
            return Err(Error::NotFound);
        }
        if node.tombstone {
            return Err(Error::Deleted);
        }
        Ok(node)
    }

    pub fn product(&self, id: &str) -> Option<Vec<u8>> {
        std::fs::read(self.root.join("bytes").join(id)).ok()
    }

    pub fn files(&self, id: &str) -> BTreeMap<String, Vec<u8>> {
        match self.product(id) {
            Some(body) => decode_snap(&body),
            None => BTreeMap::new(),
        }
    }

    pub fn evidence(&self, id: &str) -> Result<()> {
        let state = self.load();
        let id = state.resolve_deed(id);
        check_evidence(self, &state, &id, &mut HashSet::new())
    }

    pub fn trail(&self, id: &str) -> Vec<String> {
        let state = self.load();
        let id = state.resolve_deed(id);
        walk_trail(&state, &id, Vec::new())
    }

    pub fn current(&self, id: &str) -> Result<Node> {
        let state = self.load();
        let mut id = state.resolve_deed(id);
        loop {
            let node = state.get(&id).cloned().ok_or(Error::NotFound)?;
            match node.successor {
                Some(ref succ) if !succ.is_empty() => id = succ.clone(),
                _ => return Ok(node),
            }
        }
    }

    pub fn leave(&self, id: &str, dest: &Path) -> Result<PathBuf> {
        let deed = self.deed(id)?;
        let body = self.product(&deed.id).ok_or(Error::MissingBytes)?;
        let dir = dest.join(&deed.id);
        std::fs::create_dir_all(&dir).ok();
        let files = decode_snap(&body);
        for (rel, bin) in &files {
            let path = dir.join(rel);
            if let Some(parent) = path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            let _ = std::fs::write(path, bin);
        }
        let mut concat = Vec::new();
        let mut keys: Vec<_> = files.keys().cloned().collect();
        keys.sort();
        for k in keys {
            if let Some(bin) = files.get(&k) {
                concat.extend_from_slice(bin);
            }
        }
        let manifest = serde_json::json!({
            "claim_generator": "saena",
            "id": deed.id,
            "kind": deed.deed_kind.as_deref().unwrap_or("file"),
            "sha256": hash(&concat),
        });
        let _ = std::fs::write(
            dir.join("manifest.json"),
            serde_json::to_string_pretty(&manifest).unwrap_or_default(),
        );
        Ok(dir)
    }

    pub fn timestamp(&self, id: &str) -> Result<PathBuf> {
        let deed = self.deed(id)?;
        let digest =
            hex::decode(deed.sha256.as_deref().unwrap_or("")).map_err(|_| Error::Digest)?;
        Ok(stamp::write(&self.root, &deed.id, &digest))
    }

    pub fn delete_deed(&self, id: &str) -> Result<Node> {
        self.transact(|state| {
            let mut node = state.get(id).cloned().ok_or(Error::NotFound)?;
            if node.kind != Kind::Deed {
                return Err(Error::NotFound);
            }
            node.tombstone = true;
            state.put(node.clone());
            Ok(node)
        })
    }

    pub fn remember(&self, text: &str) -> Result<Node> {
        self.commit_atom(text, "remember")
    }

    pub fn prefer(&self, text: &str) -> Result<Node> {
        self.commit_atom(text, "prefer")
    }

    fn commit_atom(&self, text: &str, hint: &str) -> Result<Node> {
        if is_tool_dump(text) {
            return Err(Error::ToolDump);
        }
        self.transact(|state| {
            let (kind, claim) = match claim_from_user(text) {
                Some((k, c)) => {
                    let k = if hint == "prefer" { "preference" } else { k };
                    (k, c)
                }
                None => (
                    if hint == "prefer" {
                        "preference"
                    } else {
                        "lesson"
                    },
                    text.trim().to_string(),
                ),
            };
            let mut atom = Node::atom(id::mint("m"), kind, claim);
            atom.pin = state.pin.clone();
            state.put(atom.clone());
            Ok(atom)
        })
    }

    pub fn memory(&self) -> Vec<Node> {
        let mut atoms: Vec<Node> = self
            .load()
            .nodes
            .values()
            .filter(|n| n.kind == Kind::Atom)
            .cloned()
            .collect();
        atoms.sort_by(|a, b| a.ts.cmp(&b.ts));
        atoms
    }

    pub fn search(&self, query: &str) -> Vec<Hit> {
        let pin = self.pin_name();
        let hits = crate::search::query(&self.root, query, pin.as_deref(), 20).unwrap_or_default();
        panel::merge_hits(hits)
    }

    pub fn pin_name(&self) -> Option<String> {
        self.load().pin
    }

    pub fn pin(&self, name: &str) -> Result<String> {
        self.transact(|state| {
            state.pin = Some(name.to_string());
            Ok(name.to_string())
        })
    }

    pub fn unpin(&self) -> Result<()> {
        self.transact(|state| {
            state.pin = None;
            Ok(())
        })
    }

    #[allow(clippy::too_many_arguments)]
    pub fn plan(
        &self,
        summary: &str,
        parent: Option<String>,
        deadline: Option<String>,
        scheduled: Option<String>,
        ticket_type: Option<String>,
        tags: Vec<String>,
        priority: Option<String>,
        body: Option<String>,
    ) -> Result<Node> {
        self.transact(|state| {
            let mut ticket = Node::ticket(id::mint("p"), summary.to_string());
            ticket.parent = parent;
            ticket.deadline = deadline;
            ticket.scheduled = scheduled;
            ticket.ticket_type = ticket_type;
            ticket.tags = tags;
            ticket.priority = priority;
            if let Some(body) = body {
                ticket.body = body;
            }
            ticket.project = self.project.clone();
            state.put(ticket.clone());
            Ok(ticket)
        })
    }

    pub fn block(&self, id: &str, blocker: &str) -> Result<Node> {
        self.transact(|state| {
            let mut ticket = state.get(id).cloned().ok_or(Error::NotFound)?;
            if ticket.kind != Kind::Ticket {
                return Err(Error::NotFound);
            }
            if !ticket.blocked_by.contains(&blocker.to_string()) {
                ticket.blocked_by.push(blocker.to_string());
            }
            ticket.status = Status::Blocked;
            state.put(ticket.clone());
            Ok(ticket)
        })
    }

    pub fn close(&self, id: &str) -> Result<Node> {
        self.transact(|state| {
            let mut ticket = state.get(id).cloned().ok_or(Error::NotFound)?;
            if ticket.kind != Kind::Ticket {
                return Err(Error::NotFound);
            }
            ticket.status = Status::Done;
            state.put(ticket.clone());
            Ok(ticket)
        })
    }

    pub fn note(&self, id: &str, text: &str) -> Result<Node> {
        self.transact(|state| {
            let mut ticket = state.get(id).cloned().ok_or(Error::NotFound)?;
            ticket.notes.push(format!("{}  {text}", today()));
            state.put(ticket.clone());
            Ok(ticket)
        })
    }

    pub fn append(&self, id: &str, text: &str) -> Result<Node> {
        self.transact(|state| {
            let mut ticket = state.get(id).cloned().ok_or(Error::NotFound)?;
            let entry = format!("{}  {text}", today());
            if ticket.body.is_empty() {
                ticket.body = entry;
            } else {
                ticket.body = format!("{}\n\n{entry}", ticket.body);
            }
            state.put(ticket.clone());
            Ok(ticket)
        })
    }

    pub fn reject(&self, id: &str, dest: &str) -> Result<Node> {
        self.transact(|state| {
            let mut ticket = state.get(id).cloned().ok_or(Error::NotFound)?;
            ticket.status = Status::Rejected;
            ticket.replacement = Some(dest.to_string());
            state.put(ticket.clone());
            Ok(ticket)
        })
    }

    pub fn resolve(&self, id: &str, dest: &str) -> Result<Node> {
        self.transact(|state| {
            let mut ticket = state.get(id).cloned().ok_or(Error::NotFound)?;
            ticket.status = Status::Resolved;
            ticket.replacement = Some(dest.to_string());
            state.put(ticket.clone());
            Ok(ticket)
        })
    }

    pub fn get_plan(&self, id: &str) -> Result<Node> {
        let node = self.load().get(id).cloned().ok_or(Error::NotFound)?;
        if node.kind != Kind::Ticket {
            return Err(Error::NotFound);
        }
        Ok(node)
    }

    pub fn work_from_plan(&self, ticket_id: &str) -> Result<Node> {
        self.transact(|state| {
            let ticket = state.get(ticket_id).cloned().ok_or(Error::NotFound)?;
            if ticket.kind != Kind::Ticket {
                return Err(Error::NotFound);
            }
            let mut node = Node::work(id::mint("w"), ticket.summary.clone());
            node.ticket = Some(ticket_id.to_string());
            node.project = ticket.project.clone();
            state.put(node.clone());
            Ok(node)
        })
    }

    pub fn ensure_project(&self, name: &str, root: &Path) -> Result<Node> {
        self.transact(|state| {
            if let Some(existing) = state.nodes.values().find(|n| {
                n.kind == Kind::Ticket
                    && n.ticket_type.as_deref() == Some("project")
                    && n.project.as_deref() == Some(name)
            }) {
                return Ok(existing.clone());
            }
            let mut ticket = Node::ticket(id::mint("p"), name.to_string());
            ticket.ticket_type = Some("project".into());
            ticket.project = Some(name.to_string());
            ticket.paths = vec![root.display().to_string()];
            state.put(ticket.clone());
            Ok(ticket)
        })
    }

    fn in_project(&self, node: &Node) -> bool {
        match &self.project {
            None => true,
            Some(name) => node.project.as_deref() == Some(name.as_str()),
        }
    }

    fn plan_visible(&self, node: &Node) -> bool {
        node.kind == Kind::Ticket
            && node.ticket_type.as_deref() != Some("project")
            && self.in_project(node)
    }

    pub fn plan_ready(&self) -> Vec<Node> {
        let state = self.load();
        let mut out: Vec<Node> = state
            .nodes
            .values()
            .filter(|n| self.plan_visible(n) && ticket_ready(n, &state))
            .cloned()
            .collect();
        out.sort_by(|a, b| a.id.cmp(&b.id));
        out
    }

    pub fn plan_list(&self) -> Vec<Node> {
        let mut out: Vec<Node> = self
            .load()
            .nodes
            .values()
            .filter(|n| self.plan_visible(n))
            .cloned()
            .collect();
        out.sort_by(|a, b| a.id.cmp(&b.id));
        out
    }

    pub fn work_list(&self, archived: bool, all: bool, terminal: bool) -> Vec<Node> {
        let mut nodes: Vec<Node> = self
            .load()
            .nodes
            .values()
            .filter(|n| n.kind == Kind::Work && self.in_project(n))
            .cloned()
            .collect();
        nodes.sort_by(|a, b| a.id.cmp(&b.id));
        if all {
            return nodes;
        }
        if archived {
            return nodes.into_iter().filter(|n| n.archived).collect();
        }
        if terminal {
            return nodes.into_iter().filter(|n| n.status.terminal()).collect();
        }
        nodes
            .into_iter()
            .filter(|n| !n.archived && !n.status.terminal())
            .collect()
    }

    pub fn claims(&self) -> Vec<Node> {
        self.work_list(false, true, false)
            .into_iter()
            .filter(|n| n.status == Status::Claimed)
            .collect()
    }

    pub fn children(&self, id: &str) -> Vec<Node> {
        let mut out: Vec<Node> = self
            .load()
            .nodes
            .values()
            .filter(|n| n.kind == Kind::Ticket && n.parent.as_deref() == Some(id))
            .cloned()
            .collect();
        out.sort_by(|a, b| a.id.cmp(&b.id));
        out
    }

    pub fn ancestors(&self, id: &str) -> Vec<String> {
        walk_blockers(&self.load(), id, Vec::new())
    }

    pub fn impact(&self, id: &str) -> Vec<String> {
        self.load()
            .nodes
            .values()
            .filter(|n| n.kind == Kind::Ticket && n.blocked_by.iter().any(|b| b == id))
            .map(|n| n.id.clone())
            .collect()
    }

    pub fn related(&self, id: &str) -> Vec<Related> {
        let state = self.load();
        let ticket = match state.get(id) {
            Some(n) if n.kind == Kind::Ticket => n,
            _ => return Vec::new(),
        };
        let mine = plan_terms(ticket);
        let mut hits: Vec<Related> = state
            .nodes
            .values()
            .filter(|n| n.kind == Kind::Ticket && n.id != id)
            .filter_map(|other| {
                let lexical = jaccard(&mine, &plan_terms(other));
                let graph = related_graph(ticket, other);
                let score = lexical + graph;
                if score > 0.0 {
                    Some(Related {
                        id: other.id.clone(),
                        score,
                        summary: other.summary.clone(),
                    })
                } else {
                    None
                }
            })
            .collect();
        hits.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.id.cmp(&b.id))
        });
        hits
    }

    pub fn agenda(&self) -> Vec<Node> {
        let today = today();
        let mut out: Vec<Node> = self
            .load()
            .nodes
            .values()
            .filter(|n| {
                self.plan_visible(n) && n.status != Status::Done && dated_open(n, &today, 14)
            })
            .cloned()
            .collect();
        out.sort_by(|a, b| {
            a.deadline
                .clone()
                .unwrap_or_else(|| "9999".into())
                .cmp(&b.deadline.clone().unwrap_or_else(|| "9999".into()))
        });
        out
    }

    pub fn tree(&self, id: &str) -> PlanTree {
        build_tree(&self.load(), id)
    }

    pub fn cycles(&self) -> Vec<Vec<String>> {
        let state = self.load();
        let ids: Vec<String> = state
            .nodes
            .values()
            .filter(|n| n.kind == Kind::Ticket)
            .map(|n| n.id.clone())
            .collect();
        let mut acc = Vec::new();
        for id in ids {
            if let Some(cycle) = cycle_from(&state, &id, vec![id.clone()]) {
                if !acc.contains(&cycle) {
                    acc.push(cycle);
                }
            }
        }
        acc
    }

    pub fn backlinks(&self, id: &str) -> Vec<Node> {
        self.load()
            .nodes
            .values()
            .filter(|n| {
                n.kind == Kind::Ticket
                    && n.id != id
                    && (n.body.contains(id) || n.notes.iter().any(|note| note.contains(id)))
            })
            .cloned()
            .collect()
    }

    fn load(&self) -> Tree {
        let lock = store::lock_path(&self.root);
        let _ = store::acquire(&lock);
        let tree = Store::open(&self.root)
            .ok()
            .and_then(|s| s.load().ok())
            .unwrap_or_default();
        store::release(&lock);
        tree
    }

    pub fn verify(&self) -> Vec<String> {
        let state = self.load();
        let mut lines = Vec::new();
        for node in state.nodes.values().filter(|n| n.kind == Kind::Work) {
            for dep in &node.deps {
                if state.get(dep).is_none() {
                    lines.push(format!("missing {dep} <- {}", node.id));
                }
            }
        }
        for cycle in self.work_cycles() {
            lines.push(format!("cycle {}", cycle.join(" -> ")));
        }
        lines
    }

    fn work_cycles(&self) -> Vec<Vec<String>> {
        let state = self.load();
        let ids: Vec<String> = state
            .nodes
            .values()
            .filter(|n| n.kind == Kind::Work)
            .map(|n| n.id.clone())
            .collect();
        let mut acc = Vec::new();
        for id in ids {
            if let Some(cycle) = work_cycle_from(&state, &id, vec![id.clone()]) {
                if !acc.contains(&cycle) {
                    acc.push(cycle);
                }
            }
        }
        acc
    }

    pub fn deeds(&self) -> Vec<Node> {
        let mut out: Vec<Node> = self
            .load()
            .nodes
            .values()
            .filter(|n| n.kind == Kind::Deed && !n.tombstone)
            .cloned()
            .collect();
        out.sort_by(|a, b| a.id.cmp(&b.id));
        out
    }

    pub fn create_deed(
        &self,
        kind: crate::node::DeedKind,
        summary: &str,
        extra: BTreeMap<String, String>,
        file: Option<&Path>,
    ) -> Result<Node> {
        let body = match file {
            Some(path) => std::fs::read(path).map_err(|_| Error::MissingBytes)?,
            None => extra
                .get("excerpt")
                .cloned()
                .or_else(|| extra.get("body").cloned())
                .unwrap_or_default()
                .into_bytes(),
        };
        self.transact(|state| {
            let digest = hash(&body);
            let mut deed = Node::deed(id::mint("d"), summary.to_string(), digest.clone());
            deed.deed_kind = Some(kind.token().into());
            deed.face = Some(kind.face().token().into());
            deed.extra = extra;
            let bytes_path = self.root.join("bytes").join(&deed.id);
            let _ = std::fs::create_dir_all(self.root.join("bytes"));
            let _ = std::fs::write(&bytes_path, &body);
            state.digests.insert(digest.clone(), deed.id.clone());
            state.tip = Some(deed.id.clone());
            state.blobs.insert(deed.id.clone(), body);
            state.put(deed.clone());
            write_host(self, &deed);
            Ok(deed)
        })
    }

    pub fn export_plan(&self) -> String {
        self.plan_list()
            .into_iter()
            .filter_map(|t| serde_json::to_string(&t).ok())
            .collect::<Vec<_>>()
            .join("\n")
    }

    pub fn roadmap(&self) -> String {
        let tickets = self.plan_list();
        let open: Vec<_> = tickets
            .iter()
            .filter(|t| t.status != Status::Done)
            .collect();
        let closed: Vec<_> = tickets
            .iter()
            .filter(|t| t.status == Status::Done)
            .collect();
        let mut s = String::from("# Roadmap\n\n## Open\n");
        for t in open {
            s.push_str(&format!("- {} {}\n", t.id, t.summary));
        }
        s.push_str("\n## Closed\n");
        for t in closed {
            s.push_str(&format!("- {} {}\n", t.id, t.summary));
        }
        s
    }

    pub fn graph(&self) -> String {
        let mut s = String::from("digraph plan {\n");
        for t in self.plan_list() {
            for b in &t.blocked_by {
                s.push_str(&format!("  \"{b}\" -> \"{}\";\n", t.id));
            }
        }
        s.push_str("}\n");
        s
    }

    pub fn digest(&self) -> String {
        hash(self.export_plan().as_bytes())
    }

    pub fn stale(&self, days: i64) -> Vec<Node> {
        let now = crate::node::now();
        self.plan_list()
            .into_iter()
            .filter(|t| {
                t.status != Status::Done
                    && t.created
                        .as_deref()
                        .map(|c| older_than(c, &now, days))
                        .unwrap_or(false)
            })
            .collect()
    }

    pub fn cards(&self) -> String {
        crate::cards::dump(&self.tree)
    }

    fn transact<T>(&self, fun: impl FnOnce(&mut Tree) -> Result<T>) -> Result<T> {
        let lock = store::lock_path(&self.root);
        store::acquire(&lock).map_err(|_| Error::Conflict)?;
        let result = (|| {
            let store = Store::open(&self.root).map_err(|_| Error::NotFound)?;
            let mut state = store.load().map_err(|_| Error::NotFound)?;
            let value = fun(&mut state)?;
            store.save(&state).map_err(|_| Error::NotFound)?;
            drop(store);
            crate::search::reindex(&self.root, state.nodes.values().cloned())
                .map_err(|_| Error::NotFound)?;
            Ok(value)
        })();
        store::release(&lock);
        result
    }
}

fn data_home() -> PathBuf {
    if let Ok(v) = std::env::var("XDG_DATA_HOME") {
        if !v.is_empty() {
            return PathBuf::from(v);
        }
    }
    std::env::var("HOME")
        .ok()
        .filter(|s| !s.is_empty())
        .map(|h| PathBuf::from(h).join(".local/share"))
        .unwrap_or_else(|| PathBuf::from(".").join(".local/share"))
}

fn workspace() -> PathBuf {
    for key in ["GROK_WORKSPACE_ROOT", "CLAUDE_PROJECT_DIR"] {
        if let Ok(v) = std::env::var(key) {
            if !v.is_empty() {
                return PathBuf::from(v);
            }
        }
    }
    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

fn infer_tree(root: &Path) -> PathBuf {
    match root.file_name().and_then(|n| n.to_str()) {
        Some("desk" | ".saena") => root
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| root.to_path_buf()),
        _ => root.to_path_buf(),
    }
}

fn work_cycle_from(state: &Tree, id: &str, path: Vec<String>) -> Option<Vec<String>> {
    let node = state.get(id)?;
    for dep in &node.deps {
        if path.iter().any(|x| x == dep) {
            let mut cycle = path.clone();
            cycle.push(dep.clone());
            return Some(cycle);
        }
        let mut next = path.clone();
        next.push(dep.clone());
        if let Some(cycle) = work_cycle_from(state, dep, next) {
            return Some(cycle);
        }
    }
    None
}

fn older_than(created: &str, now: &str, days: i64) -> bool {
    let c = created.get(..10).unwrap_or(created);
    let n = now.get(..10).unwrap_or(now);
    match (chrono_days(c), chrono_days(n)) {
        (Ok(a), Ok(b)) => b - a >= days,
        _ => false,
    }
}

fn host_key(desk: &Desk) -> Option<Vec<u8>> {
    if let Ok(path) = std::env::var("SAENA_HOST_KEY") {
        if !path.is_empty() {
            return std::fs::read(path).ok();
        }
    }
    std::fs::read(desk.root.join("..").join("host.key")).ok()
}

fn host_check(desk: &Desk, node: &Node) -> Result<()> {
    let Some(key) = host_key(desk) else {
        return Ok(());
    };
    let sidecar = desk.root.join(format!("{}.host", node.id));
    let got = std::fs::read(&sidecar).map_err(|_| Error::Host)?;
    let want = host_mac(&key, node);
    if got != want {
        return Err(Error::Host);
    }
    Ok(())
}

fn host_mac(key: &[u8], node: &Node) -> Vec<u8> {
    use hmac::{Hmac, Mac};
    use sha2::Sha256;
    let mut mac = Hmac::<Sha256>::new_from_slice(key).expect("hmac key");
    let canon = format!(
        "{}|{}|{}|{}",
        node.id,
        node.sha256.as_deref().unwrap_or(""),
        node.produced_by.as_deref().unwrap_or(""),
        node.grants.join(",")
    );
    mac.update(canon.as_bytes());
    mac.finalize().into_bytes().to_vec()
}

fn write_host(desk: &Desk, node: &Node) {
    if let Some(key) = host_key(desk) {
        let _ = std::fs::write(
            desk.root.join(format!("{}.host", node.id)),
            host_mac(&key, node),
        );
    }
}

fn work_claimable(node: &Node, state: &Tree) -> bool {
    node.status == Status::Ready && !node.archived && deps_met(node, state)
}

fn deps_met(node: &Node, state: &Tree) -> bool {
    node.deps
        .iter()
        .all(|id| state.get(id).map(|n| n.status.terminal()).unwrap_or(false))
}

fn ticket_ready(ticket: &Node, state: &Tree) -> bool {
    if ticket.status.ticket_closed() {
        return false;
    }
    ticket.blocked_by.iter().all(|id| {
        state
            .get(id)
            .map(|n| n.status.ticket_closed())
            .unwrap_or(false)
    })
}

fn check_evidence(desk: &Desk, state: &Tree, id: &str, seen: &mut HashSet<String>) -> Result<()> {
    if seen.contains(id) {
        return Ok(());
    }
    let node = state.get(id).ok_or(Error::NotFound)?;
    if node.kind != Kind::Deed {
        return Err(Error::NotFound);
    }
    if node.tombstone {
        return Err(Error::Deleted);
    }
    let body = desk.product(id).ok_or(Error::MissingBytes)?;
    if hash(&body) != node.sha256.clone().unwrap_or_default() {
        return Err(Error::Digest);
    }
    host_check(desk, node)?;
    seen.insert(id.to_string());
    for src in &node.sources {
        check_evidence(desk, state, src, seen)?;
    }
    Ok(())
}

fn walk_trail(state: &Tree, id: &str, mut acc: Vec<String>) -> Vec<String> {
    if acc.iter().any(|x| x == id) {
        return acc;
    }
    match state.get(id) {
        Some(node) if node.kind == Kind::Deed => {
            for src in &node.sources {
                acc = walk_trail(state, src, acc);
            }
            acc.push(id.to_string());
            acc
        }
        _ => acc,
    }
}

fn walk_blockers(state: &Tree, id: &str, acc: Vec<String>) -> Vec<String> {
    match state.get(id) {
        Some(ticket) => ticket
            .blocked_by
            .iter()
            .fold(acc, |acc, blocker| add_blocker(state, blocker, acc)),
        None => acc,
    }
}

fn add_blocker(state: &Tree, blocker: &str, acc: Vec<String>) -> Vec<String> {
    if acc.iter().any(|x| x == blocker) {
        acc
    } else {
        let next = {
            let mut a = acc;
            a.push(blocker.to_string());
            a
        };
        walk_blockers(state, blocker, next)
    }
}

fn build_tree(state: &Tree, id: &str) -> PlanTree {
    let kids: Vec<PlanTree> = state
        .nodes
        .values()
        .filter(|n| n.kind == Kind::Ticket && n.parent.as_deref() == Some(id))
        .map(|n| build_tree(state, &n.id))
        .collect();
    let blocked_by = state
        .get(id)
        .map(|n| n.blocked_by.clone())
        .unwrap_or_default();
    PlanTree {
        id: id.to_string(),
        children: kids,
        blocked_by,
    }
}

fn cycle_from(state: &Tree, id: &str, path: Vec<String>) -> Option<Vec<String>> {
    let ticket = state.get(id)?;
    for blocker in &ticket.blocked_by {
        if path.iter().any(|x| x == blocker) {
            let mut cycle = path.clone();
            cycle.push(blocker.clone());
            return Some(cycle);
        }
        let mut next = path.clone();
        next.push(blocker.clone());
        if let Some(cycle) = cycle_from(state, blocker, next) {
            return Some(cycle);
        }
    }
    None
}

fn related_graph(ticket: &Node, other: &Node) -> f64 {
    if other.parent.as_deref() == Some(ticket.id.as_str())
        || ticket.parent.as_deref() == Some(other.id.as_str())
    {
        1.0
    } else if ticket.blocked_by.contains(&other.id) || other.blocked_by.contains(&ticket.id) {
        0.8
    } else {
        0.0
    }
}

fn plan_terms(ticket: &Node) -> HashSet<String> {
    let mut text = ticket.summary.clone();
    text.push(' ');
    text.push_str(&ticket.body);
    for note in &ticket.notes {
        text.push(' ');
        text.push_str(note);
    }
    text.to_ascii_lowercase()
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|w| w.len() >= 3 && !matches!(*w, "the" | "and" | "for" | "with" | "from"))
        .map(str::to_string)
        .collect()
}

fn jaccard(a: &HashSet<String>, b: &HashSet<String>) -> f64 {
    let uni = a.union(b).count();
    if uni == 0 {
        0.0
    } else {
        a.intersection(b).count() as f64 / uni as f64
    }
}

fn dated_open(ticket: &Node, today: &str, horizon: i64) -> bool {
    for iso in [&ticket.deadline, &ticket.scheduled].into_iter().flatten() {
        if let (Ok(t), Ok(d)) = (
            chrono_days(&iso[..iso.len().min(10)]),
            chrono_days(&today[..today.len().min(10)]),
        ) {
            if t - d <= horizon {
                return true;
            }
        }
    }
    false
}

fn chrono_days(iso: &str) -> std::result::Result<i64, ()> {
    if iso.len() < 10 {
        return Err(());
    }
    let y: i64 = iso[0..4].parse().map_err(|_| ())?;
    let m: i64 = iso[5..7].parse().map_err(|_| ())?;
    let d: i64 = iso[8..10].parse().map_err(|_| ())?;
    Ok(y * 366 + m * 31 + d)
}

fn take_snap(tree: &Path) -> BTreeMap<String, String> {
    list_files(tree)
        .into_iter()
        .filter_map(|path| {
            let rel = path.strip_prefix(tree).ok()?.to_string_lossy().into_owned();
            let body = std::fs::read(&path).ok()?;
            Some((rel, hash(&body)))
        })
        .collect()
}

fn changed_files(tree: &Path, snap: &BTreeMap<String, String>) -> Vec<(String, Vec<u8>)> {
    let now = take_snap(tree);
    let mut files = Vec::new();
    for (rel, digest) in now {
        if snap.get(&rel) != Some(&digest) {
            if let Ok(body) = std::fs::read(tree.join(&rel)) {
                files.push((rel, body));
            }
        }
    }
    files.sort_by(|a, b| a.0.cmp(&b.0));
    files
}

fn list_files(tree: &Path) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(tree) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for entry in entries.flatten() {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if SKIP.contains(&name.as_ref()) {
            continue;
        }
        let path = entry.path();
        if path.is_file() {
            out.push(path);
        } else if path.is_dir() {
            out.extend(list_files(&path));
        }
    }
    out
}

fn encode_snap(files: &[(String, Vec<u8>)]) -> Vec<u8> {
    let mut out = b"SAENASNAP1\n".to_vec();
    for (rel, bin) in files {
        out.extend(bin.len().to_string().as_bytes());
        out.push(b' ');
        out.extend(rel.as_bytes());
        out.push(b'\n');
        out.extend(bin);
        out.push(b'\n');
    }
    out
}

fn decode_snap(body: &[u8]) -> BTreeMap<String, Vec<u8>> {
    const MAGIC: &[u8] = b"SAENASNAP1\n";
    if !body.starts_with(MAGIC) {
        let mut map = BTreeMap::new();
        map.insert("file".into(), body.to_vec());
        return map;
    }
    let mut rest = &body[MAGIC.len()..];
    let mut acc = BTreeMap::new();
    while !rest.is_empty() {
        let Some(nl) = rest.iter().position(|&b| b == b'\n') else {
            break;
        };
        let header = String::from_utf8_lossy(&rest[..nl]);
        let mut parts = header.splitn(2, ' ');
        let size: usize = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0);
        let rel = parts.next().unwrap_or("file").to_string();
        rest = &rest[nl + 1..];
        if rest.len() < size {
            break;
        }
        let bin = rest[..size].to_vec();
        rest = &rest[size..];
        if rest.first() == Some(&b'\n') {
            rest = &rest[1..];
        }
        acc.insert(rel, bin);
    }
    acc
}

pub fn hash(body: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(body);
    hex::encode(hasher.finalize())
}

pub fn whoami() -> String {
    std::env::var("SAENA_ACTOR")
        .ok()
        .filter(|s| !s.is_empty())
        .or_else(|| std::env::var("USER").ok())
        .unwrap_or_else(|| "agent".into())
}
