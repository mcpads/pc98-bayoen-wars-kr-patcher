use super::*;

#[test]
fn shared_path_counts_only_non_shift_jis_characters_as_slots() {
    let mut demand = DemandAccumulator {
        entry_count: 1,
        ..DemandAccumulator::default()
    };
    demand.add_span("가A！ 나");

    let report = demand.report(TranslationRenderPath::SharedPc98Gaiji);

    assert_eq!(report.unique_rendered_character_count, 4);
    assert_eq!(report.unique_hangul_syllable_count, 2);
    assert_eq!(report.slot_glyph_count, Some(2));
    assert_eq!(report.reserved_source_glyph_count, 14);
    assert_eq!(report.available_translation_capacity, Some(170));
    assert_eq!(report.available_capacity_headroom, Some(168));
    assert_eq!(report.slot_glyph_characters, ["가", "나"]);
}

#[test]
fn dialogue_can_force_native_punctuation_into_full_cell_gaiji_slots() {
    let mut demand = DemandAccumulator::default();
    demand.add_span("가~…");
    demand.force_shared_gaiji(&DIALOGUE_GAIJI_PUNCTUATION);

    let report = demand.report(TranslationRenderPath::SharedPc98Gaiji);

    assert_eq!(report.slot_glyph_characters, ["~", "…", "가"]);
    assert_eq!(report.slot_glyph_count, Some(3));
}

#[test]
fn monochrome_sprite_path_counts_every_visible_character() {
    let mut demand = DemandAccumulator::default();
    demand.add_span("가 A!");

    let report = demand.report(TranslationRenderPath::OpeningMonochromeSprites);

    assert_eq!(report.slot_glyph_count, Some(4));
    assert_eq!(report.verified_physical_capacity, Some(254));
    assert_eq!(report.reserved_source_glyph_count, 0);
    assert_eq!(report.available_translation_capacity, Some(254));
    assert_eq!(report.physical_capacity_headroom, Some(250));
    assert!(report.slot_glyph_characters.contains(&" ".to_owned()));
}
