use super::*;

#[test]
fn both_timer_profiles_produce_the_same_sample_rate() {
    for (clock_hz, divisor) in CLOCK_PROFILES {
        assert_eq!(clock_hz / u32::from(divisor), 19_200);
        assert_eq!(
            clock_hz / u32::from(divisor) / INTERRUPT_TICKS_PER_SAMPLE,
            SAMPLE_RATE_HZ
        );
    }
}

#[test]
fn catalog_rejects_missing_sample_asset() {
    let error = catalog_sample_playback(&BTreeMap::new(), &BTreeMap::new()).unwrap_err();

    assert!(error.to_string().contains(SAMPLE_ASSET));
}
