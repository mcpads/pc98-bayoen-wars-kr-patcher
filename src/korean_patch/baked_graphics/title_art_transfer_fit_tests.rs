use super::*;

#[test]
fn neutral_backing_outside_transfers_restores_the_source_without_touching_owned_content() {
    let mut source = vec![1; TITLE_SCREEN_WIDTH * TITLE_SCREEN_HEIGHT];
    let mut target = source.clone();
    let outside = (0, 0);
    let inside_transfer = (
        TITLE_CONTENT_TRANSFERS[0].x(),
        TITLE_CONTENT_TRANSFERS[0].y(),
    );
    source[outside.1 * TITLE_SCREEN_WIDTH + outside.0] = 0;
    target[outside.1 * TITLE_SCREEN_WIDTH + outside.0] = 4;
    target[inside_transfer.1 * TITLE_SCREEN_WIDTH + inside_transfer.0] = 8;

    let stats =
        preserve_source_outside_title_transfers(&source, &mut target, &[0, 1], &[2, 3, 4, 5, 6])
            .unwrap();

    assert_eq!(
        stats,
        TitleTransferFitStats {
            source_pixels_restored_outside_transfers: 1,
        }
    );
    assert_eq!(target[outside.1 * TITLE_SCREEN_WIDTH + outside.0], 0);
    assert_eq!(
        target[inside_transfer.1 * TITLE_SCREEN_WIDTH + inside_transfer.0],
        8
    );
}

#[test]
fn essential_artwork_outside_transfers_fails_before_mutating_the_frame() {
    let source = vec![1; TITLE_SCREEN_WIDTH * TITLE_SCREEN_HEIGHT];
    let mut target = source.clone();
    target[0] = 4;
    target[1] = 8;
    let original = target.clone();

    let error =
        preserve_source_outside_title_transfers(&source, &mut target, &[0, 1], &[2, 3, 4, 5, 6])
            .unwrap_err();

    assert!(error.to_string().contains("essential palette index 8"));
    assert_eq!(target, original);
}

#[test]
fn transfer_fitting_rejects_incomplete_frames() {
    let source = vec![0; TITLE_SCREEN_WIDTH * TITLE_SCREEN_HEIGHT - 1];
    let mut target = vec![0; TITLE_SCREEN_WIDTH * TITLE_SCREEN_HEIGHT];

    let error =
        preserve_source_outside_title_transfers(&source, &mut target, &[0, 1], &[2, 3, 4, 5, 6])
            .unwrap_err();

    assert!(error.to_string().contains("complete 640x400"));
}
