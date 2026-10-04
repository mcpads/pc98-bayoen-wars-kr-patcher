mod compile;
mod model;
mod plans;
mod readback;

pub(super) use compile::compile_spring_capture_window;
pub(in crate::korean_patch) use model::CompiledSpringCaptureWindow;
pub use model::SpringCaptureWindowPatchReport;
pub(super) use plans::{add_spring_capture_window_plan, add_spring_capture_window_sources};
pub(super) use readback::verify_spring_capture_window;

pub(super) fn patch_report(
    compiled: &CompiledSpringCaptureWindow,
) -> SpringCaptureWindowPatchReport {
    compiled.report.clone()
}
