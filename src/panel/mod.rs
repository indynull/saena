//! Host search merge: named fuse, then diversify, then decay.

#![allow(clippy::needless_range_loop)]

pub mod borda;
pub mod comb;
pub mod copeland;
pub mod decay;
pub mod dowdall;
pub mod dpp;
pub mod kemeny;
pub mod mmr;
pub mod rrf;
pub mod schulze;
pub mod tideman;

use std::collections::HashMap;
use std::env;
use std::hash::Hash;

use self::borda::{borda_merge, Ballot};
use self::comb::{combmnz_merge, combsum_merge, ScoredBallot};
use self::copeland::copeland_merge;
use self::decay::temporal_decay;
use self::dowdall::dowdall_merge;
use self::dpp::dpp_rerank;
use self::kemeny::kemeny_merge;
use self::mmr::{mmr_rerank, Ranked};
use self::rrf::rrf_merge;
use self::schulze::schulze_merge;
use self::tideman::ranked_pairs_merge;

const DECAY_HALF_LIFE_DAYS: f64 = 14.0;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Fuse {
    #[default]
    Borda,
    Rrf,
    CombSum,
    CombMnz,
    Dowdall,
    Kemeny,
    Schulze,
    Copeland,
    Tideman,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Diversify {
    #[default]
    Mmr,
    Dpp,
    None,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Decay {
    #[default]
    Off,
    On,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Panel {
    pub fuse: Fuse,
    pub diversify: Diversify,
    pub decay: Decay,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UnknownVoter {
    Fuse(String),
    Diversify(String),
    Decay(String),
}

impl std::fmt::Display for UnknownVoter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Fuse(n) => write!(f, "unknown fuse `{n}`"),
            Self::Diversify(n) => write!(f, "unknown diversify `{n}`"),
            Self::Decay(n) => write!(f, "unknown decay `{n}`"),
        }
    }
}

impl std::error::Error for UnknownVoter {}

impl Fuse {
    pub fn parse(name: &str) -> Result<Self, UnknownVoter> {
        match name {
            "borda" => Ok(Self::Borda),
            "rrf" => Ok(Self::Rrf),
            "combsum" => Ok(Self::CombSum),
            "combmnz" => Ok(Self::CombMnz),
            "dowdall" => Ok(Self::Dowdall),
            "kemeny" => Ok(Self::Kemeny),
            "schulze" => Ok(Self::Schulze),
            "copeland" => Ok(Self::Copeland),
            "tideman" => Ok(Self::Tideman),
            other => Err(UnknownVoter::Fuse(other.to_string())),
        }
    }
}

impl Diversify {
    pub fn parse(name: &str) -> Result<Self, UnknownVoter> {
        match name {
            "mmr" => Ok(Self::Mmr),
            "dpp" => Ok(Self::Dpp),
            "none" => Ok(Self::None),
            other => Err(UnknownVoter::Diversify(other.to_string())),
        }
    }
}

impl Decay {
    pub fn parse(name: &str) -> Result<Self, UnknownVoter> {
        match name {
            "off" => Ok(Self::Off),
            "on" => Ok(Self::On),
            other => Err(UnknownVoter::Decay(other.to_string())),
        }
    }
}

impl Default for Panel {
    fn default() -> Self {
        Self {
            fuse: Fuse::Borda,
            diversify: Diversify::Mmr,
            decay: Decay::Off,
        }
    }
}

impl Panel {
    pub fn named(fuse: &str, diversify: &str, decay: &str) -> Result<Self, UnknownVoter> {
        Ok(Self {
            fuse: Fuse::parse(fuse)?,
            diversify: Diversify::parse(diversify)?,
            decay: Decay::parse(decay)?,
        })
    }

    pub fn from_env() -> Result<Self, UnknownVoter> {
        fn slot(
            raw: Result<String, env::VarError>,
            default: &'static str,
            empty: fn(String) -> UnknownVoter,
        ) -> Result<String, UnknownVoter> {
            match raw {
                Err(env::VarError::NotPresent) => Ok(default.to_string()),
                Ok(s) if s.is_empty() => Err(empty(String::new())),
                Ok(s) => Ok(s),
                Err(_) => Ok(default.to_string()),
            }
        }
        let fuse = slot(env::var("SAENA_FUSE"), "borda", UnknownVoter::Fuse)?;
        let diversify = slot(env::var("SAENA_DIVERSIFY"), "mmr", UnknownVoter::Diversify)?;
        let decay = slot(env::var("SAENA_DECAY"), "off", UnknownVoter::Decay)?;
        Self::named(&fuse, &diversify, &decay)
    }

    pub fn fuse_merge<T>(&self, ballots: &[Ballot<T>], k: usize) -> Vec<T>
    where
        T: Clone + Eq + Hash,
    {
        match self.fuse {
            Fuse::Borda => borda_merge(ballots, k),
            Fuse::Rrf => {
                let mut out = rrf_merge(ballots, 60);
                out.truncate(k);
                out
            }
            Fuse::CombSum | Fuse::CombMnz => {
                let scored = ranks_as_scored(ballots);
                let mut out = if self.fuse == Fuse::CombSum {
                    combsum_merge(&scored)
                } else {
                    combmnz_merge(&scored)
                };
                out.truncate(k);
                out
            }
            Fuse::Dowdall => dowdall_merge(ballots, k),
            Fuse::Kemeny => kemeny_merge(ballots, k),
            Fuse::Schulze => schulze_merge(ballots, k),
            Fuse::Copeland => copeland_merge(ballots, k),
            Fuse::Tideman => ranked_pairs_merge(ballots, k),
        }
    }

    pub fn rerank(&self, items: &[Ranked], lambda: f64) -> Vec<String> {
        match self.diversify {
            Diversify::Mmr => mmr_rerank(items, lambda),
            Diversify::Dpp => dpp_rerank(items, items.len()),
            Diversify::None => items.iter().map(|i| i.id.clone()).collect(),
        }
    }

    pub fn decay_weight(&self, source: &str, age_days: f64) -> f64 {
        match self.decay {
            Decay::Off => 1.0,
            Decay::On => temporal_decay(source, age_days, Some(DECAY_HALF_LIFE_DAYS)),
        }
    }
}

fn ranks_as_scored<T: Clone>(ballots: &[Ballot<T>]) -> Vec<ScoredBallot<T>> {
    ballots
        .iter()
        .map(|ballot| {
            let n = ballot.len() as f64;
            ballot
                .iter()
                .enumerate()
                .map(|(pos, id)| (id.clone(), n - pos as f64))
                .collect()
        })
        .collect()
}

#[derive(Clone, Debug)]
pub struct Hit {
    pub id: String,
    pub record: String,
    pub field: String,
    pub kind: String,
    pub text: String,
    pub score: f64,
}

pub fn merge_hits(hits: Vec<Hit>) -> Vec<Hit> {
    let panel = Panel::from_env().unwrap_or_default();
    if hits.is_empty() {
        return hits;
    }
    let ids: Vec<String> = hits.iter().map(|h| h.id.clone()).collect();
    let fused = panel.fuse_merge(&[ids], hits.len().max(1));
    let by_id: HashMap<_, _> = hits.into_iter().map(|h| (h.id.clone(), h)).collect();
    let items: Vec<Ranked> = fused
        .iter()
        .filter_map(|id| {
            by_id.get(id).map(|hit| Ranked {
                id: id.clone(),
                rel: hit.score * panel.decay_weight("session", 0.0),
                tokens: tokens(&hit.text),
            })
        })
        .collect();
    panel
        .rerank(&items, 0.7)
        .into_iter()
        .filter_map(|id| by_id.get(&id).cloned())
        .collect()
}

fn tokens(text: &str) -> std::collections::HashSet<String> {
    text.to_ascii_lowercase()
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|w| w.len() >= 3)
        .map(str::to_string)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn named_voters_parse() {
        for fuse in [
            "borda", "rrf", "combsum", "combmnz", "dowdall", "kemeny", "schulze", "copeland",
            "tideman",
        ] {
            assert!(Panel::named(fuse, "mmr", "off").is_ok(), "{fuse}");
        }
        assert!(Panel::named("nope", "mmr", "off").is_err());
        assert!(Panel::named("borda", "dpp", "on").is_ok());
        assert!(Panel::named("borda", "none", "off").is_ok());
        assert!(Panel::named("borda", "zzz", "off").is_err());
        assert!(Panel::named("borda", "mmr", "maybe").is_err());
    }

    #[test]
    fn borda_then_mmr_on_fixed_ballot() {
        let panel = Panel::named("borda", "none", "off").unwrap();
        let ballots = [vec!["a", "b", "c"], vec!["a", "c", "b"]];
        assert_eq!(panel.fuse_merge(&ballots, 3), vec!["a", "b", "c"]);
    }
}
