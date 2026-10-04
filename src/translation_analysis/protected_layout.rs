use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use anyhow::{Context, Result, ensure};
use serde::Deserialize;
use serde_json::Value;

use super::TranslationCorpus;
use crate::source_disk::sha256_hex;
use crate::translation_drafts::TranslationSurface;

#[derive(Debug)]
pub(super) struct ProtectedLayoutCatalog {
    segments: BTreeMap<String, ProtectedLayoutSegment>,
}

#[derive(Debug)]
pub(super) struct ProtectedLayoutSegment {
    pub id: String,
    pub surface: TranslationSurface,
    pub entries: Vec<Value>,
}

#[derive(Deserialize)]
struct ProtectedWorkspaceIndex {
    supported_source_sha256: String,
    segments: Vec<ProtectedSegmentReference>,
}

#[derive(Deserialize)]
struct ProtectedSegmentReference {
    id: String,
    surface: TranslationSurface,
    path: String,
    entry_count: usize,
    content_sha256: String,
}

#[derive(Deserialize)]
struct ProtectedSegmentDocument {
    id: String,
    surface: TranslationSurface,
    entries: Vec<Value>,
}

impl ProtectedLayoutCatalog {
    pub fn segment(&self, id: &str) -> &ProtectedLayoutSegment {
        &self.segments[id]
    }
}

pub(super) fn load_protected_layout_catalog(
    protected_directory: &Path,
    corpus: &TranslationCorpus,
) -> Result<ProtectedLayoutCatalog> {
    let index_path = protected_directory.join("index.json");
    let index_bytes = fs::read(&index_path)
        .with_context(|| format!("failed to read {}", index_path.display()))?;
    ensure!(
        sha256_hex(&index_bytes) == corpus.index.protected_index_sha256,
        "protected workspace index differs from the reviewed translation corpus"
    );
    let index: ProtectedWorkspaceIndex = serde_json::from_slice(&index_bytes)?;
    ensure!(
        index.supported_source_sha256 == corpus.index.supported_source_sha256,
        "protected workspace and reviewed translation corpus use different source media"
    );
    let expected_ids = corpus
        .index
        .segments
        .iter()
        .map(|segment| segment.id.as_str())
        .chain(
            corpus
                .index
                .producer_evidence_segments
                .iter()
                .map(String::as_str),
        )
        .collect::<BTreeSet<_>>();
    let indexed_ids = index
        .segments
        .iter()
        .map(|segment| segment.id.as_str())
        .collect::<BTreeSet<_>>();
    ensure!(
        indexed_ids.len() == index.segments.len() && indexed_ids == expected_ids,
        "protected workspace segment population differs from the reviewed translation corpus"
    );

    let draft_references = corpus
        .index
        .segments
        .iter()
        .map(|segment| (segment.id.as_str(), segment))
        .collect::<BTreeMap<_, _>>();
    let mut segments = BTreeMap::new();
    for reference in index
        .segments
        .iter()
        .filter(|reference| draft_references.contains_key(reference.id.as_str()))
    {
        let draft_reference = draft_references[reference.id.as_str()];
        ensure!(
            reference.surface == draft_reference.surface
                && reference.entry_count == draft_reference.entry_count
                && reference.content_sha256 == draft_reference.protected_segment_sha256,
            "protected layout segment {} differs from its reviewed reference",
            reference.id
        );
        let path = protected_directory.join(&reference.path);
        let bytes =
            fs::read(&path).with_context(|| format!("failed to read {}", path.display()))?;
        ensure!(
            sha256_hex(&bytes) == reference.content_sha256,
            "protected layout segment {} content hash changed",
            reference.id
        );
        let document: ProtectedSegmentDocument = serde_json::from_slice(&bytes)?;
        ensure!(
            document.id == reference.id
                && document.surface == reference.surface
                && document.entries.len() == reference.entry_count,
            "protected layout segment {} document identity changed",
            reference.id
        );
        let protected_ids = document
            .entries
            .iter()
            .map(entry_id)
            .collect::<Result<Vec<_>>>()?;
        let draft_segment = corpus
            .segments
            .iter()
            .find(|segment| segment.id == reference.id)
            .expect("draft reference and segment were validated together");
        ensure!(
            protected_ids
                == draft_segment
                    .entries
                    .iter()
                    .map(|entry| entry.id.as_str())
                    .collect::<Vec<_>>(),
            "protected layout segment {} entry order differs from the reviewed corpus",
            reference.id
        );
        let segment = ProtectedLayoutSegment {
            id: document.id,
            surface: document.surface,
            entries: document.entries,
        };
        ensure!(
            segments.insert(segment.id.clone(), segment).is_none(),
            "protected layout catalog duplicates segment {}",
            reference.id
        );
    }
    Ok(ProtectedLayoutCatalog { segments })
}

pub(super) fn entry_id(entry: &Value) -> Result<&str> {
    entry
        .get("id")
        .and_then(Value::as_str)
        .context("protected layout entry has no string ID")
}

#[cfg(test)]
#[path = "protected_layout_tests.rs"]
mod protected_layout_tests;
