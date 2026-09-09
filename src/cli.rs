//! `saena` command. One desk. The verbs a sitting uses.

use crate::desk::{whoami, Desk};
use crate::error::{Error, Result};
use crate::init;
use crate::install;
use crate::mcp;
use crate::node::Status;
use crate::sit;
use std::io::{self, BufRead, Write};
use std::path::{Path, PathBuf};

const VERBS: &[&str] = &[
    "open",
    "work",
    "claim",
    "spawn",
    "complete",
    "link",
    "unlink",
    "archive",
    "unarchive",
    "get",
    "evidence",
    "trail",
    "current",
    "leave",
    "timestamp",
    "remember",
    "prefer",
    "plan",
    "block",
    "close",
    "note",
    "append",
    "reject",
    "resolve",
    "related",
    "agenda",
    "children",
    "ancestors",
    "impact",
    "tree",
    "cycles",
    "search",
    "ready",
    "list",
    "verify",
    "pin",
    "unpin",
    "deed",
    "export",
    "roadmap",
    "graph",
    "digest",
    "stale",
    "cards",
    "status",
    "sit",
    "mcp",
    "install",
    "init",
    "surface",
    "wait",
    "mint",
    "depend",
];

pub fn main(argv: &[String]) -> Result<()> {
    match run(argv) {
        Ok(()) => Ok(()),
        Err(err) => {
            println!("error {err}");
            Err(err)
        }
    }
}

pub fn run(argv: &[String]) -> Result<()> {
    let (rest, desk) = strip_desk(argv);
    dispatch(&rest, desk.as_deref())
}

fn strip_desk(argv: &[String]) -> (Vec<String>, Option<PathBuf>) {
    let mut rest = Vec::new();
    let mut desk = None;
    let mut i = 0;
    while i < argv.len() {
        if argv[i] == "--desk" && i + 1 < argv.len() {
            desk = Some(PathBuf::from(&argv[i + 1]));
            i += 2;
            continue;
        }
        rest.push(argv[i].clone());
        i += 1;
    }
    (rest, desk)
}

fn sitting(desk: Option<&Path>) -> Result<Desk> {
    Desk::for_session(desk)
}

fn dispatch(argv: &[String], desk: Option<&Path>) -> Result<()> {
    match argv {
        [] => surface(),
        [cmd] if cmd == "surface" => surface(),
        [cmd] if cmd == "open" => do_open(desk),
        [cmd] if cmd == "status" => do_status(desk),
        [cmd, flags @ ..] if cmd == "sit" => do_sit(desk, flags),
        [cmd, words @ ..] if cmd == "search" => do_search(desk, &words.join(" ")),
        [cmd] if cmd == "mcp" => do_mcp(desk),
        [cmd, id] if cmd == "wait" => do_wait(desk, id, None),
        [cmd, id, flag, ms] if cmd == "wait" && flag == "--timeout-ms" => {
            do_wait(desk, id, Some(ms.parse().map_err(|_| Error::Usage)?))
        }
        [cmd, flags @ ..] if cmd == "init" => do_init(desk, flags),
        [cmd, flags @ ..] if cmd == "install" => do_install(flags),
        [cmd, rest @ ..] if cmd == "work" => dispatch_work(desk, rest),
        [cmd, rest @ ..] if cmd == "deed" => dispatch_deed(desk, rest),
        [cmd, rest @ ..] if cmd == "plan" => dispatch_plan(desk, rest),
        [cmd, rest @ ..] if cmd == "memory" => dispatch_memory(desk, rest),
        _ => Err(Error::Usage),
    }
}

