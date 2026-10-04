use crate::translation_drafts::TranslationSurface;

pub(super) const EXCLUDED_NON_GAME_TEXT_REASON: &str =
    "outside the adopted in-game translation scope; retained only as prior development material";

pub(super) fn excluded_non_game_text_reason(surface: TranslationSurface) -> Option<&'static str> {
    match surface {
        TranslationSurface::DosSystemText
        | TranslationSurface::MenuText
        | TranslationSurface::ExternalProgramText => Some(EXCLUDED_NON_GAME_TEXT_REASON),
        TranslationSurface::InterfaceText
        | TranslationSurface::BattleCallout
        | TranslationSurface::FixedGaijiText
        | TranslationSurface::Dialogue
        | TranslationSurface::MonochromeGlyphAtlas
        | TranslationSurface::OpeningGlyphIndexPages
        | TranslationSurface::EndingGlyphIndexPages
        | TranslationSurface::JapaneseVoicePerformance
        | TranslationSurface::BakedGraphicsText => None,
    }
}

#[cfg(test)]
#[path = "product_scope_tests.rs"]
mod product_scope_tests;
