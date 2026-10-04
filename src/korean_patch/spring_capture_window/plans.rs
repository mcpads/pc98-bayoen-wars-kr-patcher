use std::collections::BTreeMap;

use anyhow::Result;
use expected_write::WritePlan;
use v30::AssembledProgram;

use super::model::CompiledSpringCaptureWindow;
use crate::korean_patch::interface_window_layout::{
    add_window_layout_sources, add_window_layout_writes,
};

pub(in crate::korean_patch) fn add_spring_capture_window_plan(
    plan: WritePlan,
    mad_com: &[u8],
    compiled: &CompiledSpringCaptureWindow,
) -> Result<WritePlan> {
    add_window_layout_writes(
        plan,
        mad_com,
        "spring-capture-window-layout",
        &compiled.typed_writes,
        &compiled.metadata_writes,
    )
}

pub(in crate::korean_patch) fn add_spring_capture_window_sources(
    sources: &mut BTreeMap<String, AssembledProgram>,
    compiled: &CompiledSpringCaptureWindow,
) -> Result<()> {
    add_window_layout_sources(sources, &compiled.typed_writes)
}
