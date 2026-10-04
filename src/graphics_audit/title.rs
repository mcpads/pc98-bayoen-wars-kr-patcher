use std::collections::{BTreeMap, BTreeSet};

use anyhow::Result;

use super::Preview;
use super::decoded::decode_single_asset;
use super::planar::PlanarScreen;
use crate::title_runtime::{
    TITLE_ANIMATION_SKIP_SOURCE, expand_title_palette, read_title_animation_frame_sources,
    read_title_palette_rgb4,
};
use crate::title_screen::{TITLE_SCREEN_HEIGHT, TITLE_SCREEN_WIDTH, render_title_screen};
use crate::title_screen::{render_title_screen_with_animation, render_title_screen_with_palette};

pub(super) fn render_title_previews(
    mad_com: &[u8],
    installer_payload: &BTreeMap<String, Vec<u8>>,
) -> Result<Vec<Preview>> {
    let title = decode_single_asset(installer_payload, "TITLE.DAT")?;
    let title_animation = decode_single_asset(installer_payload, "TITLE2.DAT")?;
    let title_screen = render_title_screen(&title)?;
    let mut animation_screen = PlanarScreen::new();
    let animation_source = read_title_animation_frame_sources(mad_com)?
        .into_iter()
        .find(|source| *source != TITLE_ANIMATION_SKIP_SOURCE);
    if let Some(source) = animation_source {
        animation_screen.copy_compact_brgi(&title_animation, usize::from(source), 0x4b24, 8, 48)?;
    }

    Ok(vec![
        Preview {
            source_asset: "TITLE.DAT".to_owned(),
            output_file: "title-base.png".to_owned(),
            evidence:
                "MAD.COM file 0xAB6E title transfer sequence binds all visible base-title regions"
                    .to_owned(),
            image: super::planar::RgbImage {
                width: TITLE_SCREEN_WIDTH,
                height: TITLE_SCREEN_HEIGHT,
                pixels: title_screen.pixels,
            },
        },
        Preview {
            source_asset: "TITLE2.DAT".to_owned(),
            output_file: "title-animation-overlay.png".to_owned(),
            evidence: if animation_source.is_some() {
                "MAD.COM file 0xAC48 animation table selects a TITLE2 source as a 64x48 runtime overlay"
                    .to_owned()
            } else {
                "MAD.COM file 0xAC48 animation table skips all TITLE2 frames for replacement artwork"
                    .to_owned()
            },
            image: animation_screen.into_rgb(),
        },
    ])
}

pub(super) fn render_title_runtime_palette_preview(
    mad_com: &[u8],
    installer_payload: &BTreeMap<String, Vec<u8>>,
) -> Result<Preview> {
    let title = decode_single_asset(installer_payload, "TITLE.DAT")?;
    let palette = expand_title_palette(&read_title_palette_rgb4(mad_com)?);
    let title_screen = render_title_screen_with_palette(&title, &palette)?;
    Ok(Preview {
        source_asset: "TITLE.DAT".to_owned(),
        output_file: "title-runtime-palette.png".to_owned(),
        evidence: "MAD.COM file 0xACF0 title-scene RGB4 table binds the 16 stored indices to runtime colors"
            .to_owned(),
        image: super::planar::RgbImage {
            width: TITLE_SCREEN_WIDTH,
            height: TITLE_SCREEN_HEIGHT,
            pixels: title_screen.pixels,
        },
    })
}

pub(super) fn render_title_source_frame_previews(
    mad_com: &[u8],
    installer_payload: &BTreeMap<String, Vec<u8>>,
) -> Result<Vec<Preview>> {
    let title = decode_single_asset(installer_payload, "TITLE.DAT")?;
    let title_animation = decode_single_asset(installer_payload, "TITLE2.DAT")?;
    let palette = expand_title_palette(&read_title_palette_rgb4(mad_com)?);
    let frame_sources = read_title_animation_frame_sources(mad_com)?
        .into_iter()
        .filter(|source| *source != TITLE_ANIMATION_SKIP_SOURCE)
        .collect::<BTreeSet<_>>();

    frame_sources
        .into_iter()
        .map(|source| {
            let screen = render_title_screen_with_animation(
                &title,
                &title_animation,
                usize::from(source),
                &palette,
            )?;
            Ok(Preview {
                source_asset: "TITLE.DAT + TITLE2.DAT + MAD.COM".to_owned(),
                output_file: format!("title-source-frame-{source:04x}.png"),
                evidence: format!(
                    "source-extracted title composition using MAD.COM RGB4 palette and TITLE2 frame source {source:#06x}"
                ),
                image: super::planar::RgbImage {
                    width: TITLE_SCREEN_WIDTH,
                    height: TITLE_SCREEN_HEIGHT,
                    pixels: screen.pixels,
                },
            })
        })
        .collect()
}

#[cfg(test)]
#[path = "title_tests.rs"]
mod title_tests;
