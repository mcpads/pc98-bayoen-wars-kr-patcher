use std::collections::BTreeMap;

use anyhow::{Result, bail};
use serde::Serialize;

use super::review::review_asset;
use crate::source_disk::sha256_hex;

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AssetOrigin {
    InstallerPayload,
    RequiredRuntime,
    PreservedBoot,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AssetReviewStatus {
    ConfirmedLocalizationTarget,
    EvidenceBasedExclusion,
    Unresolved,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct LocalizationAssetFile {
    pub name: String,
    pub origin: AssetOrigin,
    pub size: usize,
    pub sha256: String,
    pub review_status: AssetReviewStatus,
    pub evidence: Vec<String>,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct LocalizationAssetCatalog {
    pub total_files: usize,
    pub confirmed_localization_targets: usize,
    pub evidence_based_exclusions: usize,
    pub unresolved: usize,
    pub files: Vec<LocalizationAssetFile>,
}

pub(crate) fn catalog_localization_assets(
    installer_payload: &BTreeMap<String, Vec<u8>>,
    runtime_files: &BTreeMap<String, Vec<u8>>,
    boot_files: &BTreeMap<String, Vec<u8>>,
) -> Result<LocalizationAssetCatalog> {
    let mut files = BTreeMap::new();
    insert_files(&mut files, installer_payload, AssetOrigin::InstallerPayload)?;
    insert_files(&mut files, runtime_files, AssetOrigin::RequiredRuntime)?;
    insert_files(&mut files, boot_files, AssetOrigin::PreservedBoot)?;

    let files: Vec<_> = files.into_values().collect();
    let confirmed_localization_targets =
        count_status(&files, AssetReviewStatus::ConfirmedLocalizationTarget);
    let evidence_based_exclusions = count_status(&files, AssetReviewStatus::EvidenceBasedExclusion);
    let unresolved = count_status(&files, AssetReviewStatus::Unresolved);

    Ok(LocalizationAssetCatalog {
        total_files: files.len(),
        confirmed_localization_targets,
        evidence_based_exclusions,
        unresolved,
        files,
    })
}

fn insert_files(
    destination: &mut BTreeMap<String, LocalizationAssetFile>,
    source: &BTreeMap<String, Vec<u8>>,
    origin: AssetOrigin,
) -> Result<()> {
    for (name, bytes) in source {
        let review = review_asset(name);
        let record = LocalizationAssetFile {
            name: name.clone(),
            origin,
            size: bytes.len(),
            sha256: sha256_hex(bytes),
            review_status: review.status,
            evidence: review.evidence,
        };
        if destination.insert(name.clone(), record).is_some() {
            bail!("localization asset population contains duplicate file name: {name}");
        }
    }
    Ok(())
}

fn count_status(files: &[LocalizationAssetFile], status: AssetReviewStatus) -> usize {
    files
        .iter()
        .filter(|file| file.review_status == status)
        .count()
}

#[cfg(test)]
#[path = "file_population_tests.rs"]
mod file_population_tests;