fn dispatch_work(desk: Option<&Path>, argv: &[String]) -> Result<()> {
    match argv {
        [sub, id, as_, actor, gen_, gen] if sub == "claim" && as_ == "--as" && gen_ == "--gen" => {
            do_claim(
                desk,
                id,
                actor,
                Some(gen.parse().map_err(|_| Error::Usage)?),
            )
        }
        [sub, id, as_, actor] if sub == "claim" && as_ == "--as" => do_claim(desk, id, actor, None),
        [sub, parent, words @ ..] if sub == "spawn" && !words.is_empty() => {
            do_spawn(desk, parent, &words.join(" "))
        }
        [sub, id, flags @ ..] if sub == "complete" => do_complete(desk, id, flags),
        [sub, parent, child] if sub == "link" => do_link(desk, parent, child),
        [sub, parent, child] if sub == "unlink" => do_unlink(desk, parent, child),
        [sub, id] if sub == "archive" => do_archive(desk, id),
        [sub, id] if sub == "unarchive" => do_unarchive(desk, id),
        [sub] if sub == "ready" => do_ready(desk),
        [sub] if sub == "list" => do_work_list(desk, &[]),
        [sub, flags @ ..] if sub == "list" => do_work_list(desk, flags),
        [sub] if sub == "verify" => do_verify(desk),
        [sub, ticket] if sub == "mint" => do_mint(desk, ticket),
        [sub, id, other] if sub == "depend" => do_depend(desk, id, other),
        words if !words.is_empty() && !WORK_VERBS.contains(&words[0].as_str()) => {
            do_work(desk, words)
        }
        [sub, words @ ..] if sub == "add" && !words.is_empty() => do_work(desk, words),
        _ => Err(Error::Usage),
    }
}

const WORK_VERBS: &[&str] = &[
    "claim",
    "spawn",
    "complete",
    "link",
    "unlink",
    "archive",
    "unarchive",
    "ready",
    "list",
    "verify",
    "add",
    "mint",
    "depend",
];

fn dispatch_deed(desk: Option<&Path>, argv: &[String]) -> Result<()> {
    match argv {
        [sub, id] if sub == "get" => do_get(desk, id),
        [sub, id] if sub == "evidence" => do_evidence(desk, id),
        [sub, id] if sub == "trail" => do_trail(desk, id),
        [sub, id] if sub == "current" => do_current(desk, id),
        [sub, id, dest] if sub == "leave" => do_leave(desk, id, dest),
        [sub, id] if sub == "timestamp" => do_timestamp(desk, id),
        [sub] if sub == "list" => do_deed_list(desk),
        [sub, kind, flags @ ..] if sub == "create" => do_deed(desk, kind, flags),
        [kind, flags @ ..] if crate::node::DeedKind::known(kind).is_some() => {
            do_deed(desk, kind, flags)
        }
        _ => Err(Error::Usage),
    }
}

fn dispatch_plan(desk: Option<&Path>, argv: &[String]) -> Result<()> {
    match argv {
        [sub, id, blocker] if sub == "block" => do_block(desk, id, blocker),
        [sub, id] if sub == "close" => do_close(desk, id),
        [sub, id, words @ ..] if sub == "note" && !words.is_empty() => {
            do_note(desk, id, &words.join(" "))
        }
        [sub, id, words @ ..] if sub == "append" && !words.is_empty() => {
            do_append(desk, id, &words.join(" "))
        }
        [sub, id, dest] if sub == "reject" => do_reject(desk, id, dest),
        [sub, id, dest] if sub == "resolve" => do_resolve(desk, id, dest),
        [sub, id] if sub == "related" => do_related(desk, id),
        [sub] if sub == "agenda" => do_agenda(desk),
        [sub, id] if sub == "children" => do_children(desk, id),
        [sub, id] if sub == "ancestors" => do_ancestors(desk, id),
        [sub, id] if sub == "impact" => do_impact(desk, id),
        [sub, id] if sub == "tree" => do_tree(desk, id),
        [sub] if sub == "cycles" => do_cycles(desk),
        [sub] if sub == "list" => do_plan_list(desk),
        [sub] if sub == "export" => do_export(desk),
        [sub] if sub == "roadmap" => do_roadmap(desk),
        [sub] if sub == "graph" => do_graph(desk),
        [sub] if sub == "digest" => do_digest(desk),
        [sub, days] if sub == "stale" => do_stale(desk, days),
        [sub, words @ ..] if sub == "add" && !words.is_empty() => do_plan(desk, words),
        words if !words.is_empty() && !PLAN_VERBS.contains(&words[0].as_str()) => {
            do_plan(desk, words)
        }
        _ => Err(Error::Usage),
    }
}

