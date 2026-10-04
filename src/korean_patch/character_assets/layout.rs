use std::collections::BTreeSet;

use anyhow::{Context, Result, ensure};

use super::frame_placement::{FramePlacement, SubjectScale, VisibleBounds, frame_sampling};
use super::raster::RgbSheet;
use super::source_coordinates::{CoordinateExtent, SourceCoordinateTransform};
use crate::pc98_graphics::{DIGITAL_RGBI_COLOR_COUNT, nearest_16_color_palette_index};

#[derive(Debug, Eq, PartialEq)]
pub(super) struct ConvertedFrame {
    pub(super) indices: Vec<u8>,
    pub(super) visible_bounds: VisibleBounds,
    pub(super) used_palette_color_count: usize,
    pub(super) coordinate_transform: SourceCoordinateTransform,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct SheetLayout {
    pub(super) columns: usize,
    pub(super) rows: usize,
    pub(super) panel_width: usize,
    pub(super) panel_height: usize,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct AuthoringPanel {
    pub(super) sheet: SheetLayout,
    pub(super) frame_index: usize,
    pub(super) background: [u8; 3],
}

#[derive(Debug, Clone, Copy)]
pub(super) struct ConsumerFrame {
    pub(super) width: usize,
    pub(super) height: usize,
    pub(super) visible_bounds: VisibleBounds,
    pub(super) background_index: u8,
    pub(super) placement: FramePlacement,
    pub(super) source_coordinate_transform: Option<SourceCoordinateTransform>,
    pub(super) subject_scale: Option<SubjectScale>,
}

pub(super) fn indexed_visible_bounds(
    indices: &[u8],
    width: usize,
    height: usize,
    background_index: u8,
) -> Result<VisibleBounds> {
    ensure!(
        width > 0 && height > 0 && indices.len() == width * height,
        "indexed frame dimensions differ from its pixel population"
    );
    bounds_from_predicate(width, height, |offset| indices[offset] != background_index)
        .context("indexed frame has no foreground pixels")
}

pub(super) fn fit_authoring_panel(
    sheet: &RgbSheet,
    panel: AuthoringPanel,
    target: ConsumerFrame,
    palette: &[[u8; 3]; DIGITAL_RGBI_COLOR_COUNT],
) -> Result<ConvertedFrame> {
    let sheet_layout = panel.sheet;
    let target_bounds = target.visible_bounds;
    ensure!(
        sheet_layout.columns > 0
            && sheet_layout.rows > 0
            && sheet_layout.panel_width > 0
            && sheet_layout.panel_height > 0
            && sheet.width == sheet_layout.columns * sheet_layout.panel_width
            && sheet.height == sheet_layout.rows * sheet_layout.panel_height,
        "authoring sheet dimensions differ from its panel layout"
    );
    ensure!(
        panel.frame_index < sheet_layout.columns * sheet_layout.rows,
        "authoring frame index exceeds the sheet"
    );
    ensure!(
        target_bounds.width > 0
            && target_bounds.height > 0
            && target_bounds.x + target_bounds.width <= target.width
            && target_bounds.y + target_bounds.height <= target.height,
        "target visible bounds exceed the consumer frame"
    );
    ensure!(
        usize::from(target.background_index) < palette.len(),
        "character background index exceeds its palette"
    );

    let panel_x = panel.frame_index % sheet_layout.columns * sheet_layout.panel_width;
    let panel_y = panel.frame_index / sheet_layout.columns * sheet_layout.panel_height;
    let source_bounds = bounds_from_predicate(
        sheet_layout.panel_width,
        sheet_layout.panel_height,
        |offset| {
            let local_x = offset % sheet_layout.panel_width;
            let local_y = offset / sheet_layout.panel_width;
            rgb_at(sheet, panel_x + local_x, panel_y + local_y) != panel.background
        },
    )
    .context("authoring panel has no foreground pixels")?;

    let (coordinate_transform, sampling_window) = match target.placement {
        FramePlacement::PreserveSharedSourceCoordinatesCropFrame => {
            ensure!(
                target.subject_scale.is_none(),
                "shared source coordinate placement cannot declare a subject scale"
            );
            (
                target.source_coordinate_transform.context(
                    "shared source coordinate placement requires a prior frame transform",
                )?,
                None,
            )
        }
        _ => {
            ensure!(
                target.source_coordinate_transform.is_none(),
                "independent frame placement cannot reuse a source coordinate transform"
            );
            let sampling = frame_sampling(
                source_bounds,
                target_bounds,
                target.placement,
                target.subject_scale,
            )?;
            (
                sampling.coordinate_transform(source_bounds)?,
                Some(sampling),
            )
        }
    };
    let mut indices = vec![target.background_index; target.width * target.height];
    let target_extent = CoordinateExtent {
        width: target.width,
        height: target.height,
    };
    let source_extent = CoordinateExtent {
        width: sheet_layout.panel_width,
        height: sheet_layout.panel_height,
    };
    for target_y in 0..target.height {
        for target_x in 0..target.width {
            if sampling_window
                .is_some_and(|sampling| !sampling.contains_target_coordinate(target_x, target_y))
            {
                continue;
            }
            let Some((source_x, source_y)) = coordinate_transform.source_coordinate(
                (target_x, target_y),
                target_extent,
                source_extent,
            ) else {
                continue;
            };
            let color = rgb_at(sheet, panel_x + source_x, panel_y + source_y);
            indices[target_y * target.width + target_x] = if color == panel.background {
                target.background_index
            } else {
                nearest_16_color_palette_index(color, palette)
            };
        }
    }
    let visible_bounds = indexed_visible_bounds(
        &indices,
        target.width,
        target.height,
        target.background_index,
    )?;
    if target.placement != FramePlacement::PreserveSharedSourceCoordinatesCropFrame {
        ensure!(
            visible_bounds.x >= target_bounds.x
                && visible_bounds.y >= target_bounds.y
                && visible_bounds.x + visible_bounds.width <= target_bounds.x + target_bounds.width
                && visible_bounds.y + visible_bounds.height
                    <= target_bounds.y + target_bounds.height,
            "converted Arle frame escaped the source consumer silhouette bounds"
        );
    }
    let used_palette_color_count = indices.iter().copied().collect::<BTreeSet<_>>().len();
    ensure!(
        used_palette_color_count > 1,
        "converted Arle frame lost all foreground colors"
    );
    Ok(ConvertedFrame {
        indices,
        visible_bounds,
        used_palette_color_count,
        coordinate_transform,
    })
}

fn bounds_from_predicate(
    width: usize,
    height: usize,
    mut is_foreground: impl FnMut(usize) -> bool,
) -> Option<VisibleBounds> {
    let mut minimum_x = width;
    let mut minimum_y = height;
    let mut maximum_x = 0usize;
    let mut maximum_y = 0usize;
    let mut found = false;
    for y in 0..height {
        for x in 0..width {
            if !is_foreground(y * width + x) {
                continue;
            }
            found = true;
            minimum_x = minimum_x.min(x);
            minimum_y = minimum_y.min(y);
            maximum_x = maximum_x.max(x);
            maximum_y = maximum_y.max(y);
        }
    }
    if found {
        Some(VisibleBounds {
            x: minimum_x,
            y: minimum_y,
            width: maximum_x - minimum_x + 1,
            height: maximum_y - minimum_y + 1,
        })
    } else {
        None
    }
}

fn rgb_at(sheet: &RgbSheet, x: usize, y: usize) -> [u8; 3] {
    let offset = (y * sheet.width + x) * 3;
    [
        sheet.pixels[offset],
        sheet.pixels[offset + 1],
        sheet.pixels[offset + 2],
    ]
}

#[cfg(test)]
#[path = "layout_tests.rs"]
mod layout_tests;
