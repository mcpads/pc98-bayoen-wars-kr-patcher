use super::*;

#[test]
fn diagnostic_text_population_is_finite_and_ascii() {
    assert_eq!(DIAGNOSTICS.len(), 14);
    assert_eq!(FLAG_LABELS.len(), 8);
    assert!(DIAGNOSTICS.iter().all(|(_, text)| text.is_ascii()));
    assert!(
        FLAG_LABELS
            .iter()
            .all(|(_, set, _, clear)| set.is_ascii() && clear.is_ascii())
    );
}

#[test]
fn built_in_device_population_is_finite() {
    assert_eq!(
        DEVICE_HEADERS.map(|(_, name, _, _, _)| name.trim_end()),
        ["CON", "AUX", "PRN", "CLOCK"]
    );
}

#[test]
fn catalog_rejects_missing_system_file() {
    let error = catalog_system_io(&BTreeMap::new()).unwrap_err();

    assert!(error.to_string().contains(FILE_NAME));
}
