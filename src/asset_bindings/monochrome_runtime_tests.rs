use super::*;

fn verified_runtime_fixture() -> Vec<u8> {
    let mut program = vec![0; GLYPH_RENDERER_FILE_OFFSET + 4];
    program[GLYPH_BUFFER_SEGMENT_STORE_FILE_OFFSET..NEXT_BUFFER_SEGMENT_STORE_FILE_OFFSET + 4]
        .copy_from_slice(&[
            0x2e, 0xa3, 0x93, 0xd5, // mov cs:[d593], ax
            0x05, 0x00, 0x08, // add ax, 0800h
            0x2e, 0xa3, 0x95, 0xd5, // mov cs:[d595], ax
        ]);
    program[GLYPH_INDEX_LOAD_FILE_OFFSET..GLYPH_INDEX_LOAD_FILE_OFFSET + 2]
        .copy_from_slice(&[0x8a, 0x27]);
    program[LINE_BREAK_COMPARE_FILE_OFFSET..LINE_BREAK_COMPARE_FILE_OFFSET + 3]
        .copy_from_slice(&[0x80, 0xfc, LINE_BREAK]);
    program[PAGE_END_COMPARE_FILE_OFFSET..PAGE_END_COMPARE_FILE_OFFSET + 3]
        .copy_from_slice(&[0x80, 0xfc, PAGE_END]);
    program[GLYPH_RENDERER_CALL_FILE_OFFSET..GLYPH_RENDERER_CALL_FILE_OFFSET + 3]
        .copy_from_slice(&[0xe8, 0xfc, 0x01]);
    program[GLYPH_RENDERER_FILE_OFFSET..GLYPH_RENDERER_FILE_OFFSET + 4]
        .copy_from_slice(&[0x32, 0xc0, 0xd1, 0xe8]);
    program
}

#[test]
fn byte_index_consumer_reaches_every_non_control_glyph_inside_its_buffer() {
    let runtime = catalog_monochrome_runtime(&verified_runtime_fixture()).unwrap();

    assert_eq!(runtime.decoded_buffer_paragraph_count, 0x800);
    assert_eq!(runtime.decoded_buffer_byte_size, 0x8000);
    assert_eq!(runtime.usable_glyph_count, 0xfe);
    assert_eq!(runtime.highest_glyph_index, 0xfd);
    assert_eq!(runtime.highest_glyph_end_offset, 0x7f00);
    assert_eq!(runtime.remaining_buffer_byte_count, 0x100);
}

#[test]
fn runtime_binding_rejects_a_smaller_or_unrelated_buffer_layout() {
    let mut program = verified_runtime_fixture();
    program[GLYPH_BUFFER_PARAGRAPH_COUNT_FILE_OFFSET + 1] = 0x00;
    program[GLYPH_BUFFER_PARAGRAPH_COUNT_FILE_OFFSET + 2] = 0x04;

    let error = catalog_monochrome_runtime(&program).unwrap_err();

    assert!(error.to_string().contains("index range exceeds"));
}