const PLAN_VERBS: &[&str] = &[
    "add",
    "list",
    "block",
    "close",
    "note",
    "append",
    "reject",
    "resolve",
    "related",
    "agenda",
    "children",
    "ancestors",
    "impact",
    "tree",
    "cycles",
    "export",
    "roadmap",
    "graph",
    "digest",
    "stale",
];

fn dispatch_memory(desk: Option<&Path>, argv: &[String]) -> Result<()> {
    match argv {
        [sub, words @ ..] if sub == "remember" && !words.is_empty() => {
            do_remember(desk, &words.join(" "))
        }
        [sub, words @ ..] if sub == "prefer" && !words.is_empty() => {
            do_prefer(desk, &words.join(" "))
        }
        [sub, name] if sub == "pin" => do_pin(desk, name),
        [sub] if sub == "unpin" => do_unpin(desk),
        [sub] if sub == "cards" => do_cards(desk),
        _ => Err(Error::Usage),
    }
}

fn do_open(desk: Option<&Path>) -> Result<()> {
    let opened = sitting(desk)?;
    println!("open {}", opened.root.display());
    Ok(())
}

fn do_work(desk: Option<&Path>, words: &[String]) -> Result<()> {
    let (project, summary) = take_project(words)?;
    let mut d = sitting(desk)?;
    if let Some(name) = project {
        d.project = Some(name);
    }
    let work = d.upsert(&summary)?;
    println!("{}  ready  {}", work.id, work.summary);
    Ok(())
}

fn do_mint(desk: Option<&Path>, ticket: &str) -> Result<()> {
    let d = sitting(desk)?;
    let work = d.work_from_plan(ticket)?;
    println!("{}  ready  {}", work.id, work.summary);
    Ok(())
}

fn do_depend(desk: Option<&Path>, id: &str, other: &str) -> Result<()> {
    let d = sitting(desk)?;
    let node = d.depend(id, other)?;
    println!("{}  deps {}", node.id, node.deps.join(","));
    Ok(())
}

fn do_wait(desk: Option<&Path>, id: &str, timeout_ms: Option<u64>) -> Result<()> {
    let d = sitting(desk)?;
    let node = d.wait(id, timeout_ms)?;
    match node.deed.as_deref() {
        Some(deed) => println!("{}  {}  deed {deed}", node.id, node.status.token()),
        None => println!("{}  {}", node.id, node.status.token()),
    }
    Ok(())
}

fn take_project(words: &[String]) -> Result<(Option<String>, String)> {
    let mut project = None;
    let mut rest = Vec::new();
    let mut i = 0;
    while i < words.len() {
        if words[i] == "--project" && i + 1 < words.len() {
            project = Some(words[i + 1].clone());
            i += 2;
            continue;
        }
        rest.push(words[i].clone());
        i += 1;
    }
    if rest.is_empty() {
        return Err(Error::Usage);
    }
    Ok((project, rest.join(" ")))
}

fn do_claim(desk: Option<&Path>, id: &str, actor: &str, gen: Option<u64>) -> Result<()> {
    let d = sitting(desk)?;
    let work = d.claim(id, actor, gen)?;
    println!(
        "{}  claimed  {}  {}",
        work.id,
        work.assignee.as_deref().unwrap_or(""),
        work.summary
    );
    Ok(())
}

fn do_spawn(desk: Option<&Path>, parent: &str, summary: &str) -> Result<()> {
    let d = sitting(desk)?;
    let child = d.spawn(parent, summary)?;
    println!("{}  ready  {}", child.id, child.summary);
    Ok(())
}

