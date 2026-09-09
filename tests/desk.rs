//! Desk tree: claim, deed, ticket, atom.

use saena::desk::Desk;
use saena::node::{Kind, Status};
use saena::Error;
use std::fs;
use std::path::Path;

fn open(dir: &Path) -> Desk {
    Desk::open(dir).expect("open")
}

#[test]
fn open_creates_a_desk_the_next_open_can_load() {
    let tmp = tempfile::tempdir().unwrap();
    let desk = open(tmp.path());
    let again = Desk::open(tmp.path()).unwrap();
    assert_eq!(again.root, desk.root);
    assert!(again.ready().is_empty());
}

#[test]
fn upsert_puts_ready_work_on_the_frontier() {
    let tmp = tempfile::tempdir().unwrap();
    let desk = open(tmp.path());
    let work = desk.upsert("name the note").unwrap();
    assert_eq!(work.summary, "name the note");
    assert_eq!(work.status, Status::Ready);
    assert!(work.assignee.is_none());
    let ready = desk.ready();
    assert_eq!(ready.len(), 1);
    assert_eq!(ready[0].id, work.id);
}

#[test]
fn claim_is_exclusive() {
    let tmp = tempfile::tempdir().unwrap();
    let desk = open(tmp.path());
    let work = desk.upsert("one lock").unwrap();
    let claimed = desk.claim(&work.id, "alice", None).unwrap();
    assert_eq!(claimed.status, Status::Claimed);
    assert_eq!(claimed.assignee.as_deref(), Some("alice"));
    assert_eq!(desk.claim(&work.id, "bob", None), Err(Error::NotReady));
    assert!(desk.ready().is_empty());
}

#[test]
fn spawn_inserts_a_child_the_child_actor_can_claim() {
    let tmp = tempfile::tempdir().unwrap();
    let desk = open(tmp.path());
    let parent = desk.upsert("parent").unwrap();
    let parent = desk.claim(&parent.id, "alice", None).unwrap();
    let child = desk.spawn(&parent.id, "child work").unwrap();
    assert_eq!(child.parent.as_deref(), Some(parent.id.as_str()));
    assert_eq!(child.status, Status::Ready);
    let taken = desk.claim(&child.id, "child", None).unwrap();
    assert_eq!(taken.assignee.as_deref(), Some("child"));
    let parent = desk.get_work(&parent.id).unwrap();
    assert!(parent.children.contains(&child.id));
}

#[test]
fn complete_of_producing_work_names_a_deed() {
    let tmp = tempfile::tempdir().unwrap();
    let desk = open(tmp.path());
    let product = tmp.path().join("note.txt");
    fs::write(&product, "a named note\n").unwrap();
    let work = desk.upsert("name the note").unwrap();
    desk.claim(&work.id, "alice", None).unwrap();
    let result = desk
        .complete(&work.id, Some(&product), vec![], None, None, Status::Done)
        .unwrap();
    let deed = result.deed.unwrap();
    assert_eq!(result.work.status, Status::Done);
    assert_eq!(result.work.deed.as_deref(), Some(deed.id.as_str()));
    let opened = desk.deed(&deed.id).unwrap();
    assert_eq!(opened.id, deed.id);
    assert_eq!(desk.product(&opened.id).unwrap(), b"a named note\n");
    assert!(desk.evidence(&opened.id).is_ok());
    assert_eq!(desk.trail(&opened.id), vec![opened.id]);
}

#[test]
fn complete_without_a_file_leaves_no_deed_when_nothing_changed() {
    let tmp = tempfile::tempdir().unwrap();
    let desk = open(tmp.path());
    let work = desk.upsert("empty").unwrap();
    desk.claim(&work.id, "alice", None).unwrap();
    let result = desk
        .complete(&work.id, None, vec![], None, None, Status::Done)
        .unwrap();
    assert!(result.deed.is_none());
    assert_eq!(result.work.status, Status::Done);
    assert!(result.work.deed.is_none());
}

