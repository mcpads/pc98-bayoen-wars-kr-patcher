use std::collections::BTreeMap;

use super::scan_target_occurrences;

#[test]
fn target_scanner_keeps_duplicate_and_overlapping_loader_addresses() {
    let targets = BTreeMap::from([(0x1234, "first"), (0x3412, "second")]);

    let occurrences = scan_target_occurrences(&[0x34, 0x12, 0x34, 0x12], &targets);

    assert_eq!(
        occurrences,
        [
            (0, 0x1234, "first"),
            (1, 0x3412, "second"),
            (2, 0x1234, "first")
        ]
    );
}
