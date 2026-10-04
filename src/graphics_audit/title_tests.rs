#[test]
fn title_transfers_stay_inside_the_consumer_linked_decoded_sizes() {
    for (decoded_size, source, width_bytes, height) in [
        (61_952, 0xef00, 2, 96),
        (61_952, 0x0000, 24, 224),
        (61_952, 0x5400, 16, 320),
        (61_952, 0xa400, 8, 240),
        (61_952, 0xc200, 16, 160),
        (61_952, 0xea00, 20, 16),
        (7_680, 0x0c00, 8, 48),
    ] {
        assert!(source + width_bytes * height * 4 <= decoded_size);
    }
}
