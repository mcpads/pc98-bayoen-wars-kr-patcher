use anyhow::{Result, ensure};

use super::{CenteredInterfaceWindowLayout, CompiledWindowMetadataWrite};

pub(in crate::korean_patch) fn compile_centered_frame_metadata_writes(
    id_prefix: &str,
    frame_role: &str,
    frame_record_file_offset: usize,
    source_inner_width_words: u16,
    source_invalidation_x_pixels: u16,
    layout: &CenteredInterfaceWindowLayout,
) -> Result<Vec<CompiledWindowMetadataWrite>> {
    ensure!(
        source_invalidation_x_pixels >= layout.output_frame_x_pixels,
        "{frame_role} background invalidation X moved right"
    );
    Ok(vec![
        CompiledWindowMetadataWrite {
            write_id: format!("{id_prefix}-frame-inner-width"),
            purpose: format!(
                "expand the {frame_role} frame from {} to {} inner words",
                source_inner_width_words, layout.output_inner_width_words
            ),
            file_offset: frame_record_file_offset,
            output_value: layout.output_inner_width_words,
        },
        CompiledWindowMetadataWrite {
            write_id: format!("{id_prefix}-frame-origin"),
            purpose: format!("keep the expanded {frame_role} frame centered"),
            file_offset: frame_record_file_offset + 4,
            output_value: layout.output_frame_origin,
        },
        CompiledWindowMetadataWrite {
            write_id: format!("{id_prefix}-background-invalidation-x"),
            purpose: format!(
                "invalidate the map background under the full expanded {frame_role} frame"
            ),
            file_offset: frame_record_file_offset + 6,
            output_value: layout.output_frame_x_pixels,
        },
    ])
}

#[cfg(test)]
#[path = "frame_record_tests.rs"]
mod frame_record_tests;
