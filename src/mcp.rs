//! Model Context Protocol over the desk. Same verbs as `saena`.

use crate::desk::{whoami, Desk};
use crate::error::Error;
use crate::node::Status;
use serde_json::{json, Value};
use std::path::Path;

const PROTOCOL: &str = "2024-11-05";

const INSTRUCTIONS: &str = "\
saena is enabled for this session. Call `status` before you work.
Claim ready work, or `work` then claim. File work for another
project with `project`. Mint work from that ticket. `depend` so
this sitting waits. `wait` until the node is terminal, then
`get` the deed. When the work is done, `complete` — the host
snapshots what changed. Do not ask for SAENA_DESK or file paths.
The next sitting opens the tip deed.";

pub fn handle(msg: &Value, desk: Option<&Path>) -> Option<Value> {
    let method = msg.get("method")?.as_str()?;
    if method.starts_with("notifications/") {
        return None;
    }
    let id = msg.get("id")?.clone();
    let params = msg.get("params").cloned().unwrap_or_else(|| json!({}));
    Some(reply(id, dispatch(method, &params, desk)))
}

fn reply(id: Value, result: std::result::Result<Value, Error>) -> Value {
    match result {
        Ok(value) => json!({"jsonrpc": "2.0", "id": id, "result": value}),
        Err(err) => json!({
            "jsonrpc": "2.0",
            "id": id,
            "error": {"code": -32000, "message": err.token()}
        }),
    }
}

fn dispatch(
    method: &str,
    params: &Value,
    desk: Option<&Path>,
) -> std::result::Result<Value, Error> {
    match method {
        "initialize" => Ok(json!({
            "protocolVersion": PROTOCOL,
            "capabilities": {"tools": {}, "resources": {}},
            "serverInfo": {"name": "saena", "version": "0.1.0"},
            "instructions": INSTRUCTIONS.trim()
        })),
        "ping" => Ok(json!({})),
        "tools/list" => Ok(json!({"tools": tools()})),
        "resources/list" => Ok(json!({"resources": resources()})),
        "resources/read" => {
            let uri = params.get("uri").and_then(Value::as_str).unwrap_or("");
            let text = resource_text(uri, desk)?;
            Ok(json!({"contents": [{"uri": uri, "mimeType": "text/plain", "text": text}]}))
        }
        "tools/call" => {
            let name = params.get("name").and_then(Value::as_str).unwrap_or("");
            let args = params
                .get("arguments")
                .cloned()
                .unwrap_or_else(|| json!({}));
            let _ = Desk::for_session(desk);
            call_tool(name, &args, desk)
        }
        _ => Err(Error::Method),
    }
}

fn call_tool(name: &str, args: &Value, desk: Option<&Path>) -> std::result::Result<Value, Error> {
    match invoke(name, args, desk) {
        Ok(text) => Ok(content(&text, false)),
        Err(err) => Ok(content(&format!("error {err}"), true)),
    }
}