fn do_complete(desk: Option<&Path>, id: &str, flags: &[String]) -> Result<()> {
    let d = sitting(desk)?;
    let opts = complete_opts(flags)?;
    let done = d.complete(
        id,
        opts.file.as_deref(),
        opts.sources,
        opts.supersedes,
        None,
        opts.status,
    )?;
    match done.deed {
        None => println!("done  {}", done.work.id),
        Some(deed) => println!("done  {}  deed {}", done.work.id, deed.id),
    }
    Ok(())
}

struct CompleteOpts {
    file: Option<PathBuf>,
    sources: Vec<String>,
    supersedes: Option<String>,
    status: Status,
}

fn complete_opts(flags: &[String]) -> Result<CompleteOpts> {
    let mut file = None;
    let mut sources = Vec::new();
    let mut supersedes = None;
    let mut status = Status::Done;
    let mut i = 0;
    while i < flags.len() {
        match flags[i].as_str() {
            "--file" if i + 1 < flags.len() => {
                file = Some(PathBuf::from(&flags[i + 1]));
                i += 2;
            }
            "--source" if i + 1 < flags.len() => {
                sources.push(flags[i + 1].clone());
                i += 2;
            }
            "--supersedes" if i + 1 < flags.len() => {
                supersedes = Some(flags[i + 1].clone());
                i += 2;
            }
            "--status" if i + 1 < flags.len() => {
                status = match flags[i + 1].as_str() {
                    "failed" => Status::Failed,
                    "cancelled" => Status::Cancelled,
                    _ => Status::Done,
                };
                i += 2;
            }
            _ => return Err(Error::Usage),
        }
    }
    Ok(CompleteOpts {
        file,
        sources,
        supersedes,
        status,
    })
}

fn do_link(desk: Option<&Path>, parent: &str, child: &str) -> Result<()> {
    let d = sitting(desk)?;
    let node = d.link(parent, child)?;
    println!("{}  deps {}", node.id, node.deps.join(","));
    Ok(())
}

fn do_unlink(desk: Option<&Path>, parent: &str, child: &str) -> Result<()> {
    let d = sitting(desk)?;
    let node = d.unlink(parent, child)?;
    println!("{}  unlinked", node.id);
    Ok(())
}

fn do_archive(desk: Option<&Path>, id: &str) -> Result<()> {
    let d = sitting(desk)?;
    let node = d.archive(id)?;
    println!("{}  archived", node.id);
    Ok(())
}

fn do_unarchive(desk: Option<&Path>, id: &str) -> Result<()> {
    let d = sitting(desk)?;
    let node = d.unarchive(id)?;
    println!("{}  unarchived", node.id);
    Ok(())
}

fn do_get(desk: Option<&Path>, id: &str) -> Result<()> {
    let d = sitting(desk)?;
    let deed = d.deed(id)?;
    println!("{}  {}", deed.id, deed.summary);
    let mut files: Vec<_> = d.files(id).into_iter().collect();
    files.sort_by(|a, b| a.0.cmp(&b.0));
    for (_rel, body) in files {
        let _ = io::stdout().write_all(&body);
    }
    Ok(())
}

fn do_evidence(desk: Option<&Path>, id: &str) -> Result<()> {
    let d = sitting(desk)?;
    d.evidence(id)?;
    println!("ok  {id}");
    Ok(())
}

fn do_trail(desk: Option<&Path>, id: &str) -> Result<()> {
    let d = sitting(desk)?;
    for step in d.trail(id) {
        println!("{step}");
    }
    Ok(())
}

fn do_current(desk: Option<&Path>, id: &str) -> Result<()> {
    let d = sitting(desk)?;
    let deed = d.current(id)?;
    println!("{}  {}", deed.id, deed.summary);
    Ok(())
}

