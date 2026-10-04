use super::*;

#[test]
fn observed_sprite_populations_cover_the_decoded_sizes() {
    assert_eq!(16_512 / SPRITE_SIZE, 129);
    assert_eq!(8_832 / SPRITE_SIZE, 69);
}

#[test]
fn catalog_rejects_unverified_runtime_layout() {
    let error = catalog_monochrome_sprites(&vec![0; 0xcf00], &BTreeMap::new()).unwrap_err();

    assert!(
        error
            .to_string()
            .contains("monochrome decoded-buffer start")
    );
}
