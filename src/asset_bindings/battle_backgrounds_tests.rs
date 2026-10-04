use super::*;

#[test]
fn observed_frame_counts_cover_both_decoded_assets() {
    assert_eq!(7 * FRAME_SIZE, 57_344);
    assert_eq!(4 * FRAME_SIZE, 32_768);
}

#[test]
fn selection_offsets_cover_the_six_alternate_b04_and_four_b05_frames() {
    assert_eq!(
        &SELECTION_OFFSETS[..6],
        &[0x2000, 0x4000, 0x6000, 0x8000, 0xa000, 0xc000]
    );
    assert_eq!(&SELECTION_OFFSETS[6..], &[0, 0x2000, 0x4000, 0x6000]);
}
