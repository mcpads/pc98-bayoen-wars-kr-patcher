use serde::Serialize;

use super::super::interface_window_layout::{
    CompiledWindowMetadataWrite, CompiledWindowTypedWrite,
};

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct SpringRecoveryWindowPatchReport {
    pub source_text_entry_ids: Vec<String>,
    pub required_inner_width_words: u16,
    pub source_frame_inner_width_words: u16,
    pub output_frame_inner_width_words: u16,
    pub horizontal_shift_words: u16,
    pub source_frame_origin: u16,
    pub output_frame_origin: u16,
    pub source_text_position: u16,
    pub output_text_position: u16,
    pub source_background_width_words: u16,
    pub output_background_width_words: u16,
    pub background_height_rows: u16,
    pub frame_width_file_offset: usize,
    pub frame_origin_file_offset: usize,
    pub source_background_invalidation_x_pixels: u16,
    pub output_background_invalidation_x_pixels: u16,
    pub background_invalidation_x_file_offset: usize,
    pub background_origin_instruction_file_offsets: Vec<usize>,
    pub background_width_instruction_file_offsets: Vec<usize>,
    pub text_position_instruction_file_offsets: Vec<usize>,
}

pub(in crate::korean_patch) struct CompiledSpringRecoveryWindow {
    pub report: SpringRecoveryWindowPatchReport,
    pub typed_writes: Vec<CompiledWindowTypedWrite>,
    pub metadata_writes: Vec<CompiledWindowMetadataWrite>,
}
