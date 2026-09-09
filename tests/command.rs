//! Sitting through the shipped `saena` command.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn bin() -> PathBuf {
    env!("CARGO_BIN_EXE_saena").into()
}

fn run(desk: &Path, args: &[&str]) -> (String, i32) {
    let out = Command::new(bin())
        .args(["--desk"])
        .arg(desk)
        .args(args)
        .output()
        .expect("saena");
    let text =
        String::from_utf8_lossy(&out.stdout).into_owned() + &String::from_utf8_lossy(&out.stderr);
    (text, out.status.code().unwrap_or(1))
}

fn run_home(xdg: &Path, cwd: &Path, args: &[&str]) -> (String, i32) {
    let out = Command::new(bin())
        .env("XDG_DATA_HOME", xdg)
        .env_remove("SAENA_DESK")
        .env_remove("GROK_WORKSPACE_ROOT")
        .env_remove("CLAUDE_PROJECT_DIR")
        .current_dir(cwd)
        .args(args)
        .output()
        .expect("saena");
    let text =
        String::from_utf8_lossy(&out.stdout).into_owned() + &String::from_utf8_lossy(&out.stderr);
    (text, out.status.code().unwrap_or(1))
}

fn ok_home(xdg: &Path, cwd: &Path, args: &[&str]) -> String {
    let (text, status) = run_home(xdg, cwd, args);
    assert_eq!(status, 0, "{text}");
    text
}

fn ok(desk: &Path, args: &[&str]) -> String {
    let (text, status) = run(desk, args);
    assert_eq!(status, 0, "{text}");
    text
}

fn first_id(out: &str, prefix: &str) -> String {
    out.split_whitespace()
        .find(|w| w.starts_with(prefix))
        .unwrap_or_else(|| panic!("missing {prefix} in {out}"))
        .to_string()
}

#[test]
fn first_path_open_work_claim_complete_get_remember() {
    let tmp = tempfile::tempdir().unwrap();
    let desk = tmp.path().join("desk");
    let note = tmp.path().join("note.txt");
    fs::write(&note, "a named note\n").unwrap();

    let (out, status) = run(&desk, &["open"]);
    assert_eq!(status, 0, "{out}");
    assert!(out.contains("open"), "{out}");

    let (out, status) = run(&desk, &["work", "name the note"]);
    assert_eq!(status, 0, "{out}");
    let id = first_id(&out, "w-");
    assert!(out.contains("ready"), "{out}");
    assert!(out.contains("name the note"), "{out}");

    let (out, status) = run(&desk, &["work", "claim", &id, "--as", "alice"]);
    assert_eq!(status, 0, "{out}");
    assert!(out.contains("claimed"), "{out}");
    assert!(out.contains("alice"), "{out}");

    let (out, status) = run(
        &desk,
        &["work", "complete", &id, "--file", note.to_str().unwrap()],
    );
    assert_eq!(status, 0, "{out}");
    assert!(out.contains("done"), "{out}");
    let deed = first_id(&out, "d-");

    let (out, status) = run(&desk, &["deed", "get", &deed]);
    assert_eq!(status, 0, "{out}");
    assert!(out.contains("a named note"), "{out}");

    let (out, status) = run(&desk, &["deed", "evidence", &deed]);
    assert_eq!(status, 0, "{out}");
    assert!(out.contains("ok"), "{out}");

    let (out, status) = run(&desk, &["deed", "trail", &deed]);
    assert_eq!(status, 0, "{out}");
    assert!(out.contains(&deed), "{out}");

    let (out, status) = run(
        &desk,
        &["memory", "remember", "always open the deed, not the chat"],
    );
    assert_eq!(status, 0, "{out}");
    assert!(out.contains("m-"), "{out}");
    assert!(out.contains("always open the deed, not the chat"), "{out}");
}