#[test]
fn hidden_saena_dir_snapshots_the_parent_tree() {
    let tmp = tempfile::tempdir().unwrap();
    let desk = Desk::open(tmp.path().join(".saena")).unwrap();
    let work = desk.upsert("write the note").unwrap();
    desk.claim(&work.id, "alice", None).unwrap();
    fs::write(tmp.path().join("note.txt"), "a named note\n").unwrap();
    let result = desk
        .complete(&work.id, None, vec![], None, None, Status::Done)
        .unwrap();
    let deed = result.deed.unwrap();
    let files = desk.files(&deed.id);
    assert_eq!(
        files.get("note.txt").map(Vec::as_slice),
        Some(b"a named note\n".as_slice())
    );
}

#[test]
fn complete_without_a_file_snapshots_files_written_since_claim() {
    let tmp = tempfile::tempdir().unwrap();
    let desk = open(tmp.path());
    let work = desk.upsert("write the note").unwrap();
    desk.claim(&work.id, "alice", None).unwrap();
    fs::write(tmp.path().join("note.txt"), "a named note\n").unwrap();
    let result = desk
        .complete(&work.id, None, vec![], None, None, Status::Done)
        .unwrap();
    let deed = result.deed.unwrap();
    assert!(desk.evidence(&deed.id).is_ok());
    let files = desk.files(&deed.id);
    assert_eq!(
        files.get("note.txt").map(Vec::as_slice),
        Some(b"a named note\n".as_slice())
    );
}

#[test]
fn trail_walks_source_deeds() {
    let tmp = tempfile::tempdir().unwrap();
    let desk = open(tmp.path());
    let a = tmp.path().join("a.txt");
    let b = tmp.path().join("b.txt");
    fs::write(&a, "quote").unwrap();
    fs::write(&b, "patch").unwrap();
    let first = desk.upsert("quote").unwrap();
    desk.claim(&first.id, "r", None).unwrap();
    let src = desk
        .complete(&first.id, Some(&a), vec![], None, None, Status::Done)
        .unwrap()
        .deed
        .unwrap();
    let second = desk.upsert("patch").unwrap();
    desk.claim(&second.id, "r", None).unwrap();
    let out = desk
        .complete(
            &second.id,
            Some(&b),
            vec![src.id.clone()],
            None,
            None,
            Status::Done,
        )
        .unwrap()
        .deed
        .unwrap();
    assert_eq!(desk.trail(&out.id), vec![src.id.clone(), out.id.clone()]);
    assert!(desk.evidence(&out.id).is_ok());
}

#[test]
fn remember_atom_is_visible_after_commit() {
    let tmp = tempfile::tempdir().unwrap();
    let desk = open(tmp.path());
    assert!(desk.memory().is_empty());
    let atom = desk.remember("always open the deed, not the chat").unwrap();
    assert_eq!(atom.text, "always open the deed, not the chat");
    let got = desk.memory();
    assert_eq!(got.len(), 1);
    assert_eq!(got[0].id, atom.id);
}

#[test]
fn plan_tickets_stay_open_when_work_cites_them() {
    let tmp = tempfile::tempdir().unwrap();
    let desk = open(tmp.path());
    let ticket = desk
        .plan("name the note", None, None, None, None, vec![], None, None)
        .unwrap();
    assert_eq!(ticket.status, Status::Todo);
    assert_eq!(desk.plan_ready()[0].id, ticket.id);
    let work = desk.work_from_plan(&ticket.id).unwrap();
    assert_eq!(work.ticket.as_deref(), Some(ticket.id.as_str()));
    desk.claim(&work.id, "alice", None).unwrap();
    let product = tmp.path().join("note.txt");
    fs::write(&product, "note").unwrap();
    let result = desk
        .complete(
            &work.id,
            Some(&product),
            vec![],
            None,
            Some(ticket.id.clone()),
            Status::Done,
        )
        .unwrap();
    assert_eq!(result.work.ticket.as_deref(), Some(ticket.id.as_str()));
    let still = desk.get_plan(&ticket.id).unwrap();
    assert_eq!(still.status, Status::Todo);
}