fn do_leave(desk: Option<&Path>, id: &str, dest: &str) -> Result<()> {
    let d = sitting(desk)?;
    let dir = d.leave(id, Path::new(dest))?;
    println!("leave {}", dir.display());
    Ok(())
}

fn do_timestamp(desk: Option<&Path>, id: &str) -> Result<()> {
    let d = sitting(desk)?;
    let path = d.timestamp(id)?;
    println!("timestamp {}", path.display());
    Ok(())
}

fn do_remember(desk: Option<&Path>, text: &str) -> Result<()> {
    let d = sitting(desk)?;
    let atom = d.remember(text)?;
    println!("{}  {}", atom.id, atom.text);
    Ok(())
}

fn do_prefer(desk: Option<&Path>, text: &str) -> Result<()> {
    let d = sitting(desk)?;
    let atom = d.prefer(text)?;
    println!(
        "{}  {}  {}",
        atom.id,
        atom.atom_kind.as_deref().unwrap_or("preference"),
        atom.text
    );
    Ok(())
}

fn do_plan(desk: Option<&Path>, words: &[String]) -> Result<()> {
    let mut parent = None;
    let mut deadline = None;
    let mut scheduled = None;
    let mut ticket_type = None;
    let mut tags = Vec::new();
    let mut priority = None;
    let mut body = None;
    let mut project = None;
    let mut rest = Vec::new();
    let mut i = 0;
    while i < words.len() {
        if i + 1 < words.len() {
            match words[i].as_str() {
                "--project" => {
                    project = Some(words[i + 1].clone());
                    i += 2;
                    continue;
                }
                "--parent" => {
                    parent = Some(words[i + 1].clone());
                    i += 2;
                    continue;
                }
                "--deadline" => {
                    deadline = Some(words[i + 1].clone());
                    i += 2;
                    continue;
                }
                "--scheduled" => {
                    scheduled = Some(words[i + 1].clone());
                    i += 2;
                    continue;
                }
                "--type" => {
                    ticket_type = Some(words[i + 1].clone());
                    i += 2;
                    continue;
                }
                "--tag" => {
                    tags.push(words[i + 1].clone());
                    i += 2;
                    continue;
                }
                "--priority" => {
                    priority = Some(words[i + 1].clone());
                    i += 2;
                    continue;
                }
                "--body" => {
                    body = Some(words[i + 1].clone());
                    i += 2;
                    continue;
                }
                _ => {}
            }
        }
        rest.push(words[i].clone());
        i += 1;
    }
    if rest.is_empty() {
        return Err(Error::Usage);
    }
    let mut d = sitting(desk)?;
    if let Some(name) = project {
        d.project = Some(name);
    }
    let ticket = d.plan(
        &rest.join(" "),
        parent,
        deadline,
        scheduled,
        ticket_type,
        tags,
        priority,
        body,
    )?;
    println!(
        "{}  {}  {}",
        ticket.id,
        ticket.status.token(),
        ticket.summary
    );
    Ok(())
}

fn do_block(desk: Option<&Path>, id: &str, blocker: &str) -> Result<()> {
    let d = sitting(desk)?;
    let ticket = d.block(id, blocker)?;
    println!("{}  blocked  {}", ticket.id, ticket.blocked_by.join(","));
    Ok(())
}

fn do_close(desk: Option<&Path>, id: &str) -> Result<()> {
    let d = sitting(desk)?;
    let ticket = d.close(id)?;
    println!("{}  done  {}", ticket.id, ticket.summary);
    Ok(())
}

fn do_note(desk: Option<&Path>, id: &str, text: &str) -> Result<()> {
    let d = sitting(desk)?;
    let ticket = d.note(id, text)?;
    println!("{}  note", ticket.id);
    Ok(())
}

fn do_append(desk: Option<&Path>, id: &str, text: &str) -> Result<()> {
    let d = sitting(desk)?;
    let ticket = d.append(id, text)?;
    println!("{}  append", ticket.id);
    Ok(())
}

