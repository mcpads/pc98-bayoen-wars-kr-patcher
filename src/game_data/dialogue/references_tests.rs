use super::*;

#[test]
fn target_scanner_keeps_overlapping_occurrences_and_target_identity() {
    let targets = [(0x1234, "first"), (0x3412, "second")]
        .into_iter()
        .collect();

    let occurrences = scan_target_occurrences(&[0x34, 0x12, 0x34], &targets);

    assert_eq!(occurrences, [(0, 0x1234, "first"), (1, 0x3412, "second")]);
}