#[test]
fn blocked_plan_tickets_leave_ready_until_blocker_closes() {
    let tmp = tempfile::tempdir().unwrap();
    let desk = open(tmp.path());
    let first = desk
        .plan("catalog", None, None, None, None, vec![], None, None)
        .unwrap();
    let second = desk
        .plan("schema", None, None, None, None, vec![], None, None)
        .unwrap();
    let second = desk.block(&second.id, &first.id).unwrap();
    assert_eq!(second.status, Status::Blocked);
    let ready: Vec<_> = desk.plan_ready().into_iter().map(|n| n.id).collect();
    assert!(ready.contains(&first.id));
    assert!(!ready.contains(&second.id));
    desk.close(&first.id).unwrap();
    let ready: Vec<_> = desk.plan_ready().into_iter().map(|n| n.id).collect();
    assert!(ready.contains(&second.id));
}

#[test]
fn claim_with_a_stale_generation_is_a_conflict() {
    let tmp = tempfile::tempdir().unwrap();
    let desk = open(tmp.path());
    let work = desk.upsert("one lock").unwrap();
    assert_eq!(work.gen, 0);
    assert_eq!(desk.claim(&work.id, "alice", Some(3)), Err(Error::Conflict));
    let claimed = desk.claim(&work.id, "alice", Some(0)).unwrap();
    assert_eq!(claimed.gen, 1);
    assert_eq!(desk.claim(&work.id, "bob", Some(1)), Err(Error::NotReady));
}

#[test]
fn link_blocks_the_child_until_the_parent_is_terminal() {
    let tmp = tempfile::tempdir().unwrap();
    let desk = open(tmp.path());
    let parent = desk.upsert("parent").unwrap();
    let child = desk.upsert("child").unwrap();
    let child = desk.link(&parent.id, &child.id).unwrap();
    assert!(child.deps.contains(&parent.id));
    let ready: Vec<_> = desk.ready().into_iter().map(|n| n.id).collect();
    assert!(ready.contains(&parent.id));
    assert!(!ready.contains(&child.id));
    desk.claim(&parent.id, "alice", None).unwrap();
    assert_eq!(desk.claim(&child.id, "bob", None), Err(Error::NotReady));
    desk.complete(&parent.id, None, vec![], None, None, Status::Done)
        .unwrap();
    assert!(desk.ready().iter().any(|n| n.id == child.id));
    let taken = desk.claim(&child.id, "bob", None).unwrap();
    assert_eq!(taken.assignee.as_deref(), Some("bob"));
    let child = desk.unlink(&parent.id, &child.id).unwrap();
    assert!(!child.deps.contains(&parent.id));
}

#[test]
fn project_file_names_the_checkout() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("widget");
    let nested = root.join("src");
    fs::create_dir_all(&nested).unwrap();
    let wrote = saena::project::Project::write(&root, "widget").unwrap();
    assert_eq!(wrote.name, "widget");
    let found = saena::project::Project::find(&nested).unwrap();
    assert_eq!(found.name, "widget");
    assert_eq!(found.root, root);
}

#[test]
fn ensure_project_is_the_same_ticket_twice() {
    let tmp = tempfile::tempdir().unwrap();
    let desk = open(tmp.path());
    let first = desk.ensure_project("widget", tmp.path()).unwrap();
    let again = desk.ensure_project("widget", tmp.path()).unwrap();
    assert_eq!(first.id, again.id);
    assert_eq!(first.project.as_deref(), Some("widget"));
}

