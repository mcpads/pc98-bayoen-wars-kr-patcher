use super::*;

#[test]
fn wav_header_and_linear_nibble_expansion_are_consistent() {
    let wav = encode_mono_u8(&[0, 8, 15], 2_000).unwrap();

    assert_eq!(&wav[0..4], b"RIFF");
    assert_eq!(&wav[8..12], b"WAVE");
    assert_eq!(u32::from_le_bytes(wav[24..28].try_into().unwrap()), 2_000);
    assert_eq!(u32::from_le_bytes(wav[40..44].try_into().unwrap()), 3);
    assert_eq!(&wav[44..], &[0, 136, 255]);
}
