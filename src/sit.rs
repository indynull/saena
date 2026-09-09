//! Terminal desk. Faces over the one tree.

use crate::desk::Desk;

pub fn faces() -> &'static [&'static str] {
    &["work", "plan", "deed", "memory", "search"]
}

pub fn dump(desk: &Desk) -> String {
    format!(
        "{}\n{}\n{}\n{}\n{}\n{}",
        header(desk),
        work_face(desk),
        plan_face(desk),
        deed_face(desk),
        memory_face(desk),
        search_face()
    )
}

fn header(desk: &Desk) -> String {
    let glance = desk.status();
    format!(
        "saena sit  project {}  gen {}  tip {}\n",
        desk.project.as_deref().unwrap_or("(none)"),
        glance.gen,
        glance.tip.unwrap_or_else(|| "(none)".into())
    )
}

fn work_face(desk: &Desk) -> String {
    let rows = desk.work_list(false, true, false);
    let body = if rows.is_empty() {
        "  (none)".into()
    } else {
        rows.iter()
            .map(|w| format!("  {}  {}  {}", w.id, w.status.token(), w.summary))
            .collect::<Vec<_>>()
            .join("\n")
    };
    format!("face work\n{body}\n")
}

fn plan_face(desk: &Desk) -> String {
    let ready = desk.plan_ready();
    let listed = desk.plan_list();
    let claims = desk.claims();
    let agenda = desk.agenda();
    format!(
        "face plan\n  Ready\n{}\n  List\n{}\n  Claims\n{}\n  Agenda\n{}\n  Search\n  (query on this face)\n",
        pane_ids(&ready),
        pane_ids(&listed),
        pane_work(&claims),
        pane_ids(&agenda),
    )
}

fn deed_face(desk: &Desk) -> String {
    let glance = desk.status();
    let body = match glance.tip.as_deref().and_then(|id| desk.deed(id).ok()) {
        Some(deed) => {
            let mut line = format!(
                "  {}  {}  {}  {}",
                deed.id,
                deed.deed_kind.as_deref().unwrap_or("file"),
                deed.face.as_deref().unwrap_or("document"),
                deed.summary
            );
            if let Some(excerpt) = deed.extra.get("excerpt") {
                line.push('\n');
                line.push_str("  ");
                line.push_str(excerpt);
            }
            line
        }
        None => "  (none)".into(),
    };
    format!("face deed\n{body}\n")
}

fn memory_face(desk: &Desk) -> String {
    let atoms = desk.memory();
    let body = if atoms.is_empty() {
        "  (none)".into()
    } else {
        atoms
            .iter()
            .map(|a| format!("  {}  {}", a.id, a.text))
            .collect::<Vec<_>>()
            .join("\n")
    };
    format!("face memory\n{body}\n")
}

fn search_face() -> String {
    "face search\n  (query the tree)\n".into()
}

fn pane_ids(nodes: &[crate::node::Node]) -> String {
    if nodes.is_empty() {
        "  (none)".into()
    } else {
        nodes
            .iter()
            .map(|n| format!("  {}  {}", n.id, n.summary))
            .collect::<Vec<_>>()
            .join("\n")
    }
}

fn pane_work(nodes: &[crate::node::Node]) -> String {
    if nodes.is_empty() {
        "  (none)".into()
    } else {
        nodes
            .iter()
            .map(|n| format!("  {}  {}", n.id, n.summary))
            .collect::<Vec<_>>()
            .join("\n")
    }
}