fn do_reject(desk: Option<&Path>, id: &str, dest: &str) -> Result<()> {
    let d = sitting(desk)?;
    let ticket = d.reject(id, dest)?;
    println!(
        "{}  rejected  {}",
        ticket.id,
        ticket.replacement.as_deref().unwrap_or("")
    );
    Ok(())
}

fn do_resolve(desk: Option<&Path>, id: &str, dest: &str) -> Result<()> {
    let d = sitting(desk)?;
    let ticket = d.resolve(id, dest)?;
    println!(
        "{}  resolved  {}",
        ticket.id,
        ticket.replacement.as_deref().unwrap_or("")
    );
    Ok(())
}

fn do_related(desk: Option<&Path>, id: &str) -> Result<()> {
    let d = sitting(desk)?;
    for hit in d.related(id) {
        println!("{}  {}  {}", hit.id, hit.score, hit.summary);
    }
    Ok(())
}

fn do_plan_list(desk: Option<&Path>) -> Result<()> {
    let d = sitting(desk)?;
    for ticket in d.plan_list() {
        println!(
            "{}  {}  {}",
            ticket.id,
            ticket.status.token(),
            ticket.summary
        );
    }
    Ok(())
}

fn do_agenda(desk: Option<&Path>) -> Result<()> {
    let d = sitting(desk)?;
    for t in d.agenda() {
        let when = t
            .deadline
            .clone()
            .or(t.scheduled.clone())
            .unwrap_or_default();
        println!("{}  {when}  {}", t.id, t.summary);
    }
    Ok(())
}

fn do_children(desk: Option<&Path>, id: &str) -> Result<()> {
    let d = sitting(desk)?;
    for t in d.children(id) {
        println!("{}  {}", t.id, t.summary);
    }
    Ok(())
}

fn do_ancestors(desk: Option<&Path>, id: &str) -> Result<()> {
    let d = sitting(desk)?;
    for a in d.ancestors(id) {
        println!("{a}");
    }
    Ok(())
}

fn do_impact(desk: Option<&Path>, id: &str) -> Result<()> {
    let d = sitting(desk)?;
    for a in d.impact(id) {
        println!("{a}");
    }
    Ok(())
}

fn do_tree(desk: Option<&Path>, id: &str) -> Result<()> {
    let d = sitting(desk)?;
    println!("{:?}", d.tree(id).id);
    Ok(())
}

fn do_cycles(desk: Option<&Path>) -> Result<()> {
    let d = sitting(desk)?;
    for cycle in d.cycles() {
        println!("{}", cycle.join(" -> "));
    }
    Ok(())
}

fn do_search(desk: Option<&Path>, query: &str) -> Result<()> {
    let d = sitting(desk)?;
    for hit in d.search(query) {
        println!("{}  {}  {}  {}", hit.id, hit.record, hit.field, hit.text);
    }
    Ok(())
}

fn do_ready(desk: Option<&Path>) -> Result<()> {
    let d = sitting(desk)?;
    for work in d.ready() {
        println!("{}  ready  {}", work.id, work.summary);
    }
    Ok(())
}

fn do_status(desk: Option<&Path>) -> Result<()> {
    let d = sitting(desk)?;
    print!("{}", d.format_status());
    if !d.format_status().ends_with('\n') {
        println!();
    }
    Ok(())
}

fn do_work_list(desk: Option<&Path>, flags: &[String]) -> Result<()> {
    let d = sitting(desk)?;
    let all = flags.iter().any(|f| f == "--all");
    let archived = flags.iter().any(|f| f == "--archived");
    let terminal = flags.iter().any(|f| f == "--terminal");
    for work in d.work_list(archived, all, terminal) {
        println!("{}  {}  {}", work.id, work.status.token(), work.summary);
    }
    Ok(())
}

fn do_deed_list(desk: Option<&Path>) -> Result<()> {
    let d = sitting(desk)?;
    for deed in d.deeds() {
        println!(
            "{}  {}  {}",
            deed.id,
            deed.deed_kind.as_deref().unwrap_or("file"),
            deed.summary
        );
    }
    Ok(())
}