#[test]
fn second_sitting_opens_the_same_deed_product() {
    let tmp = tempfile::tempdir().unwrap();
    let desk = tmp.path().join("desk");
    let note = tmp.path().join("note.txt");
    fs::write(&note, "same bytes\n").unwrap();
    assert_eq!(run(&desk, &["open"]).1, 0);
    let (out, status) = run(&desk, &["work", "keep"]);
    assert_eq!(status, 0, "{out}");
    let id = first_id(&out, "w-");
    assert_eq!(run(&desk, &["work", "claim", &id, "--as", "alice"]).1, 0);
    let (out, status) = run(
        &desk,
        &["work", "complete", &id, "--file", note.to_str().unwrap()],
    );
    assert_eq!(status, 0, "{out}");
    let deed = first_id(&out, "d-");
    let (first, status) = run(&desk, &["deed", "get", &deed]);
    assert_eq!(status, 0, "{first}");
    let (second, status) = run(&desk, &["deed", "get", &deed]);
    assert_eq!(status, 0, "{second}");
    assert_eq!(first, second);
    assert!(second.contains("same bytes"));
}

#[test]
fn spawn_then_child_claim() {
    let tmp = tempfile::tempdir().unwrap();
    let desk = tmp.path().join("desk");
    let _ = ok(&desk, &["open"]);
    let out = ok(&desk, &["work", "parent"]);
    let parent = first_id(&out, "w-");
    let _ = ok(&desk, &["work", "claim", &parent, "--as", "alice"]);
    let out = ok(&desk, &["work", "spawn", &parent, "child work"]);
    let child = first_id(&out, "w-");
    let out = ok(&desk, &["work", "claim", &child, "--as", "child"]);
    assert!(out.contains("child"));
}

#[test]
fn second_claim_of_the_same_node_fails() {
    let tmp = tempfile::tempdir().unwrap();
    let desk = tmp.path().join("desk");
    let _ = ok(&desk, &["open"]);
    let out = ok(&desk, &["work", "lock"]);
    let id = first_id(&out, "w-");
    let _ = ok(&desk, &["work", "claim", &id, "--as", "alice"]);
    let (out, status) = run(&desk, &["work", "claim", &id, "--as", "bob"]);
    assert_eq!(status, 1);
    assert!(out.contains("not_ready"));
}

#[test]
fn surface_lists_the_sitting_verbs() {
    let tmp = tempfile::tempdir().unwrap();
    let desk = tmp.path().join("desk");
    let out = ok(&desk, &["surface"]);
    for verb in [
        "open", "work", "claim", "spawn", "complete", "deed get", "evidence", "remember", "plan",
        "ready", "status", "mcp", "install", "init", "sit", "search", "link", "memory", "wait",
        "depend", "mint",
    ] {
        assert!(out.contains(verb), "missing {verb} in {out}");
    }
}

#[test]
fn status_prints_desk_ready_tip_last() {
    let tmp = tempfile::tempdir().unwrap();
    let desk = tmp.path().join("desk");
    let out = ok(&desk, &["status"]);
    assert!(out.contains("desk"), "{out}");
    assert!(out.contains("ready"), "{out}");
    assert!(out.contains("tip"), "{out}");
    assert!(out.contains("last"), "{out}");
}

#[test]
fn plan_accepts_a_parent_id() {
    let tmp = tempfile::tempdir().unwrap();
    let desk = tmp.path().join("desk");
    let _ = ok(&desk, &["open"]);
    let out = ok(&desk, &["plan", "catalog of bindable actions"]);
    let parent = first_id(&out, "p-");
    let out = ok(&desk, &["plan", "--parent", &parent, "keys.toml schema"]);
    let child = first_id(&out, "p-");
    assert_ne!(child, parent);
    assert!(out.contains("keys.toml schema"));
    let out = ok(&desk, &["plan", "children", &parent]);
    assert!(out.contains(&child));
}

#[test]
fn init_enables_the_desk_in_a_directory() {
    let tmp = tempfile::tempdir().unwrap();
    let dest = tmp.path().join("proj");
    let xdg = tmp.path().join("xdg");
    fs::create_dir_all(&dest).unwrap();
    let out = ok_home(&xdg, &dest, &["init", "--dir", dest.to_str().unwrap()]);
    assert!(out.contains("init"));
    let grok = fs::read_to_string(dest.join(".grok/config.toml")).unwrap();
    assert!(grok.contains("enabled = true"));
    assert!(grok.contains("saena"));
    let name = fs::read_to_string(dest.join(".saena/project")).unwrap();
    assert_eq!(name.trim(), "proj");
    assert!(!dest.join("desk").exists());
    assert!(xdg.join("saena/desk/layout").is_file());
    assert!(out.contains("project proj"));
}

