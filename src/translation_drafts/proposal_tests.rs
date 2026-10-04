use super::is_control_only_source_text;

#[test]
fn control_only_text_ignores_terminal_sequences_and_spacing() {
    assert!(is_control_only_source_text("\u{1b}[37m \r\n"));
    assert!(!is_control_only_source_text("\u{1b}[37m visible"));
    assert!(!is_control_only_source_text("\u{1b}[37m 한국어"));
}
