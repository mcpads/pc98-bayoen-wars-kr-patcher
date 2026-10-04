use super::*;
use crate::title_runtime::{SOURCE_TITLE_PALETTE_RGB4, expand_title_palette};

fn source_title_palette() -> [[u8; 3]; 16] {
    expand_title_palette(&SOURCE_TITLE_PALETTE_RGB4)
}

fn frame(width: usize, height: usize, pixels: Vec<[u8; 3]>) -> RgbFrame {
    RgbFrame {
        width,
        height,
        pixels: pixels.into_iter().flatten().collect(),
    }
}

#[test]
fn whole_rgb_frame_is_inset_without_foreground_or_background_separation() {
    let palette = source_title_palette();
    let artwork = frame(2, 1, vec![palette[10], palette[2]]);

    let indices = place_and_quantize_title_frame(
        &artwork,
        PixelRect {
            x: 0,
            y: 0,
            width: 2,
            height: 1,
        },
        &[],
        TitleFrameLayout {
            sampling_width: 4,
            sampling_height: 2,
            placement: SamplingPlacement {
                x: 1,
                y: 0,
                width: 2,
                height: 1,
            },
            output_width: 8,
            output_height: 4,
            background_palette_index: 0,
        },
        &palette,
    )
    .unwrap();

    assert_eq!(&indices[..8], &[0, 0, 10, 10, 2, 2, 0, 0]);
    assert_eq!(&indices[8..16], &[0, 0, 10, 10, 2, 2, 0, 0]);
    assert!(indices[16..].iter().all(|index| *index == 0));
}

#[test]
fn declared_crop_is_sampled_before_whole_frame_placement() {
    let palette = source_title_palette();
    let artwork = frame(4, 1, vec![palette[2], palette[10], palette[10], palette[2]]);

    let indices = place_and_quantize_title_frame(
        &artwork,
        PixelRect {
            x: 1,
            y: 0,
            width: 2,
            height: 1,
        },
        &[],
        TitleFrameLayout {
            sampling_width: 2,
            sampling_height: 1,
            placement: SamplingPlacement {
                x: 0,
                y: 0,
                width: 2,
                height: 1,
            },
            output_width: 4,
            output_height: 2,
            background_palette_index: 0,
        },
        &palette,
    )
    .unwrap();

    assert!(indices.iter().all(|index| *index == 10));
}

#[test]
fn preserved_component_region_is_left_for_the_original_consumer() {
    let palette = source_title_palette();
    let artwork = frame(2, 1, vec![palette[10], palette[2]]);

    let indices = place_and_quantize_title_frame(
        &artwork,
        PixelRect {
            x: 0,
            y: 0,
            width: 2,
            height: 1,
        },
        &[PixelRect {
            x: 1,
            y: 0,
            width: 1,
            height: 1,
        }],
        TitleFrameLayout {
            sampling_width: 2,
            sampling_height: 1,
            placement: SamplingPlacement {
                x: 0,
                y: 0,
                width: 2,
                height: 1,
            },
            output_width: 4,
            output_height: 2,
            background_palette_index: 0,
        },
        &palette,
    )
    .unwrap();

    assert_eq!(indices, [10, 10, 0, 0, 10, 10, 0, 0]);
}

#[test]
fn placement_outside_the_sampling_canvas_is_rejected() {
    let palette = source_title_palette();
    let artwork = frame(1, 1, vec![palette[10]]);

    let error = place_and_quantize_title_frame(
        &artwork,
        PixelRect {
            x: 0,
            y: 0,
            width: 1,
            height: 1,
        },
        &[],
        TitleFrameLayout {
            sampling_width: 2,
            sampling_height: 1,
            placement: SamplingPlacement {
                x: 1,
                y: 0,
                width: 2,
                height: 1,
            },
            output_width: 4,
            output_height: 2,
            background_palette_index: 0,
        },
        &palette,
    )
    .unwrap_err();

    assert!(error.to_string().contains("exceeds its sampling canvas"));
}