#[test]
fn install_puts_the_mcp_block_on_stdout() {
    let tmp = tempfile::tempdir().unwrap();
    let desk = tmp.path().join("desk");
    let bindir = tmp.path().join("bin");
    let home = tmp.path().join("home");
    let out = ok(
        &desk,
        &[
            "install",
            "--bin",
            bindir.to_str().unwrap(),
            "--home",
            home.to_str().unwrap(),
            "--source",
            bin().to_str().unwrap(),
        ],
    );
    assert!(out.contains("install"));
    assert!(out.contains("mcpServers"));
    assert!(out.contains("mcp"));
    assert!(bindir.join("saena").is_file());
}

#[test]
fn full_sitting_through_the_saena_command() {
    let tmp = tempfile::tempdir().unwrap();
    let desk = tmp.path().join("desk");
    let note = tmp.path().join("note.txt");
    let child_file = tmp.path().join("child.txt");
    fs::write(&note, "a named note\n").unwrap();
    fs::write(&child_file, "child product\n").unwrap();

    let out = ok(&desk, &["open"]);
    assert!(out.contains("open"));

    let out = ok(&desk, &["work", "name the note"]);
    let parent = first_id(&out, "w-");
    let _ = ok(&desk, &["work", "claim", &parent, "--as", "alice"]);
    let out = ok(&desk, &["work", "spawn", &parent, "child work"]);
    let child = first_id(&out, "w-");
    let _ = ok(&desk, &["work", "claim", &child, "--as", "child"]);
    let out = ok(
        &desk,
        &[
            "work",
            "complete",
            &parent,
            "--file",
            note.to_str().unwrap(),
        ],
    );
    let parent_deed = first_id(&out, "d-");
    let out = ok(
        &desk,
        &[
            "work",
            "complete",
            &child,
            "--file",
            child_file.to_str().unwrap(),
            "--source",
            &parent_deed,
        ],
    );
    let child_deed = first_id(&out, "d-");
    let out = ok(&desk, &["deed", "get", &parent_deed]);
    assert!(out.contains("a named note"));
    let out = ok(&desk, &["deed", "get", &child_deed]);
    assert!(out.contains("child product"));
    let out = ok(&desk, &["deed", "evidence", &parent_deed]);
    assert!(out.contains("ok"));
    let out = ok(&desk, &["deed", "trail", &child_deed]);
    assert!(out.contains(&parent_deed));
    assert!(out.contains(&child_deed));
    let out = ok(
        &desk,
        &["memory", "remember", "always open the deed, not the chat"],
    );
    assert!(first_id(&out, "m-").starts_with("m-"));
    let out = ok(&desk, &["plan", "catalog"]);
    let catalog = first_id(&out, "p-");
    let out = ok(&desk, &["plan", "schema"]);
    let schema = first_id(&out, "p-");
    let out = ok(&desk, &["plan", "block", &schema, &catalog]);
    assert!(out.contains("blocked"));
    let out = ok(&desk, &["plan", "close", &catalog]);
    assert!(out.contains("done"));
    let out = ok(&desk, &["work", "list", "--all"]);
    assert!(out.contains(&format!("{parent}  done")));
    assert!(out.contains(&format!("{child}  done")));
}

#[test]
fn verify_pin_deed_list_and_cards() {
    let tmp = tempfile::tempdir().unwrap();
    let desk = tmp.path().join("desk");
    fs::write(tmp.path().join("USER.md"), "human card\n").unwrap();
    let out = ok(&desk, &["work", "verify"]);
    assert!(out.contains("ok"));
    let a = ok(&desk, &["work", "a"]);
    let b = ok(&desk, &["work", "b"]);
    let aid = first_id(&a, "w-");
    let bid = first_id(&b, "w-");
    ok(&desk, &["work", "link", &aid, &bid]);
    ok(&desk, &["work", "link", &bid, &aid]);
    let out = ok(&desk, &["work", "verify"]);
    assert!(out.contains("cycle"));
    let out = ok(&desk, &["memory", "pin", "review"]);
    assert!(out.contains("review"));
    let out = ok(
        &desk,
        &[
            "deed",
            "quote",
            "--name",
            "NLL",
            "--excerpt",
            "cfg lifetimes",
        ],
    );
    assert!(out.contains("quote"));
    let out = ok(&desk, &["deed", "list"]);
    assert!(out.contains("d-"));
    let out = ok(&desk, &["memory", "cards"]);
    assert!(out.contains("USER.md"));
}

