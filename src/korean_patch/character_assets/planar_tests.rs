use super::*;

#[test]
fn all_sixteen_palette_indices_round_trip_through_brgi_planes() {
    let indices = (0..16).collect::<Vec<_>>();
    let encoded = encode_compact_brgi_indices(&indices, 16, 1).unwrap();

    assert_eq!(
        decode_compact_brgi_indices(&encoded, 16, 1).unwrap(),
        indices
    );
}

#[test]
fn palette_index_above_fifteen_is_rejected() {
    let error = encode_compact_brgi_indices(&[16; 8], 8, 1).unwrap_err();

    assert!(error.to_string().contains("above 15"));
}