#[test]
fn depend_blocks_claim_until_the_other_work_is_terminal() {
    let tmp = tempfile::tempdir().unwrap();
    let desk = open(tmp.path());
    let theirs = desk.upsert("need a list id").unwrap();
    let mine = desk.upsert("continue after list").unwrap();
    let mine = desk.depend(&mine.id, &theirs.id).unwrap();
    assert!(mine.deps.contains(&theirs.id));
    assert_eq!(desk.claim(&mine.id, "alice", None), Err(Error::NotReady));
    desk.claim(&theirs.id, "bob", None).unwrap();
    let product = tmp.path().join("list.rs");
    fs::write(&product, "pub struct ListId;\n").unwrap();
    let done = desk
        .complete(&theirs.id, Some(&product), vec![], None, None, Status::Done)
        .unwrap();
    let deed = done.deed.unwrap();
    let waited = desk.wait(&theirs.id, Some(0)).unwrap();
    assert_eq!(waited.deed.as_deref(), Some(deed.id.as_str()));
    let taken = desk.claim(&mine.id, "alice", None).unwrap();
    assert_eq!(taken.assignee.as_deref(), Some("alice"));
}

#[test]
fn archive_hides_a_terminal_node() {
    let tmp = tempfile::tempdir().unwrap();
    let desk = open(tmp.path());
    let live = desk.upsert("still open").unwrap();
    assert_eq!(desk.archive(&live.id), Err(Error::NotTerminal));
    let work = desk.upsert("done work").unwrap();
    desk.claim(&work.id, "alice", None).unwrap();
    let done = desk
        .complete(&work.id, None, vec![], None, None, Status::Done)
        .unwrap();
    assert_eq!(done.work.status, Status::Done);
    let hidden = desk.archive(&work.id).unwrap();
    assert!(hidden.archived);
    assert!(!desk
        .work_list(false, false, false)
        .iter()
        .any(|n| n.id == work.id));
    assert!(desk
        .work_list(true, false, false)
        .iter()
        .any(|n| n.id == work.id));
    let shown = desk.unarchive(&work.id).unwrap();
    assert!(!shown.archived);
    assert_eq!(shown.status, Status::Done);
}

#[test]
fn failed_and_cancelled_stay_terminal() {
    let tmp = tempfile::tempdir().unwrap();
    let desk = open(tmp.path());
    let a = desk.upsert("fail").unwrap();
    desk.claim(&a.id, "alice", None).unwrap();
    let failed = desk
        .complete(&a.id, None, vec![], None, None, Status::Failed)
        .unwrap();
    assert_eq!(failed.work.status, Status::Failed);
    assert_eq!(desk.claim(&a.id, "bob", None), Err(Error::Terminal));
    assert_eq!(
        desk.complete(&a.id, None, vec![], None, None, Status::Done)
            .err(),
        Some(Error::Terminal)
    );

    let b = desk.upsert("cancel").unwrap();
    desk.claim(&b.id, "alice", None).unwrap();
    let cancelled = desk
        .complete(&b.id, None, vec![], None, None, Status::Cancelled)
        .unwrap();
    assert_eq!(cancelled.work.status, Status::Cancelled);
    assert_eq!(desk.claim(&b.id, "bob", None), Err(Error::Terminal));
}

#[test]
fn remember_and_prefer_refuse_a_tool_dump() {
    let tmp = tempfile::tempdir().unwrap();
    let desk = open(tmp.path());
    let lesson = desk
        .remember("Remember: always open the deed first")
        .unwrap();
    assert_eq!(lesson.atom_kind.as_deref(), Some("lesson"));
    assert_eq!(lesson.text, "always open the deed first");
    let pref = desk
        .prefer("Prefer: Borda then diversify on search")
        .unwrap();
    assert_eq!(pref.atom_kind.as_deref(), Some("preference"));
    assert_eq!(pref.text, "Borda then diversify on search");
    let dump = (0..9)
        .map(|i| format!("- file{i}.rs"))
        .collect::<Vec<_>>()
        .join("\n");
    assert_eq!(desk.remember(&dump), Err(Error::ToolDump));
    let ids: Vec<_> = desk.memory().into_iter().map(|a| a.id).collect();
    assert!(ids.contains(&lesson.id));
    assert!(ids.contains(&pref.id));
}

