use super::*;

#[test]
fn review_population_is_finite() {
    assert_eq!(REVIEWED_STREAM_COUNT, 22);
}

#[test]
fn incomplete_playback_population_is_rejected() {
    let playback = SamplePlaybackCatalog {
        source_asset: "SAMPA".to_owned(),
        playback_driver: "BSAMP.COM".to_owned(),
        stream_count: 21,
        total_decoded_bytes: 0,
        timer_setup_file_offset: 0,
        interrupt_handler_file_offset: 0,
        nibble_select_file_offset: 0,
        clock_profiles: Vec::new(),
        interrupt_ticks_per_sample: 2,
        sample_rate_hz: 9_600,
        sample_format: "unsigned_4_bit_pcm_high_nibble_first".to_owned(),
    };

    let error = catalog_sample_language_review(&playback).unwrap_err();

    assert!(
        error
            .to_string()
            .contains("complete playback stream population")
    );
}
