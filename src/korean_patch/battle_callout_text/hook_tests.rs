use super::*;

#[test]
fn relative_calls_use_the_address_after_the_typed_instruction() {
    assert_eq!(relative_displacement(0x8000, 0x8100), 0xfd);
    assert_eq!(relative_displacement(0x8100, 0x8000), -0x103);
}

#[test]
fn battle_wrapper_has_a_stable_typed_source_identity() {
    assert_eq!(BATTLE_WRAPPER_SOURCE_ID, "mad-battle-callout-bank-wrapper");
}
