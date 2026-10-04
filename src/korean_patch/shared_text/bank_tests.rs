use super::*;

#[test]
fn unique_characters_keep_first_use_order_and_ignore_plain_spaces() {
    assert_eq!(
        collect_unique_characters(&["가 A나가", "…다나"]).unwrap(),
        ['가', '나', '다']
    );
}

#[test]
fn explicit_overrides_claim_native_punctuation_in_first_use_order() {
    let overrides = [('~', [0_u8; GLYPH_BYTES]), ('…', [0_u8; GLYPH_BYTES])]
        .into_iter()
        .collect();

    assert_eq!(
        collect_unique_characters_with_overrides(&["가~…나~"], &overrides).unwrap(),
        ['가', '~', '…', '나']
    );
}

#[test]
fn unsupported_whitespace_is_rejected() {
    let error = collect_unique_characters(&["가\t나"]).unwrap_err();

    assert!(error.to_string().contains("unsupported whitespace"));
}

#[test]
fn readiness_probe_slot_is_not_available_for_translation_assignment() {
    let glyphs = (0..3)
        .map(|index| GaijiGlyph {
            index,
            character_code: 0x7621 + index as u16,
            shift_jis_code: 0xeb9f + index as u16,
            file_offset: index * 34,
            byte_size: 34,
            sha256: String::new(),
            meaning: GaijiGlyphMeaning::Character { character: '字' },
        })
        .collect();
    let gaiji = GaijiCatalog {
        sha256: String::new(),
        file_size: 102,
        installer: crate::game_data::GaijiInstallerCatalog {
            install_loop_offset: 0,
            install_loop_byte_size: 0,
            install_loop_instruction_count: 0,
            install_call_offset: 0,
            install_routine_offset: 0,
            install_routine_byte_size: 0,
            install_routine_instruction_count: 0,
            bios_interrupt_vector: 0x18,
            bios_function: 0x1a,
            glyph_count: 3,
            first_character_code: 0x7621,
            row_end_character_code: 0x767e,
            next_row_previous_character_code: 0x7720,
            pointer_table_com_address: 0,
            glyph_segment_register: "bx".to_owned(),
            glyph_record_offset_register: "cx".to_owned(),
            character_code_register: "dx".to_owned(),
        },
        pointer_table_offset: 0,
        first_character_code: 0x7621,
        last_character_code: 0x7623,
        first_shift_jis_code: 0xeb9f,
        last_shift_jis_code: 0xeba1,
        uniform_record_size: Some(34),
        glyphs,
    };
    let readiness = GaijiReadinessCatalog {
        check_offset: 0,
        expected_bitmap_table_offset: 0,
        probe_renderer_offset: 0,
        probe_text_offset: 0,
        probe_shift_jis_code: 0xeba0,
        reserved_glyph_index: 1,
        row_count: 16,
    };

    let indexes = available_slots(&gaiji, &readiness)
        .unwrap()
        .into_iter()
        .map(|glyph| glyph.index)
        .collect::<Vec<_>>();

    assert_eq!(indexes, [0, 2]);
}
