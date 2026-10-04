mod compile;
mod model;

use std::collections::BTreeMap;

use anyhow::Result;
use expected_write::WritePlan;
use v30::AssembledProgram;

pub(in crate::korean_patch) use self::model::CompiledSharedAlertWindow;
use super::interface_window_layout::{
    add_window_layout_sources, add_window_layout_writes, verify_window_layout_writes,
};

pub(in crate::korean_patch) use compile::{
    compile_centered_text_window, compile_shared_alert_window,
};
pub use model::SharedAlertWindowPatchReport;

pub(in crate::korean_patch) fn add_shared_alert_window_plan(
    plan: WritePlan,
    mad_com: &[u8],
    compiled: &CompiledSharedAlertWindow,
) -> Result<WritePlan> {
    add_window_layout_writes(
        plan,
        mad_com,
        "shared-alert-window-layout",
        &compiled.typed_writes,
        &compiled.metadata_writes,
    )
}

pub(in crate::korean_patch) fn add_shared_alert_window_sources(
    sources: &mut BTreeMap<String, AssembledProgram>,
    compiled: &CompiledSharedAlertWindow,
) -> Result<()> {
    add_window_layout_sources(sources, &compiled.typed_writes)
}

pub(in crate::korean_patch) fn verify_shared_alert_window(
    mad_com: &[u8],
    compiled: &CompiledSharedAlertWindow,
) -> Result<()> {
    verify_window_layout_writes(mad_com, &compiled.typed_writes, &compiled.metadata_writes)
}

pub(in crate::korean_patch) fn patch_report(
    compiled: &CompiledSharedAlertWindow,
) -> SharedAlertWindowPatchReport {
    compiled.report.clone()
}
