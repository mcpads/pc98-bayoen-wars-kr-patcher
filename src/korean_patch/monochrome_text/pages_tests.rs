use super::*;

#[test]
fn page_encoder_uses_atlas_indices_and_control_bytes() {
    let glyphs = BTreeMap::from([('가', 0), (' ', 1), ('나', 2)]);

    let (encoded, lines) = encode_page(&["가 나".to_owned(), "나".to_owned()], &glyphs).unwrap();

    assert_eq!(encoded, [0, 1, 2, LINE_BREAK, 2, PAGE_END]);
    assert_eq!(lines, [vec![0, 1, 2], vec![2]]);
}

#[test]
fn page_encoder_rejects_unmapped_and_oversized_text() {
    let glyphs = BTreeMap::from([('가', 0)]);

    assert!(
        encode_page(&["나".to_owned()], &glyphs)
            .unwrap_err()
            .to_string()
            .contains("no atlas slot")
    );
    assert!(
        encode_page(&["가".repeat(17)], &glyphs)
            .unwrap_err()
            .to_string()
            .contains("1..=16 glyphs")
    );
    assert!(
        encode_page(&vec!["가".to_owned(); 5], &glyphs)
            .unwrap_err()
            .to_string()
            .contains("1..=4 lines")
    );
}
