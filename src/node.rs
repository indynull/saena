//! One node on the desk tree.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    Work,
    Deed,
    Ticket,
    Atom,
}

impl Kind {
    pub fn token(self) -> &'static str {
        match self {
            Self::Work => "work",
            Self::Deed => "deed",
            Self::Ticket => "ticket",
            Self::Atom => "atom",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Ready,
    Claimed,
    Done,
    Failed,
    Cancelled,
    Todo,
    Blocked,
    Rejected,
    Resolved,
    Frozen,
}

impl Status {
    pub fn token(self) -> &'static str {
        match self {
            Self::Ready => "ready",
            Self::Claimed => "claimed",
            Self::Done => "done",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
            Self::Todo => "todo",
            Self::Blocked => "blocked",
            Self::Rejected => "rejected",
            Self::Resolved => "resolved",
            Self::Frozen => "frozen",
        }
    }

    pub fn terminal(self) -> bool {
        matches!(self, Self::Done | Self::Failed | Self::Cancelled)
    }

    pub fn ticket_closed(self) -> bool {
        matches!(self, Self::Done | Self::Rejected | Self::Resolved)
    }

    pub fn parse(raw: &str) -> crate::error::Result<Self> {
        match raw {
            "ready" => Ok(Self::Ready),
            "claimed" => Ok(Self::Claimed),
            "done" => Ok(Self::Done),
            "failed" => Ok(Self::Failed),
            "cancelled" => Ok(Self::Cancelled),
            "todo" => Ok(Self::Todo),
            "blocked" => Ok(Self::Blocked),
            "rejected" => Ok(Self::Rejected),
            "resolved" => Ok(Self::Resolved),
            "frozen" => Ok(Self::Frozen),
            _ => Err(crate::error::Error::Usage),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkKind {
    #[default]
    Task,
}

impl WorkKind {
    pub fn parse(raw: &str) -> crate::error::Result<Self> {
        match raw {
            "task" => Ok(Self::Task),
            _ => Err(crate::error::Error::Usage),
        }
    }

    pub fn token(self) -> &'static str {
        "task"
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    #[default]
    Unset,
    Agent,
    Human,
    Host,
}

impl Role {
    pub fn parse(raw: &str) -> crate::error::Result<Self> {
        match raw {
            "unset" => Ok(Self::Unset),
            "agent" => Ok(Self::Agent),
            "human" => Ok(Self::Human),
            "host" => Ok(Self::Host),
            _ => Err(crate::error::Error::Usage),
        }
    }

    pub fn token(self) -> &'static str {
        match self {
            Self::Unset => "unset",
            Self::Agent => "agent",
            Self::Human => "human",
            Self::Host => "host",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DeedKind {
    #[default]
    File,
    Set,
    Quote,
    Patch,
    MailDraft,
    Clip,
    Page,
    Form,
    Table,
    Procedure,
    Event,
}

impl DeedKind {
    pub fn parse(raw: &str) -> crate::error::Result<Self> {
        Self::known(raw).ok_or(crate::error::Error::Usage)
    }

    pub fn known(raw: &str) -> Option<Self> {
        match raw {
            "file" => Some(Self::File),
            "set" => Some(Self::Set),
            "quote" => Some(Self::Quote),
            "patch" => Some(Self::Patch),
            "mailDraft" | "mail_draft" => Some(Self::MailDraft),
            "clip" => Some(Self::Clip),
            "page" => Some(Self::Page),
            "form" => Some(Self::Form),
            "table" => Some(Self::Table),
            "procedure" => Some(Self::Procedure),
            "event" => Some(Self::Event),
            _ => None,
        }
    }

    pub fn token(self) -> &'static str {
        match self {
            Self::File => "file",
            Self::Set => "set",
            Self::Quote => "quote",
            Self::Patch => "patch",
            Self::MailDraft => "mailDraft",
            Self::Clip => "clip",
            Self::Page => "page",
            Self::Form => "form",
            Self::Table => "table",
            Self::Procedure => "procedure",
            Self::Event => "event",
        }
    }

    pub fn face(self) -> Face {
        match self {
            Self::Clip => Face::Waveform,
            Self::Quote | Self::Patch => Face::Syntax,
            Self::Table | Self::Form => Face::Sheet,
            Self::MailDraft => Face::Letter,
            _ => Face::Document,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Face {
    #[default]
    Document,
    Syntax,
    Sheet,
    Letter,
    Waveform,
}

impl Face {
    pub fn token(self) -> &'static str {
        match self {
            Self::Document => "document",
            Self::Syntax => "syntax",
            Self::Sheet => "sheet",
            Self::Letter => "letter",
            Self::Waveform => "waveform",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Node {
    pub id: String,
    pub kind: Kind,
    pub status: Status,
    #[serde(default)]
    pub summary: String,
    #[serde(default)]
    pub text: String,
    #[serde(default)]
    pub gen: u64,
    #[serde(default)]
    pub assignee: Option<String>,
    #[serde(default)]
    pub parent: Option<String>,
    #[serde(default)]
    pub children: Vec<String>,
    #[serde(default)]
    pub deps: Vec<String>,
    #[serde(default)]
    pub succs: Vec<String>,
    #[serde(default)]
    pub blocked_by: Vec<String>,
    #[serde(default)]
    pub sources: Vec<String>,
    #[serde(default)]
    pub ticket: Option<String>,
    #[serde(default)]
    pub deed: Option<String>,
    #[serde(default)]
    pub work: Option<String>,
    #[serde(default)]
    pub sha256: Option<String>,
    #[serde(default)]
    pub supersedes: Option<String>,
    #[serde(default)]
    pub successor: Option<String>,
    #[serde(default)]
    pub produced_by: Option<String>,
    #[serde(default)]
    pub archived: bool,
    #[serde(default)]
    pub tombstone: bool,
    #[serde(default)]
    pub notes: Vec<String>,
    #[serde(default)]
    pub body: String,
    #[serde(default)]
    pub replacement: Option<String>,
    #[serde(default)]
    pub deadline: Option<String>,
    #[serde(default)]
    pub scheduled: Option<String>,
    #[serde(default)]
    pub created: Option<String>,
    #[serde(default)]
    pub ts: Option<String>,
    #[serde(default)]
    pub field: Option<String>,
    #[serde(default)]
    pub atom_kind: Option<String>,
    #[serde(default)]
    pub deed_kind: Option<String>,
    #[serde(default)]
    pub work_kind: Option<String>,
    #[serde(default)]
    pub role: Option<String>,
    #[serde(default)]
    pub pin: Option<String>,
    #[serde(default)]
    pub ticket_type: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub priority: Option<String>,
    #[serde(default)]
    pub face: Option<String>,
    #[serde(default)]
    pub grants: Vec<String>,
    #[serde(default)]
    pub paths: Vec<String>,
    #[serde(default)]
    pub cite: Option<String>,
    #[serde(default)]
    pub extra: BTreeMap<String, String>,
    #[serde(default)]
    pub snap: BTreeMap<String, String>,
    #[serde(default)]
    pub project: Option<String>,
}

impl Node {
    pub fn work(id: String, summary: String) -> Self {
        Self {
            id,
            kind: Kind::Work,
            status: Status::Ready,
            summary,
            ..Self::empty()
        }
    }

    pub fn ticket(id: String, summary: String) -> Self {
        Self {
            id,
            kind: Kind::Ticket,
            status: Status::Todo,
            summary,
            created: Some(now()),
            ..Self::empty()
        }
    }

    pub fn deed(id: String, summary: String, digest: String) -> Self {
        Self {
            id,
            kind: Kind::Deed,
            status: Status::Frozen,
            summary,
            sha256: Some(digest),
            deed_kind: Some(DeedKind::File.token().into()),
            face: Some(DeedKind::File.face().token().into()),
            ..Self::empty()
        }
    }

    pub fn atom(id: String, kind: &str, text: String) -> Self {
        Self {
            id,
            kind: Kind::Atom,
            status: Status::Ready,
            summary: text.clone(),
            text,
            atom_kind: Some(kind.into()),
            field: Some("atom".into()),
            ts: Some(now()),
            ..Self::empty()
        }
    }

    fn empty() -> Self {
        Self {
            id: String::new(),
            kind: Kind::Work,
            status: Status::Ready,
            summary: String::new(),
            text: String::new(),
            gen: 0,
            assignee: None,
            parent: None,
            children: Vec::new(),
            deps: Vec::new(),
            succs: Vec::new(),
            blocked_by: Vec::new(),
            sources: Vec::new(),
            ticket: None,
            deed: None,
            work: None,
            sha256: None,
            supersedes: None,
            successor: None,
            produced_by: None,
            archived: false,
            tombstone: false,
            notes: Vec::new(),
            body: String::new(),
            replacement: None,
            deadline: None,
            scheduled: None,
            created: None,
            ts: None,
            field: None,
            atom_kind: None,
            deed_kind: None,
            work_kind: Some(WorkKind::Task.token().into()),
            role: Some(Role::Unset.token().into()),
            pin: None,
            ticket_type: None,
            tags: Vec::new(),
            priority: None,
            face: None,
            grants: Vec::new(),
            paths: Vec::new(),
            cite: None,
            extra: BTreeMap::new(),
            snap: BTreeMap::new(),
            project: None,
        }
    }
}

pub fn now() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    // RFC 3339-ish without extra crates. Tests compare date prefixes via Date.
    let days = secs / 86_400;
    let (y, m, d) = civil_from_days(days as i64);
    let rem = secs % 86_400;
    let hh = rem / 3600;
    let mm = (rem % 3600) / 60;
    let ss = rem % 60;
    format!("{y:04}-{m:02}-{d:02}T{hh:02}:{mm:02}:{ss:02}Z")
}

pub fn today() -> String {
    now()[..10].to_string()
}

fn civil_from_days(z: i64) -> (i32, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y as i32, m as u32, d as u32)
}
