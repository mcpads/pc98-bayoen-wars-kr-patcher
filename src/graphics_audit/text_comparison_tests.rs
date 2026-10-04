use super::*;

#[test]
fn replacement_overrides_are_read_from_reported_program_offsets() {
    let mut program = vec![0u8; 40];
    program[6 + 2] = 0x80;
    let glyphs = vec![SharedGaijiGlyph {
        character: "한".to_owned(),
        slot_index: 1,
        shift_jis_code: "0xEB9F".to_owned(),
        record_offset: 6,
    }];

    let overrides = replacement_gaiji_overrides(&program, &glyphs).unwrap();

    assert_eq!(overrides[&'한'][0], 0x80);
}

#[test]
fn replacement_override_rejects_a_non_gaiji_prefix() {
    let mut program = vec![0u8; 40];
    program[6] = 1;
    let glyphs = vec![SharedGaijiGlyph {
        character: "한".to_owned(),
        slot_index: 1,
        shift_jis_code: "0xEB9F".to_owned(),
        record_offset: 6,
    }];

    let error = replacement_gaiji_overrides(&program, &glyphs).unwrap_err();
    assert!(error.to_string().contains("unexpected prefix"));
}
