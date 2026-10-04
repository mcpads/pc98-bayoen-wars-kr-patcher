use super::*;

#[test]
#[ignore = "requires the Galmuri and Mulmaru font files in assets/fonts or BAYOEN_WARS_FONT_DIR"]
fn styled_label_uses_source_letter_fill_outline_and_shadow_roles() {
    let canvas = DifficultyLabelCanvas::render("쉬움", 128).unwrap();

    assert_eq!(
        canvas.colors(),
        REQUIRED_LABEL_COLORS
            .into_iter()
            .collect::<std::collections::BTreeSet<_>>()
    );
}

#[test]
fn styled_label_rejects_text_wider_than_the_consumer_owned_tiles() {
    let error = DifficultyLabelCanvas::render("너무어려움", 96).unwrap_err();

    assert!(error.to_string().contains("writable tiles provide 96"));
}
