use super::*;

fn blank_font_bmp() -> Vec<u8> {
    let mut bmp = vec![0xff; BMP_DATA_OFFSET + BMP_DATA_SIZE];
    let byte_size = u32::try_from(bmp.len()).unwrap();
    bmp[..2].copy_from_slice(b"BM");
    bmp[2..6].copy_from_slice(&byte_size.to_le_bytes());
    bmp[10..14].copy_from_slice(&(BMP_DATA_OFFSET as u32).to_le_bytes());
    bmp[14..18].copy_from_slice(&40u32.to_le_bytes());
    bmp[18..22].copy_from_slice(&(BMP_WIDTH as u32).to_le_bytes());
    bmp[22..26].copy_from_slice(&(BMP_HEIGHT as u32).to_le_bytes());
    bmp[26..28].copy_from_slice(&1u16.to_le_bytes());
    bmp[28..30].copy_from_slice(&1u16.to_le_bytes());
    bmp[30..34].copy_from_slice(&0u32.to_le_bytes());
    bmp[34..38].copy_from_slice(&(BMP_DATA_SIZE as u32).to_le_bytes());
    bmp
}

#[test]
fn shift_jis_cells_map_to_the_pc98_bmp_grid() {
    assert_eq!(shift_jis_to_jis(0x8d, 0x55).unwrap(), (0x39, 0x36));
    assert_eq!(shift_jis_to_jis(0x82, 0xa0).unwrap(), (0x24, 0x22));
}

#[test]
fn bmp_rows_are_read_bottom_up_and_inverted() {
    let mut bmp = blank_font_bmp();
    let source_x = 0x39 * 16;
    let source_y = 0x36 * 16;
    let pixel_x = source_x + 3;
    let pixel_y = source_y + 4;
    let bmp_row = BMP_HEIGHT - 1 - pixel_y;
    let offset = BMP_DATA_OFFSET + bmp_row * BMP_ROW_BYTES + pixel_x / 8;
    bmp[offset] &= !(0x80 >> (pixel_x % 8));

    let font = Pc98Font::parse(bmp).unwrap();
    let glyph = font.rasterize('攻').unwrap();

    assert_eq!(glyph.width, 16);
    assert_eq!(glyph.bitmap[4 * 2] & 0x10, 0x10);
}

#[test]
fn malformed_bmp_layout_is_rejected() {
    let mut bmp = blank_font_bmp();
    bmp[28..30].copy_from_slice(&8u16.to_le_bytes());

    let error = Pc98Font::parse(bmp).err().unwrap();
    assert!(error.to_string().contains("layout differs"));
}
