use super::*;

#[test]
fn nearest_palette_mapping_preserves_every_exact_digital_rgbi_color() {
    let palette = digital_rgbi_palette();
    for index in 0..DIGITAL_RGBI_COLOR_COUNT {
        assert_eq!(
            nearest_16_color_palette_index(digital_rgbi_color(index), &palette),
            index as u8
        );
    }
}

#[test]
fn nearest_palette_mapping_uses_a_stable_lower_index_for_equal_distance() {
    assert_eq!(
        nearest_16_color_palette_index([0, 0, 85], &digital_rgbi_palette()),
        0
    );
}

#[test]
fn indexed_rgb4_palette_round_trips_all_sixteen_runtime_indices() {
    let palette = std::array::from_fn(|index| {
        [
            u8::try_from(index).unwrap(),
            u8::try_from(15 - index).unwrap(),
            u8::try_from(index / 2).unwrap(),
        ]
    });

    let encoded = encode_indexed_rgb4_palette(&palette).unwrap();

    assert_eq!(read_indexed_rgb4_palette(&encoded).unwrap(), palette);
    assert_eq!(expand_rgb4_palette(&palette)[8], [136, 119, 68]);
}

#[test]
fn indexed_rgb4_palette_rejects_changed_index_order() {
    let mut encoded = encode_indexed_rgb4_palette(&[[0; 3]; 16]).unwrap();
    encoded[4] = 2;

    let error = read_indexed_rgb4_palette(&encoded).unwrap_err();

    assert!(error.to_string().contains("index order changed"));
}
