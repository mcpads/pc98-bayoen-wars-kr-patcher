use super::*;

#[test]
fn dos_facing_text_is_excluded_from_in_game_translation_completion() {
    for surface in [
        TranslationSurface::DosSystemText,
        TranslationSurface::MenuText,
        TranslationSurface::ExternalProgramText,
    ] {
        assert_eq!(
            excluded_non_game_text_reason(surface),
            Some(EXCLUDED_NON_GAME_TEXT_REASON)
        );
    }
}

#[test]
fn game_content_surfaces_remain_in_translation_completion_scope() {
    for surface in [
        TranslationSurface::InterfaceText,
        TranslationSurface::BattleCallout,
        TranslationSurface::FixedGaijiText,
        TranslationSurface::Dialogue,
        TranslationSurface::OpeningGlyphIndexPages,
        TranslationSurface::EndingGlyphIndexPages,
        TranslationSurface::BakedGraphicsText,
    ] {
        assert_eq!(excluded_non_game_text_reason(surface), None);
    }
}
