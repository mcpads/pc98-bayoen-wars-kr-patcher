use super::*;

fn occupied_rows(bitmap: &[u8; GLYPH_BYTES]) -> Vec<usize> {
    (0..GLYPH_HEIGHT)
        .filter(|row| row_has_ink(bitmap, *row))
        .collect()
}

#[test]
#[ignore = "requires the Galmuri and Mulmaru font files in assets/fonts or BAYOEN_WARS_FONT_DIR"]
fn dialogue_wave_is_centered_and_ellipsis_uses_the_period_baseline() {
    let overrides = dialogue_punctuation_overrides().unwrap();

    assert_eq!(occupied_rows(&overrides[&'~']), [6, 7, 8]);
    assert_eq!(occupied_rows(&overrides[&'…']), [14]);
}
