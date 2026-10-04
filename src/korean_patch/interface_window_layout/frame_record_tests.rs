use super::*;
use crate::korean_patch::interface_window_layout::center_interface_window;

#[test]
fn centered_frame_moves_its_background_invalidation_origin() {
    let layout = center_interface_window(7, 12, 0x2d18, 0x321a).unwrap();
    let writes = compile_centered_frame_metadata_writes(
        "shared-alert",
        "shared alert",
        0x6bb8,
        7,
        0x00c0,
        &layout,
    )
    .unwrap();

    assert_eq!(writes.len(), 3);
    assert_eq!(writes[2].file_offset, 0x6bbe);
    assert_eq!(writes[2].output_value, 0x0090);
    assert_eq!(writes[2].write_id, "shared-alert-background-invalidation-x");
}