#[test]
fn search_returns_atoms_and_work() {
    let tmp = tempfile::tempdir().unwrap();
    let desk = open(tmp.path());
    assert!(!tmp.path().join("pack.json").exists());
    assert!(tmp.path().join("store.lmdb").is_dir());
    let atom = desk
        .remember("Remember: zircon pin belongs on the review set")
        .unwrap();
    let work = desk.upsert("zircon pin on the review set").unwrap();
    let hits = desk.search("zircon pin review");
    let ids: Vec<_> = hits.iter().map(|h| h.id.clone()).collect();
    assert!(
        ids.contains(&atom.id),
        "hits={ids:?} atom={} work={}",
        atom.id,
        work.id
    );
    let hit = hits.iter().find(|h| h.id == atom.id).unwrap();
    assert_eq!(hit.record, Kind::Atom.token());
    assert_eq!(hit.field, "atom");
    assert!(hit.text.contains("zircon"));
    assert!(ids.contains(&work.id));
    assert!(tmp.path().join("search.milli").is_dir());
    let again = Desk::open(tmp.path()).unwrap();
    let again_ids: Vec<_> = again
        .search("zircon pin review")
        .into_iter()
        .map(|h| h.id)
        .collect();
    assert!(again_ids.contains(&atom.id));
    assert!(again_ids.contains(&work.id));
}

#[test]
fn mint_work_from_a_ticket_the_ticket_is_not_a_lock() {
    let tmp = tempfile::tempdir().unwrap();
    let desk = open(tmp.path());
    let ticket = desk
        .plan("name the note", None, None, None, None, vec![], None, None)
        .unwrap();
    let work = desk.work_from_plan(&ticket.id).unwrap();
    assert_eq!(work.ticket.as_deref(), Some(ticket.id.as_str()));
    assert_eq!(desk.claim(&ticket.id, "alice", None), Err(Error::NotWork));
    let claimed = desk.claim(&work.id, "alice", None).unwrap();
    assert_eq!(claimed.assignee.as_deref(), Some("alice"));
    assert_eq!(desk.claim(&work.id, "bob", None), Err(Error::NotReady));
    let still = desk.get_plan(&ticket.id).unwrap();
    assert_eq!(still.status, Status::Todo);
}

#[test]
fn parent_block_related_agenda_children() {
    let tmp = tempfile::tempdir().unwrap();
    let desk = open(tmp.path());
    let catalog = desk
        .plan(
            "catalog of bindable actions",
            None,
            None,
            None,
            None,
            vec![],
            None,
            None,
        )
        .unwrap();
    let schema = desk
        .plan(
            "keys.toml schema and key names",
            Some(catalog.id.clone()),
            None,
            None,
            None,
            vec![],
            None,
            None,
        )
        .unwrap();
    let schema = desk.block(&schema.id, &catalog.id).unwrap();
    assert_eq!(schema.status, Status::Blocked);
    assert!(!desk.plan_ready().iter().any(|n| n.id == schema.id));
    desk.note(&catalog.id, "start with the catalog").unwrap();
    desk.append(&catalog.id, "catalog draft").unwrap();
    let kids = desk.children(&catalog.id);
    assert_eq!(
        kids.iter().map(|n| n.id.as_str()).collect::<Vec<_>>(),
        [schema.id.as_str()]
    );
    assert_eq!(desk.ancestors(&schema.id), vec![catalog.id.clone()]);
    assert!(!desk.impact(&catalog.id).contains(&catalog.id));
    assert!(desk.impact(&catalog.id).contains(&schema.id));
    assert!(desk.related(&catalog.id).iter().any(|h| h.id == schema.id));
    let dated = desk
        .plan(
            "dated work",
            None,
            Some(saena::node::today()),
            None,
            None,
            vec![],
            None,
            None,
        )
        .unwrap();
    assert!(desk.agenda().iter().any(|n| n.id == dated.id));
    let tree = desk.tree(&catalog.id);
    assert_eq!(tree.id, catalog.id);
    assert_eq!(tree.children[0].id, schema.id);
    desk.close(&catalog.id).unwrap();
    assert!(desk.plan_ready().iter().any(|n| n.id == schema.id));
}

