use anyhow::{Context, Result, ensure};
use serde::Deserialize;

use super::source_coordinates::{CoordinateExtent, SourceCoordinateTransform};

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub(super) struct VisibleBounds {
    pub(super) x: usize,
    pub(super) y: usize,
    pub(super) width: usize,
    pub(super) height: usize,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct SubjectScale {
    pub(super) source_y: usize,
    pub(super) source_height: usize,
    pub(super) target_height: usize,
    pub(super) target_baseline_y: usize,
}

#[derive(Debug, Clone, Copy, Deserialize, Eq, PartialEq)]
pub(super) enum FramePlacement {
    #[serde(rename = "contain_consumer_visible_bounds_center_x_bottom_y")]
    ContainConsumerVisibleBounds,
    #[serde(rename = "match_consumer_visible_height_center_x_crop_overflow_bottom_y")]
    MatchConsumerVisibleHeightCropOverflow,
    #[serde(rename = "normalize_subject_height_center_x_align_baseline_crop_overflow_x")]
    NormalizeSubjectHeightAlignBaselineCropXOverflow,
    #[serde(rename = "preserve_shared_source_coordinates_crop_frame")]
    PreserveSharedSourceCoordinatesCropFrame,
}

impl FramePlacement {
    pub(super) const fn as_str(self) -> &'static str {
        match self {
            Self::ContainConsumerVisibleBounds => {
                "contain_consumer_visible_bounds_center_x_bottom_y"
            }
            Self::MatchConsumerVisibleHeightCropOverflow => {
                "match_consumer_visible_height_center_x_crop_overflow_bottom_y"
            }
            Self::NormalizeSubjectHeightAlignBaselineCropXOverflow => {
                "normalize_subject_height_center_x_align_baseline_crop_overflow_x"
            }
            Self::PreserveSharedSourceCoordinatesCropFrame => {
                "preserve_shared_source_coordinates_crop_frame"
            }
        }
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub(super) struct FrameSampling {
    target_x: usize,
    target_y: usize,
    target_width: usize,
    target_height: usize,
    scaled_source_width: usize,
    scaled_source_height: usize,
    scaled_source_crop_x: usize,
    scaled_source_crop_y: usize,
}

impl FrameSampling {
    pub(super) fn contains_target_coordinate(self, x: usize, y: usize) -> bool {
        x >= self.target_x
            && x < self.target_x + self.target_width
            && y >= self.target_y
            && y < self.target_y + self.target_height
    }

    pub(super) fn coordinate_transform(
        self,
        source: VisibleBounds,
    ) -> Result<SourceCoordinateTransform> {
        let target_anchor_x = i64::try_from(self.target_x)?
            .checked_sub(i64::try_from(self.scaled_source_crop_x)?)
            .context("character target x coordinate overflow")?;
        let target_anchor_y = i64::try_from(self.target_y)?
            .checked_sub(i64::try_from(self.scaled_source_crop_y)?)
            .context("character target y coordinate overflow")?;
        SourceCoordinateTransform::new(
            (source.x, source.y),
            CoordinateExtent {
                width: source.width,
                height: source.height,
            },
            (target_anchor_x, target_anchor_y),
            CoordinateExtent {
                width: self.scaled_source_width,
                height: self.scaled_source_height,
            },
        )
        .context("character source coordinate transform has an empty span")
    }
}

pub(super) fn frame_sampling(
    source: VisibleBounds,
    target: VisibleBounds,
    placement: FramePlacement,
    subject_scale: Option<SubjectScale>,
) -> Result<FrameSampling> {
    match placement {
        FramePlacement::ContainConsumerVisibleBounds => {
            ensure!(
                subject_scale.is_none(),
                "contain placement cannot declare a subject scale"
            );
            let (placed_width, placed_height) = fit_size(source, target)?;
            Ok(FrameSampling {
                target_x: target.x + (target.width - placed_width) / 2,
                target_y: target.y + target.height - placed_height,
                target_width: placed_width,
                target_height: placed_height,
                scaled_source_width: placed_width,
                scaled_source_height: placed_height,
                scaled_source_crop_x: 0,
                scaled_source_crop_y: 0,
            })
        }
        FramePlacement::MatchConsumerVisibleHeightCropOverflow => {
            ensure!(
                subject_scale.is_none(),
                "visible-height placement cannot declare a subject scale"
            );
            let scaled_width_numerator = source
                .width
                .checked_mul(target.height)
                .context("character width scaling overflow")?;
            let scaled_width = scaled_width_numerator
                .checked_add(source.height - 1)
                .context("character width rounding overflow")?
                / source.height;
            let placed_width = scaled_width.min(target.width).max(1);
            Ok(FrameSampling {
                target_x: target.x + (target.width - placed_width) / 2,
                target_y: target.y,
                target_width: placed_width,
                target_height: target.height,
                scaled_source_width: scaled_width,
                scaled_source_height: target.height,
                scaled_source_crop_x: (scaled_width - placed_width) / 2,
                scaled_source_crop_y: 0,
            })
        }
        FramePlacement::NormalizeSubjectHeightAlignBaselineCropXOverflow => {
            let subject_scale = subject_scale.context(
                "subject-height placement requires a normalized subject scale reference",
            )?;
            ensure!(
                subject_scale.source_height > 0 && subject_scale.target_height > 0,
                "normalized subject scale heights must be nonzero"
            );
            let scaled_width = source
                .width
                .checked_mul(subject_scale.target_height)
                .context("normalized subject width scaling overflow")?
                / subject_scale.source_height;
            let scaled_height = source
                .height
                .checked_mul(subject_scale.target_height)
                .context("normalized subject height scaling overflow")?
                / subject_scale.source_height;
            let scaled_width = scaled_width.max(1);
            let scaled_height = scaled_height.max(1);
            let subject_source_end = subject_scale
                .source_y
                .checked_add(subject_scale.source_height)
                .context("normalized subject reference end overflow")?;
            ensure!(
                subject_scale.source_y >= source.y
                    && subject_source_end <= source.y + source.height,
                "normalized subject reference escapes the authored foreground"
            );
            ensure!(
                scaled_height <= target.height,
                "normalized subject scaling would crop the composite vertically"
            );
            let sampled_subject_bottom = (0..scaled_height)
                .rfind(|local_y| {
                    let source_y = source.y + local_y * source.height / scaled_height;
                    source_y >= subject_scale.source_y && source_y < subject_source_end
                })
                .context("normalized subject reference has no sampled output row")?;
            let target_y = subject_scale
                .target_baseline_y
                .checked_sub(sampled_subject_bottom)
                .context("normalized subject baseline lies above its sampled body")?;
            ensure!(
                target_y >= target.y && target_y + scaled_height <= target.y + target.height,
                "normalized subject baseline would crop the composite vertically"
            );
            let placed_width = scaled_width.min(target.width);
            Ok(FrameSampling {
                target_x: target.x + (target.width - placed_width) / 2,
                target_y,
                target_width: placed_width,
                target_height: scaled_height,
                scaled_source_width: scaled_width,
                scaled_source_height: scaled_height,
                scaled_source_crop_x: (scaled_width - placed_width) / 2,
                scaled_source_crop_y: 0,
            })
        }
        FramePlacement::PreserveSharedSourceCoordinatesCropFrame => {
            anyhow::bail!("shared source coordinates do not have independent frame sampling")
        }
    }
}

fn fit_size(source: VisibleBounds, target: VisibleBounds) -> Result<(usize, usize)> {
    let source_width_scaled = source
        .width
        .checked_mul(target.height)
        .context("character width scaling overflow")?;
    let target_width_scaled = target
        .width
        .checked_mul(source.height)
        .context("character height scaling overflow")?;
    if source_width_scaled <= target_width_scaled {
        Ok(((source_width_scaled / source.height).max(1), target.height))
    } else {
        Ok((
            target.width,
            (source.height * target.width / source.width).max(1),
        ))
    }
}
