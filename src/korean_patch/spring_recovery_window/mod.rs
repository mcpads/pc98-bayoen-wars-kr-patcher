mod compile;
mod model;

use std::collections::BTreeMap;

use anyhow::Result;
use expected_write::WritePlan;
use v30::AssembledProgram;

pub(in crate::korean_patch) use self::model::CompiledSpringRecoveryWindow;
use super::interface_window_layout::{
    add_window_layout_sources, add_window_layout_writes, verify_window_layout_writes,
};

pub(in crate::korean_patch) use compile::compile_spring_recovery_window;
pub use model::SpringRecoveryWindowPatchReport;

pub(in crate::korean_patch) fn add_spring_recovery_window_plan(
    plan: WritePlan,
    mad_com: &[u8],
    compiled: &CompiledSpringRecoveryWindow,
) -> Result<WritePlan> {
    add_window_layout_writes(
        plan,
        mad_com,
        "spring-recovery-window-layout",
        &compiled.typed_writes,
        &compiled.metadata_writes,
    )
}

pub(in crate::korean_patch) fn add_spring_recovery_window_sources(
    sources: &mut BTreeMap<String, AssembledProgram>,
    compiled: &CompiledSpringRecoveryWindow,
) -> Result<()> {
    add_window_layout_sources(sources, &compiled.typed_writes)
}

pub(in crate::korean_patch) fn verify_spring_recovery_window(
    mad_com: &[u8],
    compiled: &CompiledSpringRecoveryWindow,
) -> Result<()> {
    verify_window_layout_writes(mad_com, &compiled.typed_writes, &compiled.metadata_writes)
}

pub(in crate::korean_patch) fn patch_report(
    compiled: &CompiledSpringRecoveryWindow,
) -> SpringRecoveryWindowPatchReport {
    compiled.report.clone()
}
