use std::collections::{BTreeMap, BTreeSet};

use anyhow::{Context, Result, ensure};
use expected_write::{ExpectedWrite, ImageRegion, RegionKind, ResizePlan, WriteIntent, WritePlan};

use super::super::payload_writes::{
    PayloadFileWritePlan, PayloadWriteReport, apply_payload_write_plans,
};

pub(super) struct LocalizationComponentCandidate<'a> {
    pub id: &'static str,
    pub files: &'a BTreeMap<String, Vec<u8>>,
    pub writes: &'a [PayloadWriteReport],
    pub modified_files: &'static [&'static str],
}

#[derive(Debug)]
pub(super) struct ComposedFileFamily {
    pub files: BTreeMap<String, Vec<u8>>,
    pub writes: Vec<PayloadWriteReport>,
    pub producers: BTreeMap<String, String>,
}

pub(super) fn compose_file_family(
    baseline: &BTreeMap<String, Vec<u8>>,
    candidates: &[LocalizationComponentCandidate<'_>],
) -> Result<ComposedFileFamily> {
    let baseline_names = baseline.keys().map(String::as_str).collect::<BTreeSet<_>>();
    let mut owned_files = BTreeSet::new();
    let mut producers = BTreeMap::new();
    let mut plans = Vec::new();

    for candidate in candidates {
        let candidate_names = candidate
            .files
            .keys()
            .map(String::as_str)
            .collect::<BTreeSet<_>>();
        ensure!(
            candidate_names == baseline_names,
            "{} candidate changed the file-family population",
            candidate.id
        );
        let actual = baseline
            .iter()
            .filter_map(|(name, source)| {
                (candidate.files.get(name) != Some(source)).then_some(name.as_str())
            })
            .collect::<BTreeSet<_>>();
        let reported = candidate
            .writes
            .iter()
            .map(|write| write.file_name.as_str())
            .collect::<BTreeSet<_>>();
        let expected = candidate
            .modified_files
            .iter()
            .copied()
            .collect::<BTreeSet<_>>();
        ensure!(
            expected.len() == candidate.modified_files.len(),
            "{} declares a duplicate modified file",
            candidate.id
        );
        ensure!(
            actual == expected,
            "{} actual modified files differ from its declared ownership: actual={actual:?}, declared={expected:?}",
            candidate.id
        );
        ensure!(
            reported == expected,
            "{} reported Expected Write files differ from its declared ownership: reported={reported:?}, declared={expected:?}",
            candidate.id
        );

        for &file_name in candidate.modified_files {
            ensure!(
                owned_files.insert(file_name),
                "integrated localization components both own {file_name}"
            );
            let source = baseline.get(file_name).with_context(|| {
                format!("{} source family is missing {file_name}", candidate.id)
            })?;
            let replacement = candidate.files.get(file_name).with_context(|| {
                format!("{} candidate family is missing {file_name}", candidate.id)
            })?;
            plans.push(final_file_plan(
                file_name,
                source,
                replacement,
                candidate.id,
            ));
            producers.insert(file_name.to_owned(), candidate.id.to_owned());
        }
    }

    let applied = apply_payload_write_plans(baseline, plans)?;
    for candidate in candidates {
        for &file_name in candidate.modified_files {
            ensure!(
                applied.files.get(file_name) == candidate.files.get(file_name),
                "integrated localization output differs from the verified {file_name} candidate"
            );
        }
    }
    for (file_name, source) in baseline {
        if !owned_files.contains(file_name.as_str()) {
            ensure!(
                applied.files.get(file_name) == Some(source),
                "integrated localization changed unowned file {file_name}"
            );
        }
    }

    Ok(ComposedFileFamily {
        files: applied.files,
        writes: applied.report,
        producers,
    })
}

fn final_file_plan(
    file_name: &'static str,
    source: &[u8],
    replacement: &[u8],
    component: &str,
) -> PayloadFileWritePlan {
    let file_role = file_name
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>();
    let mut plan = WritePlan::new();
    if source.len() != replacement.len() {
        plan = plan.resize(ResizePlan {
            owner: "integrated-localization-composer".to_owned(),
            purpose: format!("adopt the verified {component} output size for {file_name}"),
            expected_input_len: source.len(),
            output_len: replacement.len(),
        });
    }
    plan = plan
        .region(ImageRegion {
            id: format!("integrated-localization-{file_role}"),
            range: 0..replacement.len(),
            kind: RegionKind::Data,
            reason: format!(
                "complete file serialized from the independently verified {component} component"
            ),
        })
        .write(ExpectedWrite {
            id: format!("integrated-localization-{file_role}"),
            owner: "integrated-localization-composer".to_owned(),
            purpose: format!("compose the verified {component} candidate for {file_name}"),
            offset: 0,
            expected_original: source[..source.len().min(replacement.len())].to_vec(),
            replacement: replacement.to_vec(),
            intent: WriteIntent::Data,
        });
    PayloadFileWritePlan { file_name, plan }
}

#[cfg(test)]
#[path = "composition_tests.rs"]
mod composition_tests;