#[test]
fn reject_resolve_cycles_and_backlinks() {
    let tmp = tempfile::tempdir().unwrap();
    let desk = open(tmp.path());
    let a = desk
        .plan("one", None, None, None, None, vec![], None, None)
        .unwrap();
    let b = desk
        .plan("two", None, None, None, None, vec![], None, None)
        .unwrap();
    desk.block(&a.id, &b.id).unwrap();
    desk.block(&b.id, &a.id).unwrap();
    assert!(!desk.cycles().is_empty());
    let c = desk
        .plan("three", None, None, None, None, vec![], None, None)
        .unwrap();
    desk.append(&c.id, &format!("see {}", a.id)).unwrap();
    assert!(desk.backlinks(&a.id).iter().any(|n| n.id == c.id));
    let rejected = desk.reject(&c.id, &a.id).unwrap();
    assert_eq!(rejected.status, Status::Rejected);
    let resolved = desk.resolve(&b.id, &a.id).unwrap();
    assert_eq!(resolved.status, Status::Resolved);
}

#[test]
fn get_evidence_trail_current_leave_timestamp_and_sha256() {
    let tmp = tempfile::tempdir().unwrap();
    let desk = open(tmp.path());
    let a = tmp.path().join("a.txt");
    let b = tmp.path().join("b.txt");
    fs::write(&a, "quote body\n").unwrap();
    fs::write(&b, "patch body\n").unwrap();
    let first = desk.upsert("quote").unwrap();
    desk.claim(&first.id, "r", None).unwrap();
    let src = desk
        .complete(&first.id, Some(&a), vec![], None, None, Status::Done)
        .unwrap()
        .deed
        .unwrap();
    let second = desk.upsert("patch").unwrap();
    desk.claim(&second.id, "r", None).unwrap();
    let out = desk
        .complete(
            &second.id,
            Some(&b),
            vec![src.id.clone()],
            Some(src.id.clone()),
            None,
            Status::Done,
        )
        .unwrap()
        .deed
        .unwrap();
    let opened = desk.deed(&out.id).unwrap();
    assert_eq!(opened.deed_kind.as_deref(), Some("file"));
    assert_eq!(desk.product(&opened.id).unwrap(), b"patch body\n");
    assert!(desk.evidence(&opened.id).is_ok());
    assert_eq!(
        desk.trail(&opened.id),
        vec![src.id.clone(), opened.id.clone()]
    );
    let tip = desk.current(&src.id).unwrap();
    assert_eq!(tip.id, out.id);
    let by_hash = desk
        .deed(&format!("sha256:{}", opened.sha256.as_deref().unwrap()))
        .unwrap();
    assert_eq!(by_hash.id, opened.id);
    let dest = tmp.path().join("left");
    let left = desk.leave(&opened.id, &dest).unwrap();
    assert!(left.join("manifest.json").exists());
    let manifest: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(left.join("manifest.json")).unwrap()).unwrap();
    assert_eq!(manifest["id"], opened.id);
    assert_eq!(manifest["claim_generator"], "saena");
    let tsq = desk.timestamp(&opened.id).unwrap();
    assert!(tsq.is_file());
    assert!(tsq.extension().and_then(|e| e.to_str()) == Some("tsq"));
    assert!(!fs::read(&tsq).unwrap().is_empty());
    let gone = desk.delete_deed(&src.id).unwrap();
    assert!(gone.tombstone);
    assert_eq!(desk.evidence(&src.id), Err(Error::Deleted));
}

#[test]
fn concurrent_claims_grant_exactly_one_actor() {
    let tmp = tempfile::tempdir().unwrap();
    let desk = open(tmp.path());
    let work = desk.upsert("one lock").unwrap();
    let id = work.id.clone();
    let root = desk.root.clone();
    let mut handles = Vec::new();
    for i in 1..=8 {
        let root = root.clone();
        let id = id.clone();
        handles.push(std::thread::spawn(move || {
            let desk = Desk::open(&root).unwrap();
            desk.claim(&id, &format!("actor-{i}"), None)
        }));
    }
    let results: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();
    let wins: Vec<_> = results.iter().filter(|r| r.is_ok()).collect();
    let losses: Vec<_> = results
        .iter()
        .filter(|r| *r == &Err(Error::NotReady))
        .collect();
    assert_eq!(wins.len(), 1);
    assert_eq!(losses.len(), 7);
    let held = desk.get_work(&id).unwrap();
    assert_eq!(held.status, Status::Claimed);
    assert_eq!(
        held.assignee.as_ref(),
        wins[0].as_ref().unwrap().assignee.as_ref()
    );
}

