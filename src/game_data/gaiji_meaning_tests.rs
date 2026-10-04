use super::*;

#[test]
fn character_ranges_end_at_the_visually_verified_boundaries() {
    assert_eq!(CHARACTER_PREFIX.chars().count(), FIRST_GRAPHIC_INDEX);
    assert_eq!(
        CHARACTER_SUFFIX.chars().count(),
        EFFECT_START - CHARACTER_SUFFIX_START
    );

    assert_eq!(
        meaning_for_index(0).unwrap(),
        GaijiGlyphMeaning::Character { character: '。' }
    );
    assert_eq!(
        meaning_for_index(138).unwrap(),
        GaijiGlyphMeaning::Character { character: 'ュ' }
    );
    assert_eq!(
        meaning_for_index(141).unwrap(),
        GaijiGlyphMeaning::Character { character: '全' }
    );
    assert_eq!(
        meaning_for_index(172).unwrap(),
        GaijiGlyphMeaning::Character { character: 'Ｉ' }
    );
}

#[test]
fn non_text_records_remain_typed_graphics() {
    assert_eq!(meaning_for_index(139).unwrap(), graphic("halftone-fill"));
    assert_eq!(meaning_for_index(140).unwrap(), graphic("solid-fill"));
    assert_eq!(
        meaning_for_index(183).unwrap(),
        graphic("particle-frame-11")
    );
    assert!(meaning_for_index(GLYPH_COUNT).is_err());
    assert_eq!(GRAPHIC_GLYPH_COUNT, 13);
}
