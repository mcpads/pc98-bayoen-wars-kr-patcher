use std::collections::{BTreeMap, BTreeSet};

use anyhow::{Result, bail, ensure};

use super::model::{SourceSegmentReference, TargetFileCoverage, TargetFileDisposition};
use crate::{AssetReviewStatus, LocalizationAssetCatalog};

const FONT_PRODUCER: &str = "GAIJI.COM";
const NEEDS_SEGMENTATION: [&str; 6] = [
    "TITLE.DAT",
    "SEL1.DAT",
    "SEL3.DAT",
    "OPM.DAT",
    "EDM.DAT",
    "SAMPA",
];

pub(super) fn catalog_target_file_coverage(
    assets: &LocalizationAssetCatalog,
    segments: &[SourceSegmentReference],
) -> Result<Vec<TargetFileCoverage>> {
    ensure!(
        assets.unresolved == 0,
        "translation workspace requires a resolved localization file population"
    );
    let mut segments_by_file = BTreeMap::<String, Vec<String>>::new();
    for segment in segments {
        segments_by_file
            .entry(segment.source_file.clone())
            .or_default()
            .push(segment.id.clone());
    }

    let target_names = assets
        .files
        .iter()
        .filter(|file| file.review_status == AssetReviewStatus::ConfirmedLocalizationTarget)
        .map(|file| file.name.as_str())
        .collect::<BTreeSet<_>>();
    ensure!(
        segments_by_file
            .keys()
            .all(|name| target_names.contains(name.as_str())),
        "translation workspace segment references a file outside the target population"
    );

    let mut coverage = Vec::with_capacity(target_names.len());
    for file in assets
        .files
        .iter()
        .filter(|file| file.review_status == AssetReviewStatus::ConfirmedLocalizationTarget)
    {
        let segment_ids = segments_by_file.remove(&file.name).unwrap_or_default();
        let disposition = if !segment_ids.is_empty() {
            TargetFileDisposition::SourceUnitsExtracted
        } else if file.name == FONT_PRODUCER {
            TargetFileDisposition::FontProducer
        } else if NEEDS_SEGMENTATION.contains(&file.name.as_str()) {
            TargetFileDisposition::NeedsSourceSegmentation
        } else {
            bail!(
                "localization target {} has no translation workspace disposition",
                file.name
            );
        };
        coverage.push(TargetFileCoverage {
            source_file: file.name.clone(),
            disposition,
            segment_ids,
            evidence: file.evidence.clone(),
        });
    }
    ensure!(
        segments_by_file.is_empty(),
        "translation workspace did not reconcile every source segment"
    );
    ensure!(
        coverage.len() == assets.confirmed_localization_targets,
        "translation workspace target coverage differs from the localization population"
    );
    Ok(coverage)
}

#[cfg(test)]
#[path = "population_tests.rs"]
mod population_tests;