#[test]
fn verify_names_a_work_cycle() {
    let tmp = tempfile::tempdir().unwrap();
    let desk = open(tmp.path());
    let a = desk.upsert("a").unwrap();
    let b = desk.upsert("b").unwrap();
    desk.link(&a.id, &b.id).unwrap();
    desk.link(&b.id, &a.id).unwrap();
    let lines = desk.verify();
    assert!(lines.iter().any(|l| l.contains("cycle")), "{lines:?}");
}

#[test]
fn list_filters_hide_done_until_all() {
    let tmp = tempfile::tempdir().unwrap();
    let desk = open(tmp.path());
    let work = desk.upsert("done work").unwrap();
    desk.claim(&work.id, "alice", None).unwrap();
    desk.complete(&work.id, None, vec![], None, None, Status::Done)
        .unwrap();
    desk.archive(&work.id).unwrap();
    assert!(!desk
        .work_list(false, false, false)
        .iter()
        .any(|n| n.id == work.id));
    assert!(desk
        .work_list(false, true, false)
        .iter()
        .any(|n| n.id == work.id));
}

#[test]
fn status_parse_rejects_raw_string() {
    assert!(saena::node::Status::parse("ready").is_ok());
    assert!(saena::node::Status::parse("nope").is_err());
    assert!(saena::node::WorkKind::parse("task").is_ok());
    assert!(saena::node::WorkKind::parse("mystery").is_err());
    assert!(saena::node::Role::parse("unset").is_ok());
    assert!(saena::node::Role::parse("wizard").is_err());
}

#[test]
fn pin_scopes_remember() {
    let tmp = tempfile::tempdir().unwrap();
    let desk = open(tmp.path());
    desk.pin("review").unwrap();
    let atom = desk
        .remember("Remember: zircon pin belongs on the review set")
        .unwrap();
    assert_eq!(atom.pin.as_deref(), Some("review"));
}

#[test]
fn create_quote_and_patch_trail() {
    let tmp = tempfile::tempdir().unwrap();
    let desk = open(tmp.path());
    let mut extra = std::collections::BTreeMap::new();
    extra.insert("edition".into(), "https://example.com/rfc".into());
    extra.insert("excerpt".into(), "lifetimes from the CFG".into());
    let quote = desk
        .create_deed(saena::node::DeedKind::Quote, "NLL", extra, None)
        .unwrap();
    assert_eq!(quote.deed_kind.as_deref(), Some("quote"));
    assert_eq!(quote.face.as_deref(), Some("syntax"));
    let dump = saena::sit::dump(&desk);
    assert!(dump.contains("syntax"));
    assert!(dump.contains("lifetimes from the CFG"));
}

#[test]
fn list_deeds_after_two_completes() {
    let tmp = tempfile::tempdir().unwrap();
    let desk = open(tmp.path());
    let a = tmp.path().join("a.txt");
    let b = tmp.path().join("b.txt");
    fs::write(&a, "one").unwrap();
    fs::write(&b, "two").unwrap();
    let w1 = desk.upsert("one").unwrap();
    desk.claim(&w1.id, "r", None).unwrap();
    let d1 = desk
        .complete(&w1.id, Some(&a), vec![], None, None, Status::Done)
        .unwrap()
        .deed
        .unwrap();
    let w2 = desk.upsert("two").unwrap();
    desk.claim(&w2.id, "r", None).unwrap();
    let d2 = desk
        .complete(&w2.id, Some(&b), vec![], None, None, Status::Done)
        .unwrap()
        .deed
        .unwrap();
    let ids: Vec<_> = desk.deeds().into_iter().map(|d| d.id).collect();
    assert!(ids.contains(&d1.id));
    assert!(ids.contains(&d2.id));
}

