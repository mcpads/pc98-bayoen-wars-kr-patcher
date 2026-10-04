use super::*;

#[test]
fn title_transfers_cover_both_declared_text_regions() {
    for (x, y, width, height) in [(96, 96, 464, 48), (64, 144, 512, 112)] {
        for point in [
            (x, y),
            (x + width - 1, y),
            (x, y + height - 1),
            (x + width - 1, y + height - 1),
        ] {
            assert!(
                TITLE_CONTENT_TRANSFERS
                    .iter()
                    .any(|transfer| transfer.contains(point.0, point.1)),
                "missing title transfer for {point:?}"
            );
        }
    }
}

#[test]
#[ignore = "requires the Galmuri and Mulmaru font files in assets/fonts or BAYOEN_WARS_FONT_DIR"]
fn title_writer_rejects_a_region_outside_consumer_transfers() {
    let mut decoded = vec![0; TITLE_DECODED_SIZE];
    let canvas = TextCanvas::render(
        FontTarget::BakedDisplay16,
        &["가".to_owned()],
        16,
        16,
        1,
        false,
    )
    .unwrap();

    let error = write_title_region(&mut decoded, 0, 0, &canvas).unwrap_err();

    assert!(error.to_string().contains("has no source transfer"));
}
