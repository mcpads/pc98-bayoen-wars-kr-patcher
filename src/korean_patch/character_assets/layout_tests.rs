use super::*;
use crate::pc98_graphics::digital_rgbi_palette;

#[test]
fn authoring_foreground_is_aspect_fitted_and_bottom_aligned_inside_source_bounds() {
    let mut pixels = vec![0; 4 * 4 * 3];
    for y in 1..3 {
        let offset = (y * 4 + 2) * 3;
        pixels[offset..offset + 3].copy_from_slice(&[255, 255, 255]);
    }
    let sheet = RgbSheet {
        width: 4,
        height: 4,
        pixels,
    };
    let converted = fit_authoring_panel(
        &sheet,
        AuthoringPanel {
            sheet: SheetLayout {
                columns: 1,
                rows: 1,
                panel_width: 4,
                panel_height: 4,
            },
            frame_index: 0,
            background: [0, 0, 0],
        },
        ConsumerFrame {
            width: 8,
            height: 8,
            visible_bounds: VisibleBounds {
                x: 2,
                y: 1,
                width: 4,
                height: 6,
            },
            background_index: 0,
            placement: FramePlacement::ContainConsumerVisibleBounds,
            source_coordinate_transform: None,
            subject_scale: None,
        },
        &digital_rgbi_palette(),
    )
    .unwrap();

    assert_eq!(
        converted.visible_bounds,
        VisibleBounds {
            x: 2,
            y: 1,
            width: 3,
            height: 6,
        }
    );
    assert_eq!(converted.indices[7 * 8 + 3], 0);
}

#[test]
fn height_matched_authoring_foreground_is_cropped_horizontally_without_shrinking() {
    let mut pixels = vec![255; 8 * 4 * 3];
    for y in 0..4 {
        for x in [0, 1, 6, 7] {
            let offset = (y * 8 + x) * 3;
            pixels[offset..offset + 3].copy_from_slice(&[255, 0, 0]);
        }
    }
    let sheet = RgbSheet {
        width: 8,
        height: 4,
        pixels,
    };
    let palette = digital_rgbi_palette();
    let cropped_color = nearest_16_color_palette_index([255, 0, 0], &palette);
    let converted = fit_authoring_panel(
        &sheet,
        AuthoringPanel {
            sheet: SheetLayout {
                columns: 1,
                rows: 1,
                panel_width: 8,
                panel_height: 4,
            },
            frame_index: 0,
            background: [0, 0, 0],
        },
        ConsumerFrame {
            width: 8,
            height: 8,
            visible_bounds: VisibleBounds {
                x: 2,
                y: 1,
                width: 4,
                height: 6,
            },
            background_index: 0,
            placement: FramePlacement::MatchConsumerVisibleHeightCropOverflow,
            source_coordinate_transform: None,
            subject_scale: None,
        },
        &palette,
    )
    .unwrap();

    assert_eq!(
        converted.visible_bounds,
        VisibleBounds {
            x: 2,
            y: 1,
            width: 4,
            height: 6,
        }
    );
    assert!(!converted.indices.contains(&cropped_color));
}

#[test]
fn normalized_subject_height_and_baseline_are_preserved_with_effects_below() {
    let mut pixels = vec![0; 8 * 4 * 3];
    for y in 1..3 {
        for x in 1..7 {
            let offset = (y * 8 + x) * 3;
            pixels[offset..offset + 3].copy_from_slice(&[255, 255, 255]);
        }
    }
    for x in 1..7 {
        let offset = (3 * 8 + x) * 3;
        pixels[offset..offset + 3].copy_from_slice(&[255, 0, 0]);
    }
    let sheet = RgbSheet {
        width: 8,
        height: 4,
        pixels,
    };
    let converted = fit_authoring_panel(
        &sheet,
        AuthoringPanel {
            sheet: SheetLayout {
                columns: 1,
                rows: 1,
                panel_width: 8,
                panel_height: 4,
            },
            frame_index: 0,
            background: [0, 0, 0],
        },
        ConsumerFrame {
            width: 4,
            height: 4,
            visible_bounds: VisibleBounds {
                x: 0,
                y: 0,
                width: 4,
                height: 4,
            },
            background_index: 0,
            placement: FramePlacement::NormalizeSubjectHeightAlignBaselineCropXOverflow,
            source_coordinate_transform: None,
            subject_scale: Some(SubjectScale {
                source_y: 1,
                source_height: 2,
                target_height: 3,
                target_baseline_y: 2,
            }),
        },
        &digital_rgbi_palette(),
    )
    .unwrap();

    assert_eq!(
        converted.visible_bounds,
        VisibleBounds {
            x: 0,
            y: 0,
            width: 4,
            height: 4,
        }
    );
    let red = nearest_16_color_palette_index([255, 0, 0], &digital_rgbi_palette());
    assert_ne!(converted.indices[2 * 4], 0);
    assert_eq!(converted.indices[3 * 4], red);
}

#[test]
fn empty_authoring_panel_is_rejected() {
    let error = fit_authoring_panel(
        &RgbSheet {
            width: 1,
            height: 1,
            pixels: vec![0, 0, 0],
        },
        AuthoringPanel {
            sheet: SheetLayout {
                columns: 1,
                rows: 1,
                panel_width: 1,
                panel_height: 1,
            },
            frame_index: 0,
            background: [0, 0, 0],
        },
        ConsumerFrame {
            width: 8,
            height: 1,
            visible_bounds: VisibleBounds {
                x: 0,
                y: 0,
                width: 8,
                height: 1,
            },
            background_index: 0,
            placement: FramePlacement::ContainConsumerVisibleBounds,
            source_coordinate_transform: None,
            subject_scale: None,
        },
        &digital_rgbi_palette(),
    )
    .unwrap_err();

    assert!(error.to_string().contains("no foreground"));
}
