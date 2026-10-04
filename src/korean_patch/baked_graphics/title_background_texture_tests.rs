use super::*;

#[test]
fn background_roles_restore_source_texture_without_restoring_source_logo() {
    let mut source = vec![2; TITLE_SCREEN_WIDTH * TITLE_SCREEN_HEIGHT];
    let mut target = source.clone();
    let transfer = TITLE_CONTENT_TRANSFERS[0];
    let edge_connected = (transfer.x(), transfer.y());
    let enclosed = (transfer.x() + 2, transfer.y() + 2);
    let source_logo = (transfer.x() + 4, transfer.y() + 2);
    let target_logo = (transfer.x() + 6, transfer.y() + 2);
    let outside = (0, 0);

    for y in 0..=edge_connected.1 {
        target[y * TITLE_SCREEN_WIDTH + edge_connected.0] = 1;
    }
    source[edge_connected.1 * TITLE_SCREEN_WIDTH + edge_connected.0] = 0;
    source[enclosed.1 * TITLE_SCREEN_WIDTH + enclosed.0] = 1;
    target[enclosed.1 * TITLE_SCREEN_WIDTH + enclosed.0] = 0;
    source[source_logo.1 * TITLE_SCREEN_WIDTH + source_logo.0] = 2;
    target[source_logo.1 * TITLE_SCREEN_WIDTH + source_logo.0] = 0;
    source[target_logo.1 * TITLE_SCREEN_WIDTH + target_logo.0] = 0;
    target[target_logo.1 * TITLE_SCREEN_WIDTH + target_logo.0] = 2;
    source[outside.1 * TITLE_SCREEN_WIDTH + outside.0] = 1;
    target[outside.1 * TITLE_SCREEN_WIDTH + outside.0] = 0;

    let stats = restore_edge_connected_background_texture(&source, &mut target, &[0, 1]).unwrap();

    assert_eq!(
        target[edge_connected.1 * TITLE_SCREEN_WIDTH + edge_connected.0],
        0
    );
    assert_eq!(target[enclosed.1 * TITLE_SCREEN_WIDTH + enclosed.0], 0);
    assert_eq!(
        target[source_logo.1 * TITLE_SCREEN_WIDTH + source_logo.0],
        0
    );
    assert_eq!(
        target[target_logo.1 * TITLE_SCREEN_WIDTH + target_logo.0],
        2
    );
    assert_eq!(target[outside.1 * TITLE_SCREEN_WIDTH + outside.0], 0);
    assert_eq!(
        stats,
        TitleBackgroundTextureStats {
            edge_connected_background_pixels_preserved: 1,
            edge_connected_background_pixel_indices_restored: 1,
            artwork_owned_background_pixels_retained: 1,
            source_logo_pixels_replaced_with_background: 1,
        }
    );
}

#[test]
fn background_preservation_rejects_incomplete_frames_before_mutating_target() {
    let source = vec![0; TITLE_SCREEN_WIDTH * TITLE_SCREEN_HEIGHT - 1];
    let mut target = vec![1; TITLE_SCREEN_WIDTH * TITLE_SCREEN_HEIGHT];
    let original = target.clone();

    let error =
        restore_edge_connected_background_texture(&source, &mut target, &[0, 1]).unwrap_err();

    assert!(error.to_string().contains("complete 640x400"));
    assert_eq!(target, original);
}

#[test]
fn enclosed_black_logo_shadow_is_not_replaced_with_source_background_texture() {
    let mut source = vec![2; TITLE_SCREEN_WIDTH * TITLE_SCREEN_HEIGHT];
    let mut target = source.clone();
    let transfer = TITLE_CONTENT_TRANSFERS[0];
    let center = (transfer.x() + 8, transfer.y() + 8);

    for y in center.1 - 1..=center.1 + 1 {
        for x in center.0 - 1..=center.0 + 1 {
            target[y * TITLE_SCREEN_WIDTH + x] = 2;
        }
    }
    source[center.1 * TITLE_SCREEN_WIDTH + center.0] = 1;
    target[center.1 * TITLE_SCREEN_WIDTH + center.0] = 0;

    restore_edge_connected_background_texture(&source, &mut target, &[0, 1]).unwrap();

    assert_eq!(
        target[center.1 * TITLE_SCREEN_WIDTH + center.0],
        0,
        "black artwork enclosed by non-background logo pixels must remain artwork-owned"
    );
}