fn do_verify(desk: Option<&Path>) -> Result<()> {
    let d = sitting(desk)?;
    let lines = d.verify();
    if lines.is_empty() {
        println!("ok");
    } else {
        for line in lines {
            println!("{line}");
        }
    }
    Ok(())
}

fn do_pin(desk: Option<&Path>, name: &str) -> Result<()> {
    let d = sitting(desk)?;
    let name = d.pin(name)?;
    println!("pin {name}");
    Ok(())
}

fn do_unpin(desk: Option<&Path>) -> Result<()> {
    let d = sitting(desk)?;
    d.unpin()?;
    println!("unpin");
    Ok(())
}

fn do_deed(desk: Option<&Path>, kind: &str, flags: &[String]) -> Result<()> {
    let kind = crate::node::DeedKind::parse(kind)?;
    let mut summary = String::new();
    let mut extra = std::collections::BTreeMap::new();
    let mut file = None;
    let mut i = 0;
    while i < flags.len() {
        if i + 1 < flags.len() {
            match flags[i].as_str() {
                "--name" | "--summary" => {
                    summary = flags[i + 1].clone();
                    i += 2;
                    continue;
                }
                "--file" | "--path" => {
                    file = Some(std::path::PathBuf::from(&flags[i + 1]));
                    i += 2;
                    continue;
                }
                flag if flag.starts_with("--") => {
                    extra.insert(
                        flag.trim_start_matches('-').to_string(),
                        flags[i + 1].clone(),
                    );
                    i += 2;
                    continue;
                }
                _ => {}
            }
        }
        if summary.is_empty() {
            summary = flags[i].clone();
        }
        i += 1;
    }
    if summary.is_empty() {
        return Err(Error::Usage);
    }
    let d = sitting(desk)?;
    let deed = d.create_deed(kind, &summary, extra, file.as_deref())?;
    println!("{}  {}  {}", deed.id, kind.token(), deed.summary);
    Ok(())
}

fn do_export(desk: Option<&Path>) -> Result<()> {
    let d = sitting(desk)?;
    println!("{}", d.export_plan());
    Ok(())
}

fn do_roadmap(desk: Option<&Path>) -> Result<()> {
    let d = sitting(desk)?;
    print!("{}", d.roadmap());
    Ok(())
}

fn do_graph(desk: Option<&Path>) -> Result<()> {
    let d = sitting(desk)?;
    print!("{}", d.graph());
    Ok(())
}

fn do_digest(desk: Option<&Path>) -> Result<()> {
    let d = sitting(desk)?;
    println!("{}", d.digest());
    Ok(())
}

fn do_stale(desk: Option<&Path>, days: &str) -> Result<()> {
    let n: i64 = days.parse().map_err(|_| Error::Usage)?;
    let d = sitting(desk)?;
    for t in d.stale(n) {
        println!(
            "{}  {}  {}",
            t.id,
            t.created.as_deref().unwrap_or(""),
            t.summary
        );
    }
    Ok(())
}

fn do_cards(desk: Option<&Path>) -> Result<()> {
    let d = sitting(desk)?;
    print!("{}", d.cards());
    Ok(())
}

fn do_sit(desk: Option<&Path>, _flags: &[String]) -> Result<()> {
    let d = sitting(desk)?;
    print!("{}", sit::dump(&d));
    Ok(())
}

fn do_mcp(desk: Option<&Path>) -> Result<()> {
    let _ = sitting(desk)?;
    let stdin = io::stdin();
    for line in stdin.lock().lines() {
        let Ok(line) = line else { break };
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if let Ok(msg) = serde_json::from_str::<serde_json::Value>(trimmed) {
            if let Some(reply) = mcp::handle(&msg, desk) {
                println!("{}", serde_json::to_string(&reply).unwrap_or_default());
            }
        }
    }
    Ok(())
}