fn invoke(name: &str, args: &Value, desk: Option<&Path>) -> std::result::Result<String, Error> {
    match name {
        "status" => {
            let d = Desk::for_session(desk)?;
            Ok(d.format_status())
        }
        "open" => {
            let opened = Desk::for_session(desk)?;
            Ok(format!("open {}", opened.root.display()))
        }
        "work" => {
            let summary = need(args, "summary")?;
            let mut d = Desk::for_session(desk)?;
            if let Some(name) = args.get("project").and_then(Value::as_str) {
                if !name.is_empty() {
                    d.project = Some(name.to_string());
                }
            }
            let work = d.upsert(&summary)?;
            Ok(format!("{}  ready  {}", work.id, work.summary))
        }
        "mint" => {
            let d = Desk::for_session(desk)?;
            let work = d.work_from_plan(&need(args, "id")?)?;
            Ok(format!("{}  ready  {}", work.id, work.summary))
        }
        "depend" => {
            let d = Desk::for_session(desk)?;
            let node = d.depend(&need(args, "id")?, &need(args, "other")?)?;
            Ok(format!("{}  deps {}", node.id, node.deps.join(",")))
        }
        "wait" => {
            let timeout = args.get("timeout_ms").and_then(Value::as_u64).or_else(|| {
                args.get("timeout_ms")
                    .and_then(Value::as_str)
                    .and_then(|s| s.parse().ok())
            });
            let d = Desk::for_session(desk)?;
            let node = d.wait(&need(args, "id")?, timeout)?;
            match node.deed.as_deref() {
                Some(deed) => Ok(format!("{}  {}  deed {deed}", node.id, node.status.token())),
                None => Ok(format!("{}  {}", node.id, node.status.token())),
            }
        }
        "claim" => {
            let id = need(args, "id")?;
            let actor = args
                .get("actor")
                .and_then(Value::as_str)
                .map(str::to_string)
                .filter(|s| !s.is_empty())
                .unwrap_or_else(whoami);
            let d = Desk::for_session(desk)?;
            let work = d.claim(&id, &actor, None)?;
            Ok(format!(
                "{}  claimed  {}  {}",
                work.id,
                work.assignee.as_deref().unwrap_or(""),
                work.summary
            ))
        }
        "spawn" => {
            let parent = need(args, "parent")?;
            let summary = need(args, "summary")?;
            let d = Desk::for_session(desk)?;
            let child = d.spawn(&parent, &summary)?;
            Ok(format!("{}  ready  {}", child.id, child.summary))
        }
        "complete" => {
            let id = need(args, "id")?;
            let file = args
                .get("file")
                .and_then(Value::as_str)
                .map(std::path::PathBuf::from);
            let source = args
                .get("source")
                .and_then(Value::as_str)
                .map(|s| vec![s.to_string()])
                .unwrap_or_default();
            let d = Desk::for_session(desk)?;
            let done = d.complete(&id, file.as_deref(), source, None, None, Status::Done)?;
            match done.deed {
                None => Ok(format!("done  {}", done.work.id)),
                Some(deed) => Ok(format!("done  {}  deed {}", done.work.id, deed.id)),
            }
        }
        "get" => {
            let id = need(args, "id")?;
            let d = Desk::for_session(desk)?;
            let deed = d.deed(&id)?;
            let mut body = format!("{}  {}\n", deed.id, deed.summary);
            for (_rel, bytes) in d.files(&id) {
                body.push_str(&String::from_utf8_lossy(&bytes));
            }
            Ok(body)
        }
        "evidence" => {
            let id = need(args, "id")?;
            let d = Desk::for_session(desk)?;
            d.evidence(&id)?;
            Ok(format!("ok  {id}"))
        }
        "trail" => {
            let id = need(args, "id")?;
            let d = Desk::for_session(desk)?;
            Ok(d.trail(&id).join("\n"))
        }
        "remember" => {
            let text = need(args, "text")?;
            let d = Desk::for_session(desk)?;
            let atom = d.remember(&text)?;
            Ok(format!("{}  {}", atom.id, atom.text))
        }
        "plan" => {
            let summary = need(args, "summary")?;
            let parent = args
                .get("parent")
                .and_then(Value::as_str)
                .filter(|s| !s.is_empty())
                .map(str::to_string);
            let mut d = Desk::for_session(desk)?;
            if let Some(name) = args.get("project").and_then(Value::as_str) {
                if !name.is_empty() {
                    d.project = Some(name.to_string());
                }
            }
            let ticket = d.plan(&summary, parent, None, None, None, vec![], None, None)?;
            Ok(format!(
                "{}  {}  {}",
                ticket.id,
                ticket.status.token(),
                ticket.summary
            ))
        }
        "verify" => {
            let d = Desk::for_session(desk)?;
            let lines = d.verify();
            if lines.is_empty() {
                Ok("ok".into())
            } else {
                Ok(lines.join("\n"))
            }
        }
        "pin" => {
            let name = need(args, "name")?;
            let d = Desk::for_session(desk)?;
            Ok(format!("pin {}", d.pin(&name)?))
        }
        "deed" => {
            let kind = crate::node::DeedKind::parse(&need(args, "kind")?)?;
            let summary = need(args, "summary")?;
            let d = Desk::for_session(desk)?;
            let deed = d.create_deed(kind, &summary, Default::default(), None)?;
            Ok(format!("{}  {}  {}", deed.id, kind.token(), deed.summary))
        }
        "ready" => {
            let d = Desk::for_session(desk)?;
            Ok(d.ready()
                .into_iter()
                .map(|w| format!("{}  ready  {}", w.id, w.summary))
                .collect::<Vec<_>>()
                .join("\n"))
        }
        "list" => {
            let d = Desk::for_session(desk)?;
            let filter = args.get("filter").and_then(Value::as_str).unwrap_or("");
            if filter == "deeds" {
                return Ok(d
                    .deeds()
                    .into_iter()
                    .map(|deed| {
                        format!(
                            "{}  {}  {}",
                            deed.id,
                            deed.deed_kind.as_deref().unwrap_or("file"),
                            deed.summary
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("\n"));
            }
            let all = filter == "all";
            let archived = filter == "archived";
            let terminal = filter == "terminal";
            Ok(d.work_list(archived, all, terminal)
                .into_iter()
                .map(|w| format!("{}  {}  {}", w.id, w.status.token(), w.summary))
                .collect::<Vec<_>>()
                .join("\n"))
        }
        "link" => {
            let d = Desk::for_session(desk)?;
            let node = d.link(&need(args, "parent")?, &need(args, "child")?)?;
            Ok(format!("{}  deps {}", node.id, node.deps.join(",")))
        }
        "unlink" => {
            let d = Desk::for_session(desk)?;
            let node = d.unlink(&need(args, "parent")?, &need(args, "child")?)?;
            Ok(format!("{}  unlinked", node.id))
        }
        "archive" => {
            let d = Desk::for_session(desk)?;
            Ok(format!("{}  archived", d.archive(&need(args, "id")?)?.id))
        }
        "unarchive" => {
            let d = Desk::for_session(desk)?;
            Ok(format!(
                "{}  unarchived",
                d.unarchive(&need(args, "id")?)?.id
            ))
        }
        "current" => {
            let d = Desk::for_session(desk)?;
            let deed = d.current(&need(args, "id")?)?;
            Ok(format!("{}  {}", deed.id, deed.summary))
        }
        "leave" => {
            let d = Desk::for_session(desk)?;
            let dest = need(args, "dest")?;
            let dir = d.leave(&need(args, "id")?, Path::new(&dest))?;
            Ok(format!("leave {}", dir.display()))
        }
        "timestamp" => {
            let d = Desk::for_session(desk)?;
            Ok(format!(
                "timestamp {}",
                d.timestamp(&need(args, "id")?)?.display()
            ))
        }
        "prefer" => {
            let d = Desk::for_session(desk)?;
            let atom = d.prefer(&need(args, "text")?)?;
            Ok(format!(
                "{}  {}  {}",
                atom.id,
                atom.atom_kind.as_deref().unwrap_or("preference"),
                atom.text
            ))
        }
        "block" => {
            let d = Desk::for_session(desk)?;
            let ticket = d.block(&need(args, "id")?, &need(args, "blocker")?)?;
            Ok(format!(
                "{}  blocked  {}",
                ticket.id,
                ticket.blocked_by.join(",")
            ))
        }
        "close" => {
            let d = Desk::for_session(desk)?;
            let ticket = d.close(&need(args, "id")?)?;
            Ok(format!("{}  done  {}", ticket.id, ticket.summary))
        }
        "note" => {
            let d = Desk::for_session(desk)?;
            Ok(format!(
                "{}  note",
                d.note(&need(args, "id")?, &need(args, "text")?)?.id
            ))
        }
        "append" => {
            let d = Desk::for_session(desk)?;
            Ok(format!(
                "{}  append",
                d.append(&need(args, "id")?, &need(args, "text")?)?.id
            ))
        }
        "reject" => {
            let d = Desk::for_session(desk)?;
            let ticket = d.reject(&need(args, "id")?, &need(args, "dest")?)?;
            Ok(format!(
                "{}  rejected  {}",
                ticket.id,
                ticket.replacement.as_deref().unwrap_or("")
            ))
        }
        "resolve" => {
            let d = Desk::for_session(desk)?;
            let ticket = d.resolve(&need(args, "id")?, &need(args, "dest")?)?;
            Ok(format!(
                "{}  resolved  {}",
                ticket.id,
                ticket.replacement.as_deref().unwrap_or("")
            ))
        }
        "related" => {
            let d = Desk::for_session(desk)?;
            Ok(d.related(&need(args, "id")?)
                .into_iter()
                .map(|h| format!("{}  {}  {}", h.id, h.score, h.summary))
                .collect::<Vec<_>>()
                .join("\n"))
        }
        "agenda" => {
            let d = Desk::for_session(desk)?;
            Ok(d.agenda()
                .into_iter()
                .map(|t| {
                    format!(
                        "{}  {}  {}",
                        t.id,
                        t.deadline
                            .clone()
                            .or(t.scheduled.clone())
                            .unwrap_or_default(),
                        t.summary
                    )
                })
                .collect::<Vec<_>>()
                .join("\n"))
        }
        "children" => {
            let d = Desk::for_session(desk)?;
            Ok(d.children(&need(args, "id")?)
                .into_iter()
                .map(|t| format!("{}  {}", t.id, t.summary))
                .collect::<Vec<_>>()
                .join("\n"))
        }
        "ancestors" => {
            let d = Desk::for_session(desk)?;
            Ok(d.ancestors(&need(args, "id")?).join("\n"))
        }
        "impact" => {
            let d = Desk::for_session(desk)?;
            Ok(d.impact(&need(args, "id")?).join("\n"))
        }
        "tree" => {
            let d = Desk::for_session(desk)?;
            Ok(d.tree(&need(args, "id")?).id)
        }
        "cycles" => {
            let d = Desk::for_session(desk)?;
            Ok(d.cycles()
                .into_iter()
                .map(|c| c.join(" -> "))
                .collect::<Vec<_>>()
                .join("\n"))
        }
        "search" => {
            let d = Desk::for_session(desk)?;
            Ok(d.search(&need(args, "query")?)
                .into_iter()
                .map(|h| format!("{}  {}  {}  {}", h.id, h.record, h.field, h.text))
                .collect::<Vec<_>>()
                .join("\n"))
        }
        "unpin" => {
            let d = Desk::for_session(desk)?;
            d.unpin()?;
            Ok("unpin".into())
        }
        "export" => {
            let d = Desk::for_session(desk)?;
            Ok(d.export_plan())
        }
        "roadmap" => {
            let d = Desk::for_session(desk)?;
            Ok(d.roadmap())
        }
        "graph" => {
            let d = Desk::for_session(desk)?;
            Ok(d.graph())
        }
        "digest" => {
            let d = Desk::for_session(desk)?;
            Ok(d.digest())
        }
        "stale" => {
            let days: i64 = need(args, "days")?.parse().map_err(|_| Error::Usage)?;
            let d = Desk::for_session(desk)?;
            Ok(d.stale(days)
                .into_iter()
                .map(|t| {
                    format!(
                        "{}  {}  {}",
                        t.id,
                        t.created.as_deref().unwrap_or(""),
                        t.summary
                    )
                })
                .collect::<Vec<_>>()
                .join("\n"))
        }
        "cards" => {
            let d = Desk::for_session(desk)?;
            Ok(d.cards())
        }
        "sit" => {
            let d = Desk::for_session(desk)?;
            Ok(crate::sit::dump(&d))
        }
        _ => Err(Error::Method),
    }
}

fn need(args: &Value, key: &str) -> std::result::Result<String, Error> {
    args.get(key)
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .ok_or(Error::Usage)
}

fn content(text: &str, is_error: bool) -> Value {
    json!({"content": [{"type": "text", "text": text}], "isError": is_error})
}

fn resources() -> Value {
    json!([{"uri": "saena://status", "name": "status", "mimeType": "text/plain"}])
}

fn resource_text(uri: &str, desk: Option<&Path>) -> std::result::Result<String, Error> {
    match uri {
        "saena://status" => {
            let d = Desk::for_session(desk)?;
            Ok(d.format_status())
        }
        _ => Err(Error::NotFound),
    }
}

fn tools() -> Value {
    json!([
        tool(
            "status",
            "Sitting glance",
            json!({"type": "object", "properties": {}})
        ),
        tool(
            "open",
            "Open the desk",
            json!({"type": "object", "properties": {}})
        ),
        tool(
            "work",
            "Put ready work on the frontier",
            req(
                &["summary"],
                json!({"summary": {"type": "string"}, "project": {"type": "string"}})
            )
        ),
        tool(
            "mint",
            "Mint work from a ticket",
            req(&["id"], json!({"id": {"type": "string"}}))
        ),
        tool(
            "depend",
            "This work waits on other",
            req(
                &["id", "other"],
                json!({"id": {"type": "string"}, "other": {"type": "string"}})
            )
        ),
        tool(
            "wait",
            "Sleep until a node is terminal",
            req(
                &["id"],
                json!({"id": {"type": "string"}, "timeout_ms": {"type": "number"}})
            )
        ),
        tool(
            "claim",
            "Lock a ready node",
            req(
                &["id"],
                json!({"id": {"type": "string"}, "actor": {"type": "string"}})
            )
        ),
        tool(
            "spawn",
            "Insert a child work node",
            req(
                &["parent", "summary"],
                json!({"parent": {"type": "string"}, "summary": {"type": "string"}})
            )
        ),
        tool(
            "complete",
            "Finish work; host snapshots changes",
            req(
                &["id"],
                json!({"id": {"type": "string"}, "file": {"type": "string"}, "source": {"type": "string"}})
            )
        ),
        tool(
            "get",
            "Open a frozen deed",
            req(&["id"], json!({"id": {"type": "string"}}))
        ),
        tool(
            "evidence",
            "Check the issued product",
            req(&["id"], json!({"id": {"type": "string"}}))
        ),
        tool(
            "trail",
            "Walk sources",
            req(&["id"], json!({"id": {"type": "string"}}))
        ),
        tool(
            "remember",
            "Commit a memory atom",
            req(&["text"], json!({"text": {"type": "string"}}))
        ),
        tool(
            "plan",
            "Durable ticket",
            req(
                &["summary"],
                json!({"summary": {"type": "string"}, "parent": {"type": "string"}, "project": {"type": "string"}})
            )
        ),
        tool(
            "ready",
            "Claimable work",
            json!({"type": "object", "properties": {}})
        ),
        tool(
            "list",
            "Work or deed nodes",
            json!({"type": "object", "properties": {"filter": {"type": "string"}}})
        ),
        tool(
            "verify",
            "Check the work graph",
            json!({"type": "object", "properties": {}})
        ),
        tool(
            "pin",
            "Scope remember and search",
            req(&["name"], json!({"name": {"type": "string"}}))
        ),
        tool(
            "deed",
            "Create a typed deed",
            req(
                &["kind", "summary"],
                json!({"kind": {"type": "string"}, "summary": {"type": "string"}})
            )
        ),
        tool(
            "link",
            "Hard dependency",
            req(
                &["parent", "child"],
                json!({"parent": {"type": "string"}, "child": {"type": "string"}})
            )
        ),
        tool(
            "unlink",
            "Drop a dependency",
            req(
                &["parent", "child"],
                json!({"parent": {"type": "string"}, "child": {"type": "string"}})
            )
        ),
        tool(
            "archive",
            "Hide a terminal node",
            req(&["id"], json!({"id": {"type": "string"}}))
        ),
        tool(
            "unarchive",
            "Show a terminal node",
            req(&["id"], json!({"id": {"type": "string"}}))
        ),
        tool(
            "current",
            "Follow supersedes to the tip",
            req(&["id"], json!({"id": {"type": "string"}}))
        ),
        tool(
            "leave",
            "Copy product bytes",
            req(
                &["id", "dest"],
                json!({"id": {"type": "string"}, "dest": {"type": "string"}})
            )
        ),
        tool(
            "timestamp",
            "RFC 3161 request",
            req(&["id"], json!({"id": {"type": "string"}}))
        ),
        tool(
            "prefer",
            "Commit a preference atom",
            req(&["text"], json!({"text": {"type": "string"}}))
        ),
        tool(
            "block",
            "Ticket waits on other",
            req(
                &["id", "blocker"],
                json!({"id": {"type": "string"}, "blocker": {"type": "string"}})
            )
        ),
        tool(
            "close",
            "Close a ticket",
            req(&["id"], json!({"id": {"type": "string"}}))
        ),
        tool(
            "note",
            "Dated note",
            req(
                &["id", "text"],
                json!({"id": {"type": "string"}, "text": {"type": "string"}})
            )
        ),
        tool(
            "append",
            "Dated report",
            req(
                &["id", "text"],
                json!({"id": {"type": "string"}, "text": {"type": "string"}})
            )
        ),
        tool(
            "reject",
            "Reject toward dest",
            req(
                &["id", "dest"],
                json!({"id": {"type": "string"}, "dest": {"type": "string"}})
            )
        ),
        tool(
            "resolve",
            "Resolve toward dest",
            req(
                &["id", "dest"],
                json!({"id": {"type": "string"}, "dest": {"type": "string"}})
            )
        ),
        tool(
            "related",
            "Nearby tickets",
            req(&["id"], json!({"id": {"type": "string"}}))
        ),
        tool(
            "agenda",
            "Dated open tickets",
            json!({"type": "object", "properties": {}})
        ),
        tool(
            "children",
            "Tickets under parent",
            req(&["id"], json!({"id": {"type": "string"}}))
        ),
        tool(
            "ancestors",
            "Blockers walked up",
            req(&["id"], json!({"id": {"type": "string"}}))
        ),
        tool(
            "impact",
            "Tickets waiting on this",
            req(&["id"], json!({"id": {"type": "string"}}))
        ),
        tool(
            "tree",
            "Parent tree",
            req(&["id"], json!({"id": {"type": "string"}}))
        ),
        tool(
            "cycles",
            "Blocked-by cycles",
            json!({"type": "object", "properties": {}})
        ),
        tool(
            "search",
            "Search the tree",
            req(&["query"], json!({"query": {"type": "string"}}))
        ),
        tool(
            "unpin",
            "Clear the pin",
            json!({"type": "object", "properties": {}})
        ),
        tool(
            "export",
            "Tickets as JSON lines",
            json!({"type": "object", "properties": {}})
        ),
        tool(
            "roadmap",
            "Open then closed tickets",
            json!({"type": "object", "properties": {}})
        ),
        tool(
            "graph",
            "Blocked-by edges",
            json!({"type": "object", "properties": {}})
        ),
        tool(
            "digest",
            "Stable ticket digest",
            json!({"type": "object", "properties": {}})
        ),
        tool(
            "stale",
            "Open tickets older than N days",
            req(&["days"], json!({"days": {"type": "string"}}))
        ),
        tool(
            "cards",
            "Human USER.md and MEMORY.md",
            json!({"type": "object", "properties": {}})
        ),
        tool(
            "sit",
            "Terminal desk dump",
            json!({"type": "object", "properties": {}})
        ),
    ])
}

fn tool(name: &str, description: &str, schema: Value) -> Value {
    json!({"name": name, "description": description, "inputSchema": schema})
}

fn req(required: &[&str], properties: Value) -> Value {
    json!({"type": "object", "properties": properties, "required": required})
}