#[test]
fn mcp_stdio_sitting_names_a_deed() {
    use std::io::{BufRead, BufReader, Write};
    use std::process::{Command, Stdio};

    let tmp = tempfile::tempdir().unwrap();
    let desk = tmp.path().join("mcp-desk");
    fs::create_dir_all(&desk).unwrap();
    let note = tmp.path().join("note.txt");
    fs::write(&note, "a named note\n").unwrap();

    let mut child = Command::new(bin())
        .args(["--desk"])
        .arg(&desk)
        .arg("mcp")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let mut stdin = child.stdin.take().unwrap();
    let stdout = child.stdout.take().unwrap();
    let mut lines = BufReader::new(stdout).lines();

    let rpc = |stdin: &mut std::process::ChildStdin,
               lines: &mut std::io::Lines<BufReader<std::process::ChildStdout>>,
               id: u32,
               method: &str,
               params: serde_json::Value|
     -> serde_json::Value {
        let msg = serde_json::json!({"jsonrpc":"2.0","id":id,"method":method,"params":params});
        writeln!(stdin, "{msg}").unwrap();
        let line = lines.next().unwrap().unwrap();
        serde_json::from_str(&line).unwrap()
    };

    let reply = rpc(
        &mut stdin,
        &mut lines,
        1,
        "initialize",
        serde_json::json!({"protocolVersion":"2024-11-05"}),
    );
    assert_eq!(reply["result"]["serverInfo"]["name"], "saena");

    writeln!(
        stdin,
        "{}",
        serde_json::json!({"jsonrpc":"2.0","method":"notifications/initialized"})
    )
    .unwrap();

    let reply = rpc(
        &mut stdin,
        &mut lines,
        2,
        "tools/list",
        serde_json::json!({}),
    );
    let names: Vec<String> = reply["result"]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["name"].as_str().unwrap().to_string())
        .collect();
    for verb in [
        "open", "work", "claim", "spawn", "complete", "get", "evidence", "trail", "remember",
        "plan", "ready", "verify", "pin", "search", "sit", "wait", "depend", "mint",
    ] {
        assert!(names.contains(&verb.to_string()), "missing {verb}");
    }

    let reply = rpc(
        &mut stdin,
        &mut lines,
        3,
        "tools/call",
        serde_json::json!({"name":"work","arguments":{"summary":"name the note"}}),
    );
    let text = reply["result"]["content"][0]["text"].as_str().unwrap();
    let id = first_id(text, "w-");

    let reply = rpc(
        &mut stdin,
        &mut lines,
        4,
        "tools/call",
        serde_json::json!({"name":"claim","arguments":{"id":id,"actor":"alice"}}),
    );
    assert!(reply["result"]["content"][0]["text"]
        .as_str()
        .unwrap()
        .contains("alice"));

    let reply = rpc(
        &mut stdin,
        &mut lines,
        5,
        "tools/call",
        serde_json::json!({"name":"complete","arguments":{"id":id,"file":note}}),
    );
    let text = reply["result"]["content"][0]["text"].as_str().unwrap();
    assert!(text.contains("deed"));
    let deed = first_id(text, "d-");

    let reply = rpc(
        &mut stdin,
        &mut lines,
        6,
        "tools/call",
        serde_json::json!({"name":"get","arguments":{"id":deed}}),
    );
    assert!(reply["result"]["content"][0]["text"]
        .as_str()
        .unwrap()
        .contains("a named note"));

    let reply = rpc(
        &mut stdin,
        &mut lines,
        7,
        "tools/call",
        serde_json::json!({"name":"evidence","arguments":{"id":deed}}),
    );
    assert!(reply["result"]["content"][0]["text"]
        .as_str()
        .unwrap()
        .contains("ok"));

    let _ = child.kill();
    let _ = child.wait();
}

#[test]
fn init_links_a_project_to_the_home_desk() {
    let tmp = tempfile::tempdir().unwrap();
    let xdg = tmp.path().join("xdg");
    let proj = tmp.path().join("widget");
    fs::create_dir_all(&proj).unwrap();
    let out = ok_home(
        &xdg,
        &proj,
        &[
            "init",
            "--dir",
            proj.to_str().unwrap(),
            "--project",
            "widget",
        ],
    );
    assert!(out.contains("project widget"), "{out}");
    assert_eq!(
        fs::read_to_string(proj.join(".saena/project"))
            .unwrap()
            .trim(),
        "widget"
    );
    let status = ok_home(&xdg, &proj, &["status"]);
    assert!(status.contains("project widget"), "{status}");
    assert!(status.contains("saena/desk"), "{status}");
}

