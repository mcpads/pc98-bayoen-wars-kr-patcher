use super::*;

#[test]
fn each_scene_layout_consumes_its_exact_four_planes() {
    for layout in SCENE_SHEETS {
        assert_eq!(layout.source_row_bytes * layout.height, layout.plane_stride);
        assert_eq!(layout.plane_stride * 4, layout.width * layout.height / 2);
    }
}
