use std::collections::BTreeMap;

use super::*;

#[test]
fn codes_outside_the_installed_catalog_are_rejected() {
    let by_shift_jis = BTreeMap::new();
    let mut slot = Vec::new();
    for line in 0..6 {
        slot.extend_from_slice(if line == 0 {
            &[0xeb, 0x9f]
        } else {
            &[0x81, 0x40]
        });
        for _ in 1..7 {
            slot.extend_from_slice(&[0x81, 0x40]);
        }
        slot.extend_from_slice(if line == 5 { b"$$" } else { b"$0" });
    }

    let error = parse_fixed_gaiji_slot(&slot, &by_shift_jis).unwrap_err();

    assert!(error.to_string().contains("uninstalled Shift_JIS code"));
}

#[test]
fn readable_text_preserves_cells_and_line_breaks() {
    let lines = vec![
        vec![
            GaijiTextCell::Glyph {
                glyph_index: 1,
                character_code: 0x7622,
                shift_jis_code: 0xeba0,
                source: GaijiGlyphMeaning::Character { character: 'あ' },
            },
            GaijiTextCell::Space,
            GaijiTextCell::Standard {
                text: "！".to_owned(),
            },
        ],
        vec![GaijiTextCell::Glyph {
            glyph_index: 139,
            character_code: 0x772e,
            shift_jis_code: 0xec4d,
            source: GaijiGlyphMeaning::Graphic {
                role: "halftone-fill".to_owned(),
            },
        }],
    ];

    assert_eq!(
        format_fixed_gaiji_text(&lines),
        "あ　！\n<GAIJI-GRAPHIC:halftone-fill>"
    );
}

#[test]
fn standard_shift_jis_punctuation_is_a_fixed_cell() {
    let by_shift_jis = BTreeMap::new();
    let mut slot = Vec::new();
    for line in 0..6 {
        for _ in 0..7 {
            slot.extend_from_slice(&[0x81, 0x49]);
        }
        slot.extend_from_slice(if line == 5 { b"$$" } else { b"$0" });
    }

    let (lines, glyphs) = parse_fixed_gaiji_slot(&slot, &by_shift_jis).unwrap();

    assert!(glyphs.is_empty());
    assert!(matches!(
        &lines[0][0],
        GaijiTextCell::Standard { text } if text == "！"
    ));
}
