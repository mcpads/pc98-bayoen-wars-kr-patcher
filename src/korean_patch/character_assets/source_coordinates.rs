#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub(super) struct SourceCoordinateTransform {
    source_anchor_x: usize,
    source_anchor_y: usize,
    source_span_width: usize,
    source_span_height: usize,
    target_anchor_x: i64,
    target_anchor_y: i64,
    target_span_width: usize,
    target_span_height: usize,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub(super) struct CoordinateExtent {
    pub(super) width: usize,
    pub(super) height: usize,
}

impl SourceCoordinateTransform {
    pub(super) fn new(
        source_anchor: (usize, usize),
        source_span: CoordinateExtent,
        target_anchor: (i64, i64),
        target_span: CoordinateExtent,
    ) -> Option<Self> {
        if source_span.width == 0
            || source_span.height == 0
            || target_span.width == 0
            || target_span.height == 0
        {
            return None;
        }
        Some(Self {
            source_anchor_x: source_anchor.0,
            source_anchor_y: source_anchor.1,
            source_span_width: source_span.width,
            source_span_height: source_span.height,
            target_anchor_x: target_anchor.0,
            target_anchor_y: target_anchor.1,
            target_span_width: target_span.width,
            target_span_height: target_span.height,
        })
    }

    pub(super) fn source_coordinate(
        self,
        target: (usize, usize),
        target_extent: CoordinateExtent,
        source_extent: CoordinateExtent,
    ) -> Option<(usize, usize)> {
        if target.0 >= target_extent.width || target.1 >= target_extent.height {
            return None;
        }
        let source_x = map_axis(
            target.0,
            self.target_anchor_x,
            self.target_span_width,
            self.source_anchor_x,
            self.source_span_width,
            source_extent.width,
        )?;
        let source_y = map_axis(
            target.1,
            self.target_anchor_y,
            self.target_span_height,
            self.source_anchor_y,
            self.source_span_height,
            source_extent.height,
        )?;
        Some((source_x, source_y))
    }
}

fn map_axis(
    target_coordinate: usize,
    target_anchor: i64,
    target_span: usize,
    source_anchor: usize,
    source_span: usize,
    source_extent: usize,
) -> Option<usize> {
    let target_coordinate = i64::try_from(target_coordinate).ok()?;
    let target_span = i64::try_from(target_span).ok()?;
    let source_anchor = i64::try_from(source_anchor).ok()?;
    let source_span = i64::try_from(source_span).ok()?;
    let scaled_offset = target_coordinate
        .checked_sub(target_anchor)?
        .checked_mul(source_span)?
        .div_euclid(target_span);
    let source_coordinate = source_anchor.checked_add(scaled_offset)?;
    let source_coordinate = usize::try_from(source_coordinate).ok()?;
    (source_coordinate < source_extent).then_some(source_coordinate)
}

#[cfg(test)]
#[path = "source_coordinates_tests.rs"]
mod source_coordinates_tests;
