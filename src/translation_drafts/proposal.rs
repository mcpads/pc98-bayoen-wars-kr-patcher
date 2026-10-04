use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use anyhow::{Context, Result, ensure};
use serde::Deserialize;

use super::model::{ProposalBatch, ProtectedWorkspace, ProtectedWorkspaceIndex};
use crate::source_disk::sha256_hex;

#[derive(Deserialize)]
struct ProtectedSegmentDocument {
    entries: Vec<ProtectedEntryIdentity>,
}

#[derive(Deserialize)]
struct ProtectedEntryIdentity {
    id: String,
    text: Option<String>,
}

pub(super) fn read_protected_workspace(root: &Path) -> Result<ProtectedWorkspace> {
    let index_path = root.join("index.json");
    let index: ProtectedWorkspaceIndex = serde_json::from_slice(
        &fs::read(&index_path)
            .with_context(|| format!("failed to read {}", index_path.display()))?,
    )?;
    ensure!(
        index.translation_segmentation_complete,
        "protected workspace still has unsegmented translation targets"
    );
    let mut entry_ids_by_segment = BTreeMap::new();
    let mut control_only_entry_ids = BTreeSet::new();
    for segment in &index.segments {
        let path = root.join(&segment.path);
        let bytes =
            fs::read(&path).with_context(|| format!("failed to read {}", path.display()))?;
        ensure!(
            sha256_hex(&bytes) == segment.content_sha256,
            "protected segment {} content hash differs from its index",
            segment.id
        );
        let document: ProtectedSegmentDocument = serde_json::from_slice(&bytes)?;
        ensure!(
            document.entries.len() == segment.entry_count,
            "protected segment {} entry count differs from its index",
            segment.id
        );
        let ids = document
            .entries
            .into_iter()
            .map(|entry| {
                if entry
                    .text
                    .as_deref()
                    .is_some_and(is_control_only_source_text)
                {
                    control_only_entry_ids.insert(entry.id.clone());
                }
                entry.id
            })
            .collect::<Vec<_>>();
        let unique = ids.iter().collect::<BTreeSet<_>>();
        ensure!(
            unique.len() == ids.len(),
            "protected segment {} has duplicate entry IDs",
            segment.id
        );
        entry_ids_by_segment.insert(segment.id.clone(), ids);
    }
    Ok(ProtectedWorkspace {
        root: root.to_path_buf(),
        index,
        entry_ids_by_segment,
        control_only_entry_ids,
    })
}

fn is_control_only_source_text(text: &str) -> bool {
    let mut characters = text.chars().peekable();
    while let Some(character) = characters.next() {
        if character == '\u{1b}' && characters.peek() == Some(&'[') {
            characters.next();
            for parameter in characters.by_ref() {
                if ('\u{40}'..='\u{7e}').contains(&parameter) {
                    break;
                }
            }
        } else if !character.is_control() && !character.is_whitespace() {
            return false;
        }
    }
    true
}

pub(super) fn read_reviewed_proposals(directory: &Path) -> Result<Vec<ProposalBatch>> {
    let mut paths = fs::read_dir(directory)
        .with_context(|| format!("failed to read proposal directory {}", directory.display()))?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<std::io::Result<Vec<_>>>()?;
    paths.retain(|path| {
        path.extension()
            .is_some_and(|extension| extension == "json")
    });
    paths.sort();
    ensure!(!paths.is_empty(), "reviewed proposal directory is empty");
    paths
        .into_iter()
        .map(|path| {
            serde_json::from_slice(
                &fs::read(&path)
                    .with_context(|| format!("failed to read proposal {}", path.display()))?,
            )
            .with_context(|| format!("invalid proposal {}", path.display()))
        })
        .collect()
}

#[cfg(test)]
#[path = "proposal_tests.rs"]
mod proposal_tests;
