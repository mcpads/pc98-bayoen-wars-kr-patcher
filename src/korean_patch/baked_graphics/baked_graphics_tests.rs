use super::*;
use crate::korean_patch::font_catalog::font_provenance_for;

#[test]
fn baked_asset_names_are_separate_consumer_files() {
    assert_eq!(
        [TITLE_FILE, DIFFICULTY_FILE, SELECTION_FILE],
        ["TITLE.DAT", "SEL1.DAT", "SEL3.DAT"]
    );
}

#[test]
#[ignore = "requires the Galmuri and Mulmaru font files in assets/fonts or BAYOEN_WARS_FONT_DIR"]
fn baked_font_roles_report_their_exact_consumer_targets() {
    let targets = [
        FontTarget::BakedDisplay16,
        FontTarget::Difficulty32,
        FontTarget::TitlePrimary16,
    ]
    .into_iter()
    .map(|target| (target.id(), font_provenance_for(target).unwrap().profile_id))
    .collect::<Vec<_>>();

    assert_eq!(
        targets,
        vec![
            (
                "baked-display16",
                "baked-display16-mulmaru-mono-v1.0-14px".to_owned(),
            ),
            (
                "difficulty32",
                "difficulty32-galmuri11-bold-v2.40.3-28px".to_owned(),
            ),
            (
                "title-primary16",
                "title-primary16-galmuri11-bold-v2.40.3-15px".to_owned(),
            ),
        ]
    );
}
