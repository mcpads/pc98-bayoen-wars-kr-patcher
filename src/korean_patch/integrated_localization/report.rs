use std::collections::BTreeMap;

use anyhow::{Context, Result, ensure};

use super::model::{
    IntegratedLocalizationComponentReport, IntegratedLocalizationFileDisposition,
    IntegratedLocalizationFileReport,
};
use crate::source_disk::sha256_hex;

pub(super) fn component_report(
    id: &str,
    modified_files: &[&str],
    final_expected_write_count: usize,
    verified_inner_write_count: usize,
) -> IntegratedLocalizationComponentReport {
    IntegratedLocalizationComponentReport {
        id: id.to_owned(),
        modified_files: modified_files
            .iter()
            .map(|file_name| (*file_name).to_owned())
            .collect(),
        final_expected_write_count,
        verified_inner_write_count,
    }
}

pub(super) fn patched_file_reports(
    baseline: &BTreeMap<String, Vec<u8>>,
    output: &BTreeMap<String, Vec<u8>>,
    producers: &BTreeMap<String, String>,
) -> Result<Vec<IntegratedLocalizationFileReport>> {
    producers
        .iter()
        .map(|(file_name, producer)| {
            let source = baseline
                .get(file_name)
                .with_context(|| format!("integrated source family is missing {file_name}"))?;
            let replacement = output
                .get(file_name)
                .with_context(|| format!("integrated output family is missing {file_name}"))?;
            ensure!(
                source != replacement,
                "integrated producer {producer} did not change {file_name}"
            );
            Ok(IntegratedLocalizationFileReport {
                file_name: file_name.clone(),
                disposition: IntegratedLocalizationFileDisposition::PatchedDevelopment,
                producer: producer.clone(),
                source_size: source.len(),
                output_size: replacement.len(),
                output_sha256: sha256_hex(replacement),
            })
        })
        .collect()
}

pub(super) fn preserved_file_report(
    baseline: &BTreeMap<String, Vec<u8>>,
    output: &BTreeMap<String, Vec<u8>>,
    file_name: &str,
    disposition: IntegratedLocalizationFileDisposition,
    producer: &str,
) -> Result<IntegratedLocalizationFileReport> {
    let source = baseline
        .get(file_name)
        .with_context(|| format!("integrated source family is missing {file_name}"))?;
    let preserved = output
        .get(file_name)
        .with_context(|| format!("integrated output family is missing {file_name}"))?;
    ensure!(
        source == preserved,
        "integrated preservation policy {producer} changed {file_name}"
    );
    Ok(IntegratedLocalizationFileReport {
        file_name: file_name.to_owned(),
        disposition,
        producer: producer.to_owned(),
        source_size: source.len(),
        output_size: preserved.len(),
        output_sha256: sha256_hex(preserved),
    })
}
