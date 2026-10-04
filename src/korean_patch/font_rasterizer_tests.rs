use super::*;

#[test]
#[ignore = "requires the Galmuri and Mulmaru font files in assets/fonts or BAYOEN_WARS_FONT_DIR"]
fn embedded_profile_verifies_the_font_and_provenance() {
    let provenance = font_provenance().unwrap();

    assert_eq!(
        provenance.font_sha256,
        "6fe6c3fe4369e3837ac348431e8670733d67aa4bd550982baa72cc93c81a1c68"
    );
    assert_eq!(provenance.font_version, "2.404");
    assert_eq!(
        provenance.upstream_revision,
        "bdb86ae89466a361eb8df861222736b81ff975ef"
    );
}

#[test]
#[ignore = "requires the Galmuri and Mulmaru font files in assets/fonts or BAYOEN_WARS_FONT_DIR"]
fn rasterized_hangul_is_a_nonempty_16_by_16_bitmap() {
    let bitmap = rasterize_hangul_syllable('가').unwrap();

    assert_eq!(bitmap.len(), GLYPH_BYTES);
    assert!(bitmap.iter().any(|byte| *byte != 0));
}

#[test]
#[ignore = "requires the Galmuri and Mulmaru font files in assets/fonts or BAYOEN_WARS_FONT_DIR"]
fn title_and_narrative_targets_produce_their_declared_cell_sizes() {
    let narrative = LargeCellRasterizer::load_for(FontTarget::Narrative32)
        .unwrap()
        .rasterize_visible_character('마')
        .unwrap();
    let title = FixedCellRasterizer::load_for(FontTarget::TitlePrimary16)
        .unwrap()
        .rasterize_visible_character('마')
        .unwrap();

    assert_eq!(narrative.len(), LARGE_GLYPH_BYTES);
    assert_eq!(title.len(), GLYPH_BYTES);
}

#[test]
fn non_hangul_input_is_rejected() {
    let error = rasterize_hangul_syllable('A').unwrap_err();

    assert!(error.to_string().contains("not a modern Hangul syllable"));
}

#[test]
#[ignore = "requires the Galmuri and Mulmaru font files in assets/fonts or BAYOEN_WARS_FONT_DIR"]
fn visible_punctuation_uses_the_same_fixed_font_profile() {
    let rasterizer = FixedCellRasterizer::load().unwrap();
    let ellipsis = rasterizer.rasterize_visible_character('…').unwrap();
    let comma = rasterizer.rasterize_visible_character(',').unwrap();

    assert_eq!(ellipsis.len(), GLYPH_BYTES);
    assert!(ellipsis.iter().any(|byte| *byte != 0));
    assert!(comma.iter().any(|byte| *byte != 0));
}

#[test]
#[ignore = "requires the Galmuri and Mulmaru font files in assets/fonts or BAYOEN_WARS_FONT_DIR"]
fn whitespace_is_rejected_as_a_font_glyph() {
    let error = FixedCellRasterizer::load()
        .unwrap()
        .rasterize_visible_character(' ')
        .unwrap_err();

    assert!(error.to_string().contains("not a visible character"));
}

#[test]
#[ignore = "requires the Galmuri and Mulmaru font files in assets/fonts or BAYOEN_WARS_FONT_DIR"]
fn unsupported_character_is_not_rendered_as_the_missing_glyph() {
    let error = FixedCellRasterizer::load()
        .unwrap()
        .rasterize_visible_character('\u{10ffff}')
        .unwrap_err();

    assert!(error.to_string().contains("has no glyph"));
}