fn do_init(desk: Option<&Path>, flags: &[String]) -> Result<()> {
    let mut root = None;
    let mut name = None;
    let mut i = 0;
    while i < flags.len() {
        if i + 1 < flags.len() {
            match flags[i].as_str() {
                "--dir" => {
                    root = Some(PathBuf::from(&flags[i + 1]));
                    i += 2;
                    continue;
                }
                "--project" => {
                    name = Some(flags[i + 1].clone());
                    i += 2;
                    continue;
                }
                _ => {}
            }
        }
        i += 1;
    }
    let result = init::run(root.as_deref(), name.as_deref(), desk)?;
    println!("init {}", result.root.display());
    println!("project {}", result.project);
    println!("desk {}", result.desk.display());
    Ok(())
}

fn do_install(flags: &[String]) -> Result<()> {
    let mut bin = None;
    let mut home = None;
    let mut source = None;
    let mut i = 0;
    while i < flags.len() {
        if i + 1 >= flags.len() {
            break;
        }
        match flags[i].as_str() {
            "--bin" => bin = Some(PathBuf::from(&flags[i + 1])),
            "--home" => home = Some(PathBuf::from(&flags[i + 1])),
            "--source" => source = Some(PathBuf::from(&flags[i + 1])),
            _ => {}
        }
        i += 2;
    }
    let result = install::run(bin.as_deref(), home.as_deref(), source.as_deref())?;
    println!("install {}", result.bin.display());
    println!(
        "{}",
        serde_json::to_string_pretty(&result.block).unwrap_or_default()
    );
    Ok(())
}

fn surface() -> Result<()> {
    println!("saena — a desk. One tree. Claim work. Complete a deed. Remember.");
    println!();
    println!("  desk is $XDG_DATA_HOME/saena/desk");
    println!("  a checkout names itself in .saena/project");
    println!();
    println!("  status                    desk glance");
    println!("  sit                       terminal desk");
    println!("  search QUERY              search the tree");
    println!("  open                      start or resume a desk");
    println!("  wait ID                   sleep until that node is terminal");
    println!();
    println!("  work SUMMARY              put ready work on the frontier");
    println!("  work --project NAME       file work in another project");
    println!("  work claim ID --as ACTOR  lock a ready node");
    println!("  work spawn PARENT SUMMARY insert a child work node");
    println!("  work complete ID          finish; host snapshots changes");
    println!("  work mint TICKET          mint work from a ticket");
    println!("  work depend ID OTHER      ID waits on OTHER");
    println!("  work link PARENT CHILD    hard dependency");
    println!("  work ready                claimable work");
    println!("  work list                 live work");
    println!("  work verify               check the work graph");
    println!();
    println!("  deed get ID               open a frozen deed");
    println!("  deed list                 live deeds");
    println!("  deed KIND --name TEXT     create a typed deed");
    println!("  deed evidence ID          check the issued product");
    println!("  deed trail ID             walk sources");
    println!();
    println!("  plan SUMMARY              durable ticket");
    println!("  plan --project NAME       file a ticket in another project");
    println!("  plan list                 tickets in this project");
    println!("  plan block ID OTHER       ticket waits on other");
    println!("  plan close ID             close a ticket");
    println!("  plan agenda               dated open tickets");
    println!();
    println!("  memory remember TEXT      commit a memory atom");
    println!("  memory prefer TEXT        commit a preference atom");
    println!("  memory pin NAME           scope remember and search");
    println!("  memory cards              USER.md and MEMORY.md");
    println!();
    println!("  mcp                       Model Context Protocol");
    println!("  install                   put saena on the path");
    println!("  init                      opt this folder in");
    println!("  surface                   list the sitting verbs");
    Ok(())
}

pub fn verbs() -> &'static [&'static str] {
    VERBS
}

#[allow(dead_code)]
pub fn default_actor() -> String {
    whoami()
}
