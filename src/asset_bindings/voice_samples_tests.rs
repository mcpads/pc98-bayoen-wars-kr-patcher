use super::*;

#[test]
fn binding_preserves_stable_identity_and_exact_stream_bytes() {
    let packed = [3, 1, 0, 0xab, 0];
    let stream = DecodedStream {
        packed_offset: 0,
        packed_size: packed.len(),
        command_count: 2,
        output: vec![1, 0, 0xab],
    };
    let review = SampleLanguageReview {
        id: "voice-sample-01".to_owned(),
        stream_index: 0,
        classification: SampleLanguageClassification::JapaneseVoicePerformance,
        transcript_status: SampleTranscriptStatus::NeedsIndependentReview,
    };

    let unit = bind_voice_sample(0, &packed, &stream, &review, 9_600).unwrap();

    assert_eq!(unit.id, "voice-sample-01");
    assert_eq!(unit.packed_raw_hex, "030100ab00");
    assert_eq!(unit.decoded_raw_hex, "0100ab");
    assert_eq!(unit.sample_count, 2);
    assert_eq!(unit.encoded_sample_bytes, 1);
}

#[test]
fn binding_rejects_a_review_for_another_stream() {
    let stream = DecodedStream {
        packed_offset: 0,
        packed_size: 1,
        command_count: 1,
        output: vec![0, 0],
    };
    let review = SampleLanguageReview {
        id: "voice-sample-02".to_owned(),
        stream_index: 1,
        classification: SampleLanguageClassification::JapaneseVoicePerformance,
        transcript_status: SampleTranscriptStatus::NeedsIndependentReview,
    };

    assert!(bind_voice_sample(0, &[0], &stream, &review, 9_600).is_err());
}
