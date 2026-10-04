use super::*;

fn source_mad() -> Vec<u8> {
    let palette = encode_title_palette_table(&SOURCE_TITLE_PALETTE_RGB4).unwrap();
    let animation = encode_title_animation_frame_table(&SOURCE_TITLE_ANIMATION_FRAME_SOURCES);
    let mut mad = vec![0; TITLE_PALETTE_TABLE_FILE_OFFSET + palette.len()];
    mad[TITLE_PALETTE_TABLE_FILE_OFFSET..TITLE_PALETTE_TABLE_FILE_OFFSET + palette.len()]
        .copy_from_slice(&palette);
    mad[TITLE_ANIMATION_FRAME_TABLE_FILE_OFFSET
        ..TITLE_ANIMATION_FRAME_TABLE_FILE_OFFSET + animation.len()]
        .copy_from_slice(&animation);
    mad[TITLE_ANIMATION_END_FALLBACK_FILE_OFFSET..TITLE_ANIMATION_END_FALLBACK_FILE_OFFSET + 3]
        .copy_from_slice(&[0xb8, 0x00, 0x0c]);
    mad
}

fn apply_runtime_plan(files: &mut BTreeMap<String, Vec<u8>>, palette: &TitlePaletteRgb4) {
    let planned = title_art_runtime_plan(files, palette).unwrap();
    let typed_sources = planned.typed_sources;
    let verifier = v30::ExpectedWriteVerifier::new(move |source_id: &str| {
        typed_sources.get(source_id).cloned().ok_or_else(|| {
            expected_write::MachineCodeVerifierError::new(format!(
                "unknown test source {source_id}"
            ))
        })
    });
    let patched = planned
        .plan
        .plan
        .apply(&files[MAD_COM], Some(&verifier))
        .unwrap();
    files.insert(MAD_COM.to_owned(), patched);
}

#[test]
fn runtime_plan_disables_every_overlay_frame_and_installs_the_declared_palette() {
    let mut files = BTreeMap::from([(MAD_COM.to_owned(), source_mad())]);
    let mut palette = SOURCE_TITLE_PALETTE_RGB4;
    palette[3] = [0, 10, 9];

    apply_runtime_plan(&mut files, &palette);

    assert_eq!(read_title_palette_rgb4(&files[MAD_COM]).unwrap(), palette);
    assert!(title_animation_is_disabled(&files[MAD_COM]).unwrap());
    assert!(title_animation_end_fallback_is_disabled(&files[MAD_COM]).unwrap());
}

#[test]
fn runtime_plan_leaves_the_source_palette_without_a_noop_write() {
    let files = BTreeMap::from([(MAD_COM.to_owned(), source_mad())]);

    let planned = title_art_runtime_plan(&files, &SOURCE_TITLE_PALETTE_RGB4).unwrap();

    assert_eq!(planned.plan.plan.writes.len(), 2);
    assert_eq!(
        planned.plan.plan.writes[0].id,
        "disable-title2-animation-frames"
    );
    assert_eq!(
        planned.plan.plan.writes[1].id,
        "disable-title2-animation-end-fallback"
    );
}

#[test]
fn runtime_plan_rejects_a_different_source_palette_table() {
    let mut mad = source_mad();
    mad[TITLE_PALETTE_TABLE_FILE_OFFSET + 1] ^= 1;
    let files = BTreeMap::from([(MAD_COM.to_owned(), mad)]);

    let error = title_art_runtime_plan(&files, &SOURCE_TITLE_PALETTE_RGB4)
        .err()
        .expect("changed source palette must be rejected");

    assert!(
        error
            .to_string()
            .contains("source title palette table changed")
    );
}

#[test]
fn runtime_plan_rejects_a_different_source_animation_end_fallback() {
    let mut mad = source_mad();
    mad[TITLE_ANIMATION_END_FALLBACK_FILE_OFFSET + 1] ^= 1;
    let files = BTreeMap::from([(MAD_COM.to_owned(), mad)]);

    let error = title_art_runtime_plan(&files, &SOURCE_TITLE_PALETTE_RGB4)
        .err()
        .expect("changed source fallback must be rejected");

    assert!(
        error
            .to_string()
            .contains("source title animation end fallback changed")
    );
}
