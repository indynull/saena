//! Closed atom schema. No I/O.

pub const SCHEMA: &str = "saena.atom/v1";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Atom {
    pub kind: String,
    pub text: String,
    pub field: String,
    pub timestamp: String,
    pub cite: Option<String>,
}

impl Atom {
    pub fn new(
        kind: impl Into<String>,
        text: impl Into<String>,
        timestamp: impl Into<String>,
    ) -> Self {
        Self {
            kind: kind.into(),
            text: text.into(),
            field: "atom".into(),
            timestamp: timestamp.into(),
            cite: None,
        }
    }
}

pub fn is_live(tombstone: bool, valid_to: Option<&str>, now: &str) -> bool {
    if tombstone {
        return false;
    }
    match valid_to {
        None | Some("") => true,
        Some(until) => until > now,
    }
}

pub fn entity_jaccard<'a, I, J>(left: I, right: J) -> f64
where
    I: IntoIterator<Item = &'a str>,
    J: IntoIterator<Item = &'a str>,
{
    use std::collections::HashSet;
    let a: HashSet<&str> = left.into_iter().collect();
    let b: HashSet<&str> = right.into_iter().collect();
    if a.is_empty() && b.is_empty() {
        return 0.0;
    }
    let inter = a.intersection(&b).count() as f64;
    let union = a.union(&b).count() as f64;
    if union == 0.0 {
        0.0
    } else {
        inter / union
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn live_open_valid_to() {
        assert!(is_live(false, None, "2026-08-20T00:00:00Z"));
        assert!(is_live(
            false,
            Some("2099-01-01T00:00:00Z"),
            "2026-08-20T00:00:00Z"
        ));
        assert!(!is_live(
            false,
            Some("2000-01-01T00:00:00Z"),
            "2026-08-20T00:00:00Z"
        ));
        assert!(!is_live(true, None, "2026-08-20T00:00:00Z"));
    }

    #[test]
    fn jaccard_overlap() {
        let v = entity_jaccard(["grok", "pack"], ["pack", "seat"]);
        assert!((v - 1.0 / 3.0).abs() < 1e-9);
    }
}