#[test]
fn file_in_other_project_mint_depend_wait_consumes_the_deed() {
    let tmp = tempfile::tempdir().unwrap();
    let xdg = tmp.path().join("xdg");
    let alpha = tmp.path().join("alpha");
    let beta = tmp.path().join("beta");
    fs::create_dir_all(&alpha).unwrap();
    fs::create_dir_all(&beta).unwrap();
    let _ = ok_home(
        &xdg,
        &alpha,
        &[
            "init",
            "--dir",
            alpha.to_str().unwrap(),
            "--project",
            "alpha",
        ],
    );
    let _ = ok_home(
        &xdg,
        &beta,
        &["init", "--dir", beta.to_str().unwrap(), "--project", "beta"],
    );

    let out = ok_home(
        &xdg,
        &alpha,
        &["plan", "--project", "beta", "need a list id"],
    );
    let ticket = first_id(&out, "p-");
    assert!(out.contains("need a list id"), "{out}");

    let listed = ok_home(&xdg, &beta, &["plan", "list"]);
    assert!(listed.contains(&ticket), "{listed}");
    assert!(listed.contains("need a list id"), "{listed}");
    let hidden = ok_home(&xdg, &alpha, &["plan", "list"]);
    assert!(!hidden.contains(&ticket), "{hidden}");

    let out = ok_home(&xdg, &beta, &["work", "mint", &ticket]);
    let theirs = first_id(&out, "w-");

    let out = ok_home(&xdg, &alpha, &["work", "continue after list id"]);
    let mine = first_id(&out, "w-");
    let linked = ok_home(&xdg, &alpha, &["work", "depend", &mine, &theirs]);
    assert!(linked.contains(&theirs), "{linked}");

    let (claim, status) = run_home(&xdg, &alpha, &["work", "claim", &mine, "--as", "alice"]);
    assert_eq!(status, 1, "{claim}");
    assert!(claim.contains("not_ready"), "{claim}");

    let ready_b = ok_home(&xdg, &beta, &["work", "ready"]);
    assert!(ready_b.contains(&theirs), "{ready_b}");
    let ready_a = ok_home(&xdg, &alpha, &["work", "ready"]);
    assert!(!ready_a.contains(&theirs), "{ready_a}");
    assert!(!ready_a.contains(&mine), "{ready_a}");

    let _ = ok_home(&xdg, &beta, &["work", "claim", &theirs, "--as", "bob"]);
    let product = beta.join("list.rs");
    fs::write(&product, "pub struct ListId;\n").unwrap();
    let out = ok_home(
        &xdg,
        &beta,
        &[
            "work",
            "complete",
            &theirs,
            "--file",
            product.to_str().unwrap(),
        ],
    );
    let deed = first_id(&out, "d-");

    let waited = ok_home(&xdg, &alpha, &["wait", &theirs]);
    assert!(waited.contains("done"), "{waited}");
    assert!(waited.contains(&deed), "{waited}");

    let claimed = ok_home(&xdg, &alpha, &["work", "claim", &mine, "--as", "alice"]);
    assert!(claimed.contains("alice"), "{claimed}");
    let got = ok_home(&xdg, &alpha, &["deed", "get", &deed]);
    assert!(got.contains("pub struct ListId"), "{got}");
}

#[test]
fn wait_on_open_work_times_out() {
    let tmp = tempfile::tempdir().unwrap();
    let xdg = tmp.path().join("xdg");
    let proj = tmp.path().join("solo");
    fs::create_dir_all(&proj).unwrap();
    let _ = ok_home(
        &xdg,
        &proj,
        &["init", "--dir", proj.to_str().unwrap(), "--project", "solo"],
    );
    let out = ok_home(&xdg, &proj, &["work", "still open"]);
    let id = first_id(&out, "w-");
    let (text, status) = run_home(&xdg, &proj, &["wait", &id, "--timeout-ms", "0"]);
    assert_eq!(status, 1, "{text}");
    assert!(text.contains("timeout"), "{text}");
}
