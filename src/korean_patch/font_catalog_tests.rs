use super::*;

#[test]
#[ignore = "requires the Galmuri and Mulmaru font files in assets/fonts or BAYOEN_WARS_FONT_DIR"]
fn every_consumer_target_has_verified_provenance() {
    let cases = [
        (
            FontTarget::Body16,
            "6fe6c3fe4369e3837ac348431e8670733d67aa4bd550982baa72cc93c81a1c68",
            "bdb86ae89466a361eb8df861222736b81ff975ef",
        ),
        (
            FontTarget::Narrative32,
            "6fe6c3fe4369e3837ac348431e8670733d67aa4bd550982baa72cc93c81a1c68",
            "bdb86ae89466a361eb8df861222736b81ff975ef",
        ),
        (
            FontTarget::BakedDisplay16,
            "34a1641eb4e94449b26192321e8e0c2bd4f07ef3674fac8abed33d8953a7f70d",
            "ea9843cefa3a26fe71fd4abd7d4b46d82cb90354",
        ),
        (
            FontTarget::Difficulty32,
            "5265b2f437fe81f0c8095b44c0173dd9a276b58a42552bf983f21c0e69e6e8af",
            "c53c3038e200c5516c84c66cede840ae1c409b2d",
        ),
        (
            FontTarget::TitlePrimary16,
            "5265b2f437fe81f0c8095b44c0173dd9a276b58a42552bf983f21c0e69e6e8af",
            "c53c3038e200c5516c84c66cede840ae1c409b2d",
        ),
    ];

    for (target, expected_sha256, expected_revision) in cases {
        let provenance = font_provenance_for(target).unwrap();
        assert_eq!(provenance.font_sha256, expected_sha256, "{target:?}");
        assert_eq!(
            provenance.upstream_revision, expected_revision,
            "{target:?}"
        );
    }
}

#[test]
#[ignore = "requires the Galmuri and Mulmaru font files in assets/fonts or BAYOEN_WARS_FONT_DIR"]
fn profiles_are_bound_to_distinct_consumer_roles() {
    let body = load_font_profile(FontTarget::Body16).unwrap();
    let narrative = load_font_profile(FontTarget::Narrative32).unwrap();
    let display = load_font_profile(FontTarget::BakedDisplay16).unwrap();
    let difficulty = load_font_profile(FontTarget::Difficulty32).unwrap();
    let title = load_font_profile(FontTarget::TitlePrimary16).unwrap();

    assert_ne!(body.profile.id, narrative.profile.id);
    assert_ne!(body.profile.id, display.profile.id);
    assert_ne!(display.profile.id, difficulty.profile.id);
    assert_ne!(difficulty.profile.id, title.profile.id);
    assert_ne!(display.profile.id, title.profile.id);
}
