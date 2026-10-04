use super::*;

fn transform() -> SourceCoordinateTransform {
    SourceCoordinateTransform::new(
        (4, 3),
        CoordinateExtent {
            width: 12,
            height: 14,
        },
        (1, 5),
        CoordinateExtent {
            width: 10,
            height: 12,
        },
    )
    .unwrap()
}

#[test]
fn reused_transform_keeps_source_coordinates_fixed_across_target_extents() {
    let transform = transform();
    let source_extent = CoordinateExtent {
        width: 32,
        height: 32,
    };

    let long_frame = transform.source_coordinate(
        (8, 10),
        CoordinateExtent {
            width: 16,
            height: 20,
        },
        source_extent,
    );
    let short_frame = transform.source_coordinate(
        (8, 10),
        CoordinateExtent {
            width: 16,
            height: 12,
        },
        source_extent,
    );

    assert_eq!(short_frame, long_frame);
}

#[test]
fn shorter_target_extent_clips_rows_without_rescaling_coordinates() {
    let transform = transform();
    let source_extent = CoordinateExtent {
        width: 32,
        height: 32,
    };

    assert!(
        transform
            .source_coordinate(
                (8, 15),
                CoordinateExtent {
                    width: 16,
                    height: 20,
                },
                source_extent,
            )
            .is_some()
    );
    assert_eq!(
        transform.source_coordinate(
            (8, 15),
            CoordinateExtent {
                width: 16,
                height: 12,
            },
            source_extent,
        ),
        None
    );
}
