use std::collections::{BTreeMap, BTreeSet};

use anyhow::{Context, Result, bail};
use expected_write::{MachineCodeVerifier, WriteIntent, WritePlan};
use serde::Serialize;

pub(crate) struct PayloadFileWritePlan {
    pub file_name: &'static str,
    pub plan: WritePlan,
}

#[derive(Debug)]
pub(crate) struct AppliedPayloadWrites {
    pub files: BTreeMap<String, Vec<u8>>,
    pub report: Vec<PayloadWriteReport>,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize)]
pub struct PayloadWriteReport {
    pub id: String,
    pub file_name: String,
    pub owner: String,
    pub offset: usize,
    pub byte_size: usize,
    pub intent: String,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize)]
pub struct UnpackedWriteReport {
    pub id: String,
    pub owner: String,
    pub offset: usize,
    pub byte_size: usize,
    pub intent: String,
}

pub(crate) fn unpacked_write_report(plan: &WritePlan) -> Vec<UnpackedWriteReport> {
    plan.writes
        .iter()
        .map(|write| UnpackedWriteReport {
            id: write.id.clone(),
            owner: write.owner.clone(),
            offset: write.offset,
            byte_size: write.replacement.len(),
            intent: match &write.intent {
                WriteIntent::Data => "data",
                WriteIntent::Metadata => "metadata",
                WriteIntent::MachineCode(_) => "machine_code",
            }
            .to_owned(),
        })
        .collect()
}

pub(crate) fn apply_payload_write_plans(
    installer_payload: &BTreeMap<String, Vec<u8>>,
    file_plans: Vec<PayloadFileWritePlan>,
) -> Result<AppliedPayloadWrites> {
    apply_payload_write_plans_with_verifier(installer_payload, file_plans, None)
}

pub(crate) fn combine_payload_write_plans(
    file_plans: Vec<PayloadFileWritePlan>,
) -> Result<Vec<PayloadFileWritePlan>> {
    let mut combined = BTreeMap::<&'static str, WritePlan>::new();
    for file_plan in file_plans {
        let Some(existing) = combined.get_mut(file_plan.file_name) else {
            combined.insert(file_plan.file_name, file_plan.plan);
            continue;
        };
        match (existing.resize.as_ref(), file_plan.plan.resize.as_ref()) {
            (Some(first), Some(second)) => bail!(
                "multiple Expected Write resize plans own installer file {}: {} and {}",
                file_plan.file_name,
                first.owner,
                second.owner
            ),
            (None, Some(_)) => existing.resize = file_plan.plan.resize,
            _ => {}
        }
        existing.regions.extend(file_plan.plan.regions);
        existing.writes.extend(file_plan.plan.writes);
    }
    Ok(combined
        .into_iter()
        .map(|(file_name, plan)| PayloadFileWritePlan { file_name, plan })
        .collect())
}

pub(crate) fn apply_payload_write_plans_with_verifier(
    installer_payload: &BTreeMap<String, Vec<u8>>,
    file_plans: Vec<PayloadFileWritePlan>,
    machine_code_verifier: Option<&dyn MachineCodeVerifier>,
) -> Result<AppliedPayloadWrites> {
    let mut planned_files = BTreeSet::new();
    let mut output = installer_payload.clone();
    let mut report = Vec::new();

    for file_plan in file_plans {
        if !planned_files.insert(file_plan.file_name) {
            bail!(
                "multiple Expected Write plans own installer file {}",
                file_plan.file_name
            );
        }
        let baseline = installer_payload
            .get(file_plan.file_name)
            .with_context(|| {
                format!(
                    "Expected Write plan names missing installer file {}",
                    file_plan.file_name
                )
            })?;
        let patched = file_plan
            .plan
            .apply(baseline, machine_code_verifier)
            .with_context(|| format!("apply Expected Writes to {}", file_plan.file_name))?;
        file_plan
            .plan
            .audit(baseline, &patched, machine_code_verifier)
            .with_context(|| format!("audit Expected Writes for {}", file_plan.file_name))?;
        output.insert(file_plan.file_name.to_owned(), patched);
        report.extend(file_plan.plan.writes.iter().map(|write| {
            PayloadWriteReport {
                id: write.id.clone(),
                file_name: file_plan.file_name.to_owned(),
                owner: write.owner.clone(),
                offset: write.offset,
                byte_size: write.replacement.len(),
                intent: match &write.intent {
                    WriteIntent::Data => "data",
                    WriteIntent::Metadata => "metadata",
                    WriteIntent::MachineCode(_) => "machine_code",
                }
                .to_owned(),
            }
        }));
    }

    Ok(AppliedPayloadWrites {
        files: output,
        report,
    })
}

#[cfg(test)]
#[path = "payload_writes_tests.rs"]
mod payload_writes_tests;
