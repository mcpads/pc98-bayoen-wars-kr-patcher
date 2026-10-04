use super::*;

#[test]
fn frame_background_and_text_source_columns_are_aligned() {
    assert_eq!(SOURCE_FRAME_WORDS[0] + 2, BACKGROUND_WIDTH_WORDS);
    assert_eq!(SOURCE_TEXT_POSITION % 80, SOURCE_FRAME_WORDS[2] % 80 + 2);
    assert_eq!(BACKGROUND_HEIGHT_ROWS, 0x50);
}
