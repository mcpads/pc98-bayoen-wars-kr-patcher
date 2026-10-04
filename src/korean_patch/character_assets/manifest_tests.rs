use std::path::Path;

use super::*;

#[test]
#[ignore = "requires the Arle battle sprite set in assets/characters/arle"]
fn tracked_manifest_preserves_the_bayoen_arle_consumer_contract() {
    let manifest = load_manifest(Path::new("assets/characters/arle/battle-sprites.json")).unwrap();

    assert_eq!(manifest.frame_bindings.len(), 8);
    assert_eq!(manifest.consumer.slot_index, 6);
    assert_eq!(manifest.consumer.primary_asset, "C07");
    assert_eq!(manifest.consumer.secondary_asset, "C08");
    assert_eq!(
        manifest.consumer.palette_table_file_offset,
        CHARACTER_PALETTE_TABLE_FILE_OFFSET
    );
    assert_eq!(
        manifest.consumer.runtime_palette_rgb4,
        SOURCE_CHARACTER_PALETTE_RGB4
    );
    let c07_placements = manifest
        .frame_bindings
        .iter()
        .filter(|binding| binding.target_asset == "C07")
        .map(|binding| {
            (
                binding.role.as_str(),
                (
                    binding.placement,
                    binding.source_coordinate_reference.as_deref(),
                ),
            )
        })
        .collect::<BTreeMap<_, _>>();
    assert_eq!(
        c07_placements["idle"],
        (FramePlacement::ContainConsumerVisibleBounds, None)
    );
    assert_eq!(
        c07_placements["charge-ready"],
        (FramePlacement::ContainConsumerVisibleBounds, None)
    );
    assert_eq!(
        c07_placements["charge-initial"],
        (
            FramePlacement::PreserveSharedSourceCoordinatesCropFrame,
            Some("charge-ready")
        )
    );
    assert_eq!(
        c07_placements["charge-person-scale"],
        (FramePlacement::ContainConsumerVisibleBounds, None)
    );
    assert_eq!(
        c07_placements["charge-maximum"],
        (
            FramePlacement::PreserveSharedSourceCoordinatesCropFrame,
            Some("charge-person-scale")
        )
    );
    assert!(manifest.frame_bindings.iter().all(|binding| {
        binding.target_asset != "C08"
            || (binding.placement
                == FramePlacement::NormalizeSubjectHeightAlignBaselineCropXOverflow
                && binding.source_coordinate_reference.is_none())
    }));
    let c08_subject_heights = manifest
        .frame_bindings
        .iter()
        .filter(|binding| binding.target_asset == "C08")
        .map(|binding| {
            let reference = binding.subject_scale.as_ref().unwrap();
            (
                reference.source_bounds.height,
                reference.target_height,
                reference.target_baseline_y,
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(
        c08_subject_heights,
        [(161, 156, 178), (156, 156, 178), (150, 156, 178)]
    );
}

#[test]
#[ignore = "requires the Arle battle sprite set in assets/characters/arle"]
fn authoring_asset_cannot_escape_the_manifest_directory() {
    let path = Path::new("assets/characters/arle/battle-sprites.json");
    let mut manifest = load_manifest(path).unwrap();
    manifest.source.authoring_asset = "../battle-sprites-authoring.png".to_owned();

    let error = validate_authoring_asset(path, &manifest.source).unwrap_err();

    assert!(error.to_string().contains("relative to its manifest"));
}

#[test]
#[ignore = "requires the Arle battle sprite set in assets/characters/arle"]
fn compatible_custom_manifest_may_declare_its_own_provenance() {
    let path = Path::new("assets/characters/arle/battle-sprites.json");
    let mut manifest = load_manifest(path).unwrap();
    manifest.source.origin_repository = "user-owned-arle-assets".to_owned();
    manifest.source.origin_commit = "0".repeat(40);
    manifest.source.origin_asset = "battle-sprites.pc98.png".to_owned();
    manifest.source.origin_manifest = "battle-sprites.json".to_owned();
    manifest.source.origin_manifest_sha256 = "1".repeat(64);

    validate_manifest(&manifest).unwrap();
}

#[test]
#[ignore = "requires the Arle battle sprite set in assets/characters/arle"]
fn custom_manifest_provenance_requires_exact_hash_identities() {
    let path = Path::new("assets/characters/arle/battle-sprites.json");
    let mut manifest = load_manifest(path).unwrap();
    manifest.source.origin_manifest_sha256 = "not-a-sha256".to_owned();

    let error = validate_manifest(&manifest).unwrap_err();

    assert!(error.to_string().contains("provenance is incomplete"));
}
