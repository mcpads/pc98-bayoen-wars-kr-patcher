use std::fs;

use super::*;

#[test]
#[ignore = "requires the Arle battle sprite set in assets/characters/arle"]
fn tracked_set_resolves_all_three_source_assets() {
    let resolved = ArleCharacterAssetSet::tracked().resolve().unwrap();

    assert!(resolved.manifest.ends_with(MANIFEST_FILENAME));
    assert!(resolved.artwork.ends_with(ARTWORK_FILENAME));
}

#[test]
fn directory_set_requires_the_complete_source_asset_set() {
    let directory = tempfile::tempdir().unwrap();
    for file_name in &ARLE_CHARACTER_ASSET_SET_FILENAMES[..2] {
        fs::write(directory.path().join(file_name), b"fixture").unwrap();
    }

    let error = ArleCharacterAssetSet::from_directory(directory.path())
        .resolve()
        .unwrap_err();

    assert!(error.to_string().contains("battle-sprites-authoring.png"));
}

#[test]
#[ignore = "requires the Arle battle sprite set in assets/characters/arle"]
fn tracked_set_rejects_a_changed_source_asset_identity() {
    let directory = tempfile::tempdir().unwrap();
    let tracked_directory = ArleCharacterAssetSet::tracked().directory().to_owned();
    for file_name in ARLE_CHARACTER_ASSET_SET_FILENAMES {
        fs::copy(
            tracked_directory.join(file_name),
            directory.path().join(file_name),
        )
        .unwrap();
    }
    fs::write(directory.path().join(MANIFEST_FILENAME), b"changed").unwrap();
    let selected = ArleCharacterAssetSet {
        source: ArleCharacterAssetSetSource::Tracked(directory.path().to_owned()),
    };

    let error = selected.resolve().unwrap_err();

    assert!(
        error
            .to_string()
            .contains("battle-sprites.json hash changed")
    );
}

#[test]
fn original_preservation_resolves_without_reading_custom_assets() {
    let selection = ArleCharacterAssetSelection::preserve_original()
        .resolve()
        .unwrap();

    assert_eq!(
        selection,
        ResolvedArleCharacterAssetSelection::PreserveOriginal
    );
}
