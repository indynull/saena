//! milli projection over the tree.

use anyhow::{anyhow, Result};
use milli::documents::{DocumentsBatchBuilder, DocumentsBatchReader};
use milli::heed::EnvOpenOptions;
use milli::update::{
    ClearDocuments, IndexDocuments, IndexDocumentsConfig, IndexerConfig, Settings,
};
use milli::{Index, Search, SearchResult};
use serde_json::{Map, Value};
use std::collections::HashSet;
use std::io::Cursor;
use std::path::Path;

use crate::node::Node;
use crate::panel::Hit;

const MAP_SIZE: usize = 1024 * 1024 * 1024;
const SEARCHABLE: &[&str] = &["text", "summary", "kind", "id"];
const FILTERABLE: &[&str] = &["record", "kind", "status", "field", "pin"];
const DISPLAYED: &[&str] = &[
    "id", "record", "kind", "status", "text", "summary", "field", "pin",
];

pub fn reindex(root: &Path, nodes: impl IntoIterator<Item = Node>) -> Result<()> {
    let path = root.join("search.milli");
    std::fs::create_dir_all(&path)?;
    let docs: Vec<Map<String, Value>> = nodes
        .into_iter()
        .map(|n| {
            let text = if n.text.is_empty() {
                format!("{} {}", n.summary, n.body)
            } else {
                n.text.clone()
            };
            let mut map = Map::new();
            map.insert("id".into(), Value::String(n.id.clone()));
            map.insert("record".into(), Value::String(n.kind.token().into()));
            map.insert(
                "kind".into(),
                Value::String(
                    n.atom_kind
                        .clone()
                        .or(n.deed_kind.clone())
                        .or(n.work_kind.clone())
                        .unwrap_or_else(|| n.kind.token().into()),
                ),
            );
            map.insert("status".into(), Value::String(n.status.token().into()));
            map.insert("text".into(), Value::String(text));
            map.insert("summary".into(), Value::String(n.summary.clone()));
            map.insert(
                "field".into(),
                Value::String(n.field.clone().unwrap_or_else(|| n.kind.token().into())),
            );
            map.insert(
                "pin".into(),
                Value::String(n.pin.clone().unwrap_or_default()),
            );
            map
        })
        .collect();
    index_documents(&path, docs)
}

pub fn query(root: &Path, q: &str, pin: Option<&str>, limit: usize) -> Result<Vec<Hit>> {
    let path = root.join("search.milli");
    if !path.exists() {
        return Ok(Vec::new());
    }
    let index = open_index(&path)?;
    let rtxn = match index.read_txn() {
        Ok(txn) => txn,
        Err(_) => return Ok(Vec::new()),
    };
    let mut search = Search::new(&rtxn, &index);
    if !q.is_empty() {
        search.query(q);
    }
    search.limit(limit.max(1));
    search.authorize_typos(true);
    let clause = pin
        .filter(|p| !p.is_empty())
        .map(|p| format!("pin = \"{}\"", p.replace('"', "\\\"")));
    if let Some(text) = clause.as_deref() {
        if let Some(filter) = milli::Filter::from_str(text).map_err(|err| anyhow!("{err}"))? {
            search.filter(filter);
        }
    }
    let SearchResult { documents_ids, .. } = match search.execute() {
        Ok(result) => result,
        Err(_) => return Ok(Vec::new()),
    };
    let fields = index
        .fields_ids_map(&rtxn)
        .map_err(|err| anyhow!("{err}"))?;
    let displayed: Vec<_> = match index
        .displayed_fields_ids(&rtxn)
        .map_err(|err| anyhow!("{err}"))?
    {
        Some(ids) => ids,
        None => fields.iter().map(|(id, _)| id).collect(),
    };
    let docs = index
        .documents(&rtxn, documents_ids.clone())
        .map_err(|err| anyhow!("{err}"))?;
    let n = documents_ids.len().max(1) as f64;
    let mut hits = Vec::new();
    let mut seen = HashSet::new();
    for (rank, (_docid, obkv)) in docs.into_iter().enumerate() {
        let obj = milli::obkv_to_json(&displayed, &fields, obkv).map_err(|err| anyhow!("{err}"))?;
        let id = obj
            .get("id")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_owned();
        if id.is_empty() || !seen.insert(id.clone()) {
            continue;
        }
        hits.push(Hit {
            id,
            record: obj
                .get("record")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_owned(),
            field: obj
                .get("field")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_owned(),
            kind: obj
                .get("kind")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_owned(),
            text: obj
                .get("text")
                .and_then(Value::as_str)
                .or_else(|| obj.get("summary").and_then(Value::as_str))
                .unwrap_or("")
                .to_owned(),
            score: (n - rank as f64) / n,
        });
    }
    Ok(hits)
}

