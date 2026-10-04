use super::*;

#[test]
fn resident_extension_inserts_paragraphs_without_changing_the_tail() {
    let source = (0_u8..64).collect::<Vec<_>>();

    let output = reconstruct_resident_extension(&source, 16, 48).unwrap();

    assert_eq!(&output[..16], &source[..16]);
    assert_eq!(&output[16..48], &[0; 32]);
    assert_eq!(&output[48..], &source[16..]);
}

#[test]
fn paragraph_end_rounds_up_without_adding_a_spare_paragraph() {
    assert_eq!(paragraph_aligned_end(0x1230).unwrap(), 0x1230);
    assert_eq!(paragraph_aligned_end(0x1231).unwrap(), 0x1240);
}
