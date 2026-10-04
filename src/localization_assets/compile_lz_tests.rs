use super::*;

#[test]
fn decoder_matches_literal_overlap_and_zero_fill_behavior() {
    let streams = decode_all_streams(&[
        2, b'A', b'B', 0x82, 1, 0, // ABABABA
        0x80, 0, 0, // three leading zero bytes
    ])
    .unwrap();

    assert_eq!(
        streams,
        vec![
            DecodedStream {
                packed_offset: 0,
                packed_size: 6,
                command_count: 3,
                output: b"ABABABA".to_vec(),
            },
            DecodedStream {
                packed_offset: 6,
                packed_size: 3,
                command_count: 2,
                output: vec![0; 3],
            },
        ]
    );
}

#[test]
fn truncated_literal_is_rejected_at_its_command() {
    let error = decode_all_streams(&[3, b'A', b'B']).unwrap_err();

    assert!(
        error
            .to_string()
            .contains("Compile LZ literal at 0x0 exceeds the packed input")
    );
}

#[test]
fn missing_stream_terminator_is_rejected() {
    let error = decode_all_streams(&[1, b'A']).unwrap_err();

    assert!(
        error
            .to_string()
            .contains("stream at 0x0 has no terminator")
    );
}

#[test]
fn encoder_round_trips_literals_repeats_and_empty_input() {
    for input in [
        Vec::new(),
        b"literal text without a useful repeat".to_vec(),
        vec![0x5a; 500],
        (0..1000).map(|index| (index * 37 + 11) as u8).collect(),
    ] {
        let encoded = encode_single_stream(&input).unwrap();
        let decoded = decode_all_streams(&encoded).unwrap();

        assert_eq!(encoded.last(), Some(&0));
        assert_eq!(decoded.len(), 1);
        assert_eq!(decoded[0].packed_size, encoded.len());
        assert_eq!(decoded[0].output, input);
    }
}

#[test]
fn encoder_uses_overlapping_back_references() {
    let input = vec![0xa5; 500];

    let encoded = encode_single_stream(&input).unwrap();

    assert!(encoded.len() < input.len());
    assert!(encoded.iter().any(|command| *command >= 0x80));
}