#[test]
fn unique_product_hash_opens_the_deed() {
    let tmp = tempfile::tempdir().unwrap();
    let desk = open(tmp.path());
    let a = tmp.path().join("a.txt");
    fs::write(&a, "unique-bytes-xyz").unwrap();
    let w = desk.upsert("file").unwrap();
    desk.claim(&w.id, "r", None).unwrap();
    let deed = desk
        .complete(&w.id, Some(&a), vec![], None, None, Status::Done)
        .unwrap()
        .deed
        .unwrap();
    let digest = deed.sha256.clone().unwrap();
    let opened = desk.deed(&format!("sha256:{digest}")).unwrap();
    assert_eq!(opened.id, deed.id);
}

#[test]
fn host_key_requires_sidecar() {
    let tmp = tempfile::tempdir().unwrap();
    let desk = open(&tmp.path().join("desk"));
    fs::write(tmp.path().join("host.key"), "secret-host-key").unwrap();
    let file = tmp.path().join("n.txt");
    fs::write(&file, "body").unwrap();
    let w = desk.upsert("n").unwrap();
    desk.claim(&w.id, "r", None).unwrap();
    let deed = desk
        .complete(&w.id, Some(&file), vec![], None, None, Status::Done)
        .unwrap()
        .deed
        .unwrap();
    assert!(desk.evidence(&deed.id).is_ok());
    let _ = fs::remove_file(desk.root.join(format!("{}.host", deed.id)));
    assert_eq!(desk.evidence(&deed.id), Err(saena::Error::Host));
}

#[test]
fn plan_fields_and_export_and_stale() {
    let tmp = tempfile::tempdir().unwrap();
    let desk = open(tmp.path());
    let t = desk
        .plan(
            "keys.toml",
            None,
            Some("2026-09-20".into()),
            None,
            Some("schema".into()),
            vec!["keys".into()],
            Some("A".into()),
            Some("body text".into()),
        )
        .unwrap();
    assert_eq!(t.ticket_type.as_deref(), Some("schema"));
    assert_eq!(t.tags, vec!["keys"]);
    assert_eq!(t.priority.as_deref(), Some("A"));
    assert!(desk.agenda().iter().any(|n| n.id == t.id));
    assert!(desk.export_plan().contains(&t.id));
    assert!(desk.roadmap().contains("Open"));
    assert!(desk.graph().contains("digraph"));
    let d1 = desk.digest();
    let d2 = desk.digest();
    assert_eq!(d1, d2);
    let old = desk
        .plan("ancient", None, None, None, None, vec![], None, None)
        .unwrap();
    // created is now; stale 0 includes open tickets
    assert!(desk.stale(0).iter().any(|n| n.id == old.id));
}

#[test]
fn cards_are_files_not_atoms() {
    let tmp = tempfile::tempdir().unwrap();
    fs::write(tmp.path().join("USER.md"), "I am the human\n").unwrap();
    let desk = open(&tmp.path().join("desk"));
    let text = desk.cards();
    assert!(text.contains("USER.md"));
    assert!(text.contains("I am the human"));
    assert!(desk.memory().is_empty());
}

#[test]
fn sit_dump_names_the_faces() {
    let tmp = tempfile::tempdir().unwrap();
    let desk = open(tmp.path());
    desk.upsert("name the note").unwrap();
    desk.plan("catalog", None, None, None, None, vec![], None, None)
        .unwrap();
    desk.remember("Remember: always open the deed first")
        .unwrap();
    let text = saena::sit::dump(&desk);
    for face in saena::sit::faces() {
        assert!(text.contains(&format!("face {face}")), "{text}");
    }
    assert!(text.contains("Ready"));
    assert!(text.contains("name the note"));
    assert!(text.contains("catalog"));
    assert!(text.contains("always open the deed first"));
}
