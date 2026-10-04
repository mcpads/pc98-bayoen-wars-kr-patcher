use super::*;

#[test]
fn only_source_growth_kinds_are_advisories() {
    assert!(is_source_growth(
        TranslationLayoutFindingKind::SourceStorageGrowth
    ));
    assert!(!is_source_growth(
        TranslationLayoutFindingKind::FixedGridColumnOverflow
    ));
}

#[test]
fn completed_static_producer_surfaces_have_no_open_consumer_gate() {
    for surface in [
        TranslationSurface::FixedGaijiText,
        TranslationSurface::OpeningGlyphIndexPages,
        TranslationSurface::EndingGlyphIndexPages,
        TranslationSurface::BakedGraphicsText,
    ] {
        assert_eq!(in_game_consumer_gate_reason("fixture", surface), None);
    }

    assert!(
        in_game_consumer_gate_reason("mad-interface", TranslationSurface::InterfaceText).is_some()
    );
    assert!(in_game_consumer_gate_reason("mad-dialogue", TranslationSurface::Dialogue).is_some());
}

#[test]
fn observed_dialogue_groups_have_no_open_consumer_gate() {
    for group in 1..=11 {
        assert_eq!(
            in_game_consumer_gate_reason(
                &format!("dialogue-group-{group:02}"),
                TranslationSurface::Dialogue,
            ),
            None
        );
    }

    assert!(
        in_game_consumer_gate_reason("dialogue-group-11", TranslationSurface::InterfaceText)
            .is_some()
    );
    assert!(
        in_game_consumer_gate_reason("dialogue-group-01", TranslationSurface::InterfaceText)
            .is_some()
    );
    assert!(
        in_game_consumer_gate_reason("dialogue-group-03", TranslationSurface::InterfaceText)
            .is_some()
    );
}
