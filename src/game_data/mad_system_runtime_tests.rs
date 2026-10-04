use std::collections::BTreeMap;

use super::scan_target_occurrences;

#[test]
fn target_scanner_preserves_base_offsets_and_overlapping_words() {
    let targets = BTreeMap::from([(0x1234, "first"), (0x3412, "second")]);

    let occurrences = scan_target_occurrences(&[0x34, 0x12, 0x34], 0x200, &targets);

    assert_eq!(
        occurrences,
        [(0x200, 0x1234, "first"), (0x201, 0x3412, "second")]
    );
}
