use super::*;

#[test]
#[ignore = "requires the project title artwork in assets/title_art"]
fn tracked_project_authored_frame_matches_its_manifest_identity() {
    let project_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let manifest_bytes =
        std::fs::read(project_root.join("assets/title_art/development-title-art.json")).unwrap();
    let manifest: TitleArtworkManifest = serde_json::from_slice(&manifest_bytes).unwrap();
    let artwork_bytes = read_matching_file(
        &project_root.join("assets/title_art/title-authored-frame.png"),
        &manifest.artwork.sha256,
        "project-authored title frame",
    )
    .unwrap();
    let artwork = decode_rgb_png(&artwork_bytes, "project-authored title frame").unwrap();

    assert_eq!(manifest.artwork.kind, "project_authored_consumer_frame");
    assert_eq!((artwork.width, artwork.height), (1280, 800));
    assert_eq!(
        (manifest.artwork.width, manifest.artwork.height),
        (artwork.width, artwork.height)
    );
    assert_eq!(
        (
            manifest.artwork.conversion.source_region.x,
            manifest.artwork.conversion.source_region.y,
            manifest.artwork.conversion.source_region.width,
            manifest.artwork.conversion.source_region.height,
        ),
        (143, 12, 1280, 800)
    );
    assert_eq!(
        sha256_hex(&artwork.pixels),
        manifest.artwork.conversion.output_rgb_sha256
    );
}

#[test]
fn artwork_file_must_match_the_manifest_digest() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("artwork.png");
    std::fs::write(&path, b"not the declared artwork").unwrap();

    let error = read_matching_file(&path, &"00".repeat(32), "generated title artwork").unwrap_err();

    assert!(error.to_string().contains("SHA-256 differs"));
}
