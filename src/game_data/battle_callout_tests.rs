use super::*;

#[test]
fn callout_pointer_table_shape_is_fixed_by_the_consumer() {
    assert_eq!(UNIT_COUNT * CALLOUTS_PER_UNIT, 72);
    assert_eq!(POINTER_TABLE_OFFSET + POINTER_COUNT * 2, TEXT_START_OFFSET);
}

#[test]
fn renderer_calls_are_not_inferred_from_text_frequency() {
    assert_eq!(CALL_SITE_OFFSETS.len(), 6);
    assert!(CALL_SITE_OFFSETS.windows(2).all(|pair| pair[0] < pair[1]));
}
