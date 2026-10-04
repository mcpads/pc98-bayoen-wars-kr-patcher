use super::*;

fn lines(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_owned()).collect()
}

#[test]
fn manually_reflowed_dialogue_fits_the_verified_rom_region() {
    validate_dialogue_entry_layout(
        "dialogue-g10-r02",
        &lines(&["응, 뭐,", "후쿠진즈케를", "찾으러…"]),
    )
    .unwrap();
    validate_dialogue_entry_layout(
        "dialogue-g10-r03",
        &lines(&["그렇게까지 해서", "사탄님께 다가가고", "싶은 거야!?"]),
    )
    .unwrap();
}

#[test]
fn dialogue_rom_layout_rejects_encoded_width_and_line_count_overflow() {
    let width_error = validate_dialogue_entry_layout(
        "dialogue-g10-r02",
        &lines(&["응, 뭐,", "후쿠진즈케를 찾으러…"]),
    )
    .unwrap_err()
    .to_string();
    assert!(width_error.contains("352px"));
    assert!(width_error.contains("288px"));

    let line_error = validate_dialogue_entry_layout(
        "dialogue-g10-r02",
        &lines(&["응,", "뭐,", "찾으러", "왔어…"]),
    )
    .unwrap_err()
    .to_string();
    assert!(line_error.contains("4 lines"));
    assert!(line_error.contains("holds 3"));
}

#[test]
fn dialogue_punctuation_width_matches_its_full_cell_gaiji_encoding() {
    assert_eq!(dialogue_line_pixel_width("저기~…").unwrap(), 128);
}