fn open_index(path: &Path) -> Result<Index> {
    std::fs::create_dir_all(path)?;
    let mut options = EnvOpenOptions::new();
    options.map_size(MAP_SIZE);
    options.max_dbs(30);
    Index::new(options, path).map_err(|err| anyhow!("milli open: {err}"))
}

fn apply_settings(index: &Index) -> Result<()> {
    let config = IndexerConfig::default();
    let needs_primary = {
        let rtxn = index.read_txn().map_err(|err| anyhow!(err.to_string()))?;
        index
            .primary_key(&rtxn)
            .map_err(|err| anyhow!(err.to_string()))?
            .is_none()
    };
    let mut wtxn = index.write_txn().map_err(|err| anyhow!(err.to_string()))?;
    let mut settings = Settings::new(&mut wtxn, index, &config);
    if needs_primary {
        settings.set_primary_key("id".to_owned());
    }
    settings.set_searchable_fields(SEARCHABLE.iter().map(|s| (*s).to_owned()).collect());
    settings.set_displayed_fields(DISPLAYED.iter().map(|s| (*s).to_owned()).collect());
    settings.set_filterable_fields(FILTERABLE.iter().map(|s| (*s).to_owned()).collect());
    settings.set_min_word_len_one_typo(4);
    settings.set_autorize_typos(true);
    settings
        .execute(|_| (), || false)
        .map_err(|err| anyhow!(err.to_string()))?;
    wtxn.commit().map_err(|err| anyhow!(err.to_string()))?;
    Ok(())
}

fn documents_reader(
    docs: Vec<Map<String, Value>>,
) -> Result<DocumentsBatchReader<Cursor<Vec<u8>>>> {
    let mut builder = DocumentsBatchBuilder::new(Vec::new());
    for doc in docs {
        builder
            .append_json_object(&doc)
            .map_err(|err| anyhow!(err.to_string()))?;
    }
    let bytes = builder
        .into_inner()
        .map_err(|err| anyhow!(err.to_string()))?;
    DocumentsBatchReader::from_reader(Cursor::new(bytes)).map_err(|err| anyhow!(err.to_string()))
}

fn index_documents(path: &Path, docs: Vec<Map<String, Value>>) -> Result<()> {
    let index = open_index(path)?;
    apply_settings(&index)?;
    {
        let mut wtxn = index.write_txn().map_err(|err| anyhow!(err.to_string()))?;
        ClearDocuments::new(&mut wtxn, &index)
            .execute()
            .map_err(|err| anyhow!(err.to_string()))?;
        wtxn.commit().map_err(|err| anyhow!(err.to_string()))?;
    }
    if docs.is_empty() {
        return Ok(());
    }
    let reader = documents_reader(docs)?;
    let config = IndexerConfig::default();
    let indexing = IndexDocumentsConfig::default();
    let mut wtxn = index.write_txn().map_err(|err| anyhow!(err.to_string()))?;
    let builder = IndexDocuments::new(&mut wtxn, &index, &config, indexing, |_| (), || false)
        .map_err(|err| anyhow!(err.to_string()))?;
    let (builder, added) = builder
        .add_documents(reader)
        .map_err(|err| anyhow!(err.to_string()))?;
    added.map_err(|err| anyhow!(err.to_string()))?;
    builder.execute().map_err(|err| anyhow!(err.to_string()))?;
    wtxn.commit().map_err(|err| anyhow!(err.to_string()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::node::Node;

    #[test]
    fn reindex_empty_dir() {
        let dir = tempfile::tempdir().unwrap();
        let atom = Node::atom("m-1".into(), "lesson", "zircon pin review set".into());
        reindex(dir.path(), [atom]).expect("reindex");
        let hits = query(dir.path(), "zircon", None, 10).expect("query");
        assert!(
            hits.iter().any(|h| h.id == "m-1"),
            "{:?}",
            hits.iter().map(|h| h.id.clone()).collect::<Vec<_>>()
        );
    }
}
