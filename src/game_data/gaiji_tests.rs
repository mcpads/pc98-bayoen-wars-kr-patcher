use super::*;

fn gaiji_fixture(first_code: u16, row_end: u16, next_row_previous: u16) -> Vec<u8> {
    let mut bytes = vec![0_u8; 0xd0];
    bytes[0x12..0x51].copy_from_slice(&[
        0xb9,
        0x03,
        0x00,
        0x2e,
        0xc7,
        0x06,
        0x78,
        0x01,
        first_code as u8,
        (first_code >> 8) as u8,
        0xbb,
        0x80,
        0x01,
        0x2e,
        0xa1,
        0x76,
        0x01,
        0xd1,
        0xe0,
        0x53,
        0x03,
        0xd8,
        0x51,
        0x2e,
        0x8b,
        0x0f,
        0x2e,
        0xff,
        0x06,
        0x76,
        0x01,
        0x2e,
        0x8b,
        0x16,
        0x78,
        0x01,
        0x81,
        0xfa,
        row_end as u8,
        (row_end >> 8) as u8,
        0x75,
        0x07,
        0x2e,
        0xc7,
        0x06,
        0x78,
        0x01,
        next_row_previous as u8,
        (next_row_previous >> 8) as u8,
        0x2e,
        0xff,
        0x06,
        0x78,
        0x01,
        0x53,
        0xe8,
        0x23,
        0x00,
        0x5b,
        0x59,
        0x5b,
        0xe2,
        0xce,
    ]);
    bytes[0x6f..0x76].copy_from_slice(&[0x8c, 0xcb, 0xb4, 0x1a, 0xcd, 0x18, 0xc3]);
    bytes[0x80..0x86].copy_from_slice(&[0x90, 0x01, 0xa0, 0x01, 0xb0, 0x01]);
    bytes
}

#[test]
fn current_code_is_used_before_advancing_to_the_next_row() {
    let bytes = gaiji_fixture(0x767d, 0x767e, 0x7720);

    let catalog = parse_gaiji_program(&bytes).unwrap();

    let codes: Vec<_> = catalog
        .glyphs
        .iter()
        .map(|glyph| (glyph.character_code, glyph.shift_jis_code))
        .collect();
    assert_eq!(
        codes,
        vec![(0x767d, 0xebfb), (0x767e, 0xebfc), (0x7721, 0xec40)]
    );
}

#[test]
fn non_increasing_record_pointers_are_rejected() {
    let mut bytes = gaiji_fixture(0x7621, 0x767e, 0x7720);
    bytes[0x82..0x84].copy_from_slice(&0x190_u16.to_le_bytes());

    let error = parse_gaiji_program(&bytes).unwrap_err();

    assert!(error.to_string().contains("not strictly increasing"));
}
