use super::*;

fn gaiji(index: usize, shift_jis_code: u16) -> GaijiGlyph {
    GaijiGlyph {
        index,
        character_code: 0x7621 + index as u16,
        shift_jis_code,
        file_offset: 0,
        byte_size: 34,
        sha256: String::new(),
        meaning: GaijiGlyphMeaning::Character { character: '字' },
    }
}

#[test]
fn parser_preserves_standard_text_controls_and_gaiji_identity() {
    let custom = gaiji(3, 0xeba2);
    let gaiji_by_shift_jis = BTreeMap::from([(custom.shift_jis_code, &custom)]);
    let bytes = [
        0x83, 0x65, 0x83, 0x58, 0x83, 0x67, b'$', b'3', 0xeb, 0xa2, b'$', b'0', 0x82, 0xa0, b'$',
        b'$',
    ];

    let parsed = parse_interface_text(&bytes, 0, &gaiji_by_shift_jis).unwrap();

    assert_eq!(parsed.end_offset, bytes.len());
    assert_eq!(parsed.text, "テスト<DISPLAY:3>字\nあ");
    assert_eq!(parsed.gaiji_glyph_indices, vec![3]);
    assert!(matches!(
        parsed.tokens[1],
        InterfaceTextToken::DisplayAttribute { code: 3 }
    ));
    assert!(matches!(
        parsed.tokens[2],
        InterfaceTextToken::Gaiji {
            glyph_index: 3,
            shift_jis_code: 0xeba2,
            source: GaijiGlyphMeaning::Character { character: '字' },
            ..
        }
    ));
}

#[test]
fn unsupported_control_is_rejected() {
    let error = parse_interface_text(b"$9$$", 0, &BTreeMap::new()).unwrap_err();

    assert!(
        error
            .to_string()
            .contains("unsupported interface text control $9")
    );
}

#[test]
fn invalid_shift_jis_trail_byte_is_rejected() {
    let error = parse_interface_text(&[0x82, 0x7f, b'$', b'$'], 0, &BTreeMap::new()).unwrap_err();

    assert!(
        error
            .to_string()
            .contains("invalid Shift_JIS trailing byte")
    );
}
