use std::collections::BTreeMap;

use anyhow::{Result, bail};

const GAIJI_PRODUCER: &str = "GAIJI.COM";

#[derive(Debug, Clone, Eq, PartialEq)]
pub(crate) struct GaijiCodeUsageAudit {
    pub audited_file_count: usize,
    pub occurrences: Vec<GaijiCodeOccurrence>,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub(crate) struct GaijiCodeOccurrence {
    pub origin: &'static str,
    pub file_name: String,
    pub offset: usize,
}

pub(crate) fn require_unreferenced_gaiji_code(
    shift_jis_code: u16,
    installer_payload: &BTreeMap<String, Vec<u8>>,
    runtime_files: &BTreeMap<String, Vec<u8>>,
    boot_files: &BTreeMap<String, Vec<u8>>,
) -> Result<GaijiCodeUsageAudit> {
    let audit =
        audit_gaiji_code_usage(shift_jis_code, installer_payload, runtime_files, boot_files);
    if !audit.occurrences.is_empty() {
        let first = &audit.occurrences[0];
        bail!(
            "GAIJI Shift_JIS code {shift_jis_code:04X} is already present in {} {} at offset {:#X}",
            first.origin,
            first.file_name,
            first.offset
        );
    }
    Ok(audit)
}

fn audit_gaiji_code_usage(
    shift_jis_code: u16,
    installer_payload: &BTreeMap<String, Vec<u8>>,
    runtime_files: &BTreeMap<String, Vec<u8>>,
    boot_files: &BTreeMap<String, Vec<u8>>,
) -> GaijiCodeUsageAudit {
    let needle = shift_jis_code.to_be_bytes();
    let mut audited_file_count = 0;
    let mut occurrences = Vec::new();
    for (origin, files) in [
        ("installer payload", installer_payload),
        ("runtime files", runtime_files),
        ("boot files", boot_files),
    ] {
        for (file_name, bytes) in files {
            if origin == "installer payload" && file_name == GAIJI_PRODUCER {
                continue;
            }
            audited_file_count += 1;
            occurrences.extend(
                bytes
                    .windows(needle.len())
                    .enumerate()
                    .filter(|(_, window)| *window == needle)
                    .map(|(offset, _)| GaijiCodeOccurrence {
                        origin,
                        file_name: file_name.clone(),
                        offset,
                    }),
            );
        }
    }
    GaijiCodeUsageAudit {
        audited_file_count,
        occurrences,
    }
}

#[cfg(test)]
#[path = "gaiji_code_usage_tests.rs"]
mod gaiji_code_usage_tests;
