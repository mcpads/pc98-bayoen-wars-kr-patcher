use super::*;

#[test]
fn page_parser_preserves_glyph_lines_and_controls() {
    let lines = parse_page(&[0, 2, LINE_BREAK, 1, PAGE_END], 3).unwrap();

    assert_eq!(lines, vec![vec![0, 2], vec![1]]);
}

#[test]
fn page_parser_rejects_invalid_boundaries_and_glyphs() {
    assert!(parse_page(&[0, PAGE_END, 1, PAGE_END], 2).is_err());
    assert!(parse_page(&[0, LINE_BREAK, PAGE_END], 2).is_err());
    assert!(parse_page(&[2, PAGE_END], 2).is_err());
}
