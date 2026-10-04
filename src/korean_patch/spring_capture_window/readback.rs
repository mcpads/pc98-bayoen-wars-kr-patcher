use anyhow::Result;

use super::model::CompiledSpringCaptureWindow;
use crate::korean_patch::interface_window_layout::verify_window_layout_writes;

pub(in crate::korean_patch) fn verify_spring_capture_window(
    mad_com: &[u8],
    compiled: &CompiledSpringCaptureWindow,
) -> Result<()> {
    verify_window_layout_writes(mad_com, &compiled.typed_writes, &compiled.metadata_writes)
}
