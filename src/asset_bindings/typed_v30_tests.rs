use super::*;

#[test]
fn complete_block_uses_typed_v30_decode_and_requires_near_return() {
    let instructions = decode_complete_block(&[0x90, 0xc3], 0..2, "test block").unwrap();

    assert_eq!(
        instructions,
        vec![Instruction::Nop, Instruction::Ret { pop: 0 }]
    );
    assert!(decode_complete_block(&[0x90], 0..1, "test block").is_err());
}
