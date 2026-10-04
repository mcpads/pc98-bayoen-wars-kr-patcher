use std::path::Path;

use super::*;

#[test]
#[ignore = "requires the Arle battle sprite set in assets/characters/arle"]
fn tracked_authoring_sheet_matches_its_pinned_identity() {
    let sheet = load_rgb_sheet(
        Path::new("assets/characters/arle/battle-sprites-authoring.png"),
        "41e743d387e9d8f6a9da94c529c61d1dddc40fa96ab25ce09981bf56e1cd26df",
        1980,
        792,
    )
    .unwrap();

    assert_eq!(sheet.pixels.len(), 1980 * 792 * 3);
}

#[test]
#[ignore = "requires the Arle battle sprite set in assets/characters/arle"]
fn tracked_normalized_sheet_matches_the_build_input_identity() {
    let sheet = load_rgb_sheet(
        Path::new("assets/characters/arle/battle-sprites.pc98.png"),
        "abc6f393708efa54511b7a8cb1404979d576fce03c31f8e813475293be347cc7",
        960,
        384,
    )
    .unwrap();

    assert_eq!(sheet.pixels.len(), 960 * 384 * 3);
    let manifest = super::super::manifest::load_manifest(Path::new(
        "assets/characters/arle/battle-sprites.json",
    ))
    .unwrap();
    assert!(validate_declared_palette(&sheet, &manifest.source.palette_rgb).unwrap() > 1);
}

#[test]
fn normalized_sheet_rejects_colors_outside_the_declared_source_palette() {
    let error = validate_declared_palette(
        &RgbSheet {
            width: 1,
            height: 1,
            pixels: vec![1, 2, 3],
        },
        &[[0, 0, 0], [255, 255, 255]],
    )
    .unwrap_err();

    assert!(
        error
            .to_string()
            .contains("outside its declared source palette")
    );
}
