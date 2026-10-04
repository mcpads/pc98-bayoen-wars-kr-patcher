use super::{
    DECOMPRESSOR, MAXIMUM_COM_IMAGE_SIZE, decode_packed_stream, pack_self_expanding_com,
    unpack_self_expanding_com,
};

#[test]
fn literals_overlap_and_zero_prefix_match_the_observed_stub() {
    let packed = [
        0x03, b'A', b'B', b'C', 0x80, 0x02, 0x82, 0x01, 0x80, 0xff, 0x00,
    ];

    let (decoded, consumed) = decode_packed_stream(&packed).unwrap();

    assert_eq!(decoded, b"ABCABCBCBCB\0\0\0");
    assert_eq!(consumed, packed.len());
}

#[test]
fn self_expanding_image_requires_the_complete_stream() {
    let mut bytes = DECOMPRESSOR.to_vec();
    bytes.extend_from_slice(&[0x01, b'X', 0x00]);
    let image = unpack_self_expanding_com(&bytes).unwrap();
    assert_eq!(image.packed_size, bytes.len());
    assert_eq!(image.unpacked, b"X");

    bytes.push(0xff);
    assert!(unpack_self_expanding_com(&bytes).is_err());
}

#[test]
fn truncated_literal_and_back_reference_are_rejected() {
    assert!(decode_packed_stream(&[0x02, b'A']).is_err());
    assert!(decode_packed_stream(&[0x80]).is_err());
}

#[test]
fn packer_round_trips_empty_and_command_boundaries() {
    for length in [0, 1, 0x7e, 0x7f, 0x80, 0xfe, 0xff, 0x100] {
        let unpacked = (0..length)
            .map(|index| u8::try_from(index & 0xff).unwrap())
            .collect::<Vec<_>>();

        let packed = pack_self_expanding_com(&unpacked).unwrap();
        let decoded = unpack_self_expanding_com(&packed).unwrap();

        assert_eq!(decoded.unpacked, unpacked);
    }
}

#[test]
fn packer_is_deterministic_and_uses_bounded_matches() {
    let unpacked = vec![0x5a; 0x84];
    let first = pack_self_expanding_com(&unpacked).unwrap();
    let second = pack_self_expanding_com(&unpacked).unwrap();

    assert_eq!(first, second);
    assert_eq!(first.get(DECOMPRESSOR.len()), Some(&0x01));
    assert_eq!(first.get(DECOMPRESSOR.len() + 1), Some(&0x5a));
    assert_eq!(first.get(DECOMPRESSOR.len() + 2), Some(&0xff));
    assert_eq!(first.get(DECOMPRESSOR.len() + 3), Some(&0x00));
    assert_eq!(first.get(DECOMPRESSOR.len() + 4), Some(&0x01));
    assert_eq!(first.get(DECOMPRESSOR.len() + 5), Some(&0x5a));
    assert_eq!(first.last(), Some(&0));
}

#[test]
fn packer_enforces_the_target_output_limit() {
    let maximum = vec![0; MAXIMUM_COM_IMAGE_SIZE];
    let packed = pack_self_expanding_com(&maximum).unwrap();
    assert_eq!(
        unpack_self_expanding_com(&packed).unwrap().unpacked,
        maximum
    );

    let oversized = vec![0; MAXIMUM_COM_IMAGE_SIZE + 1];
    assert!(pack_self_expanding_com(&oversized).is_err());
}
