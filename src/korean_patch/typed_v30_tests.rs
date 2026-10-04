use super::*;

#[test]
fn relocated_immediate_preserves_the_admitted_register_and_width() {
    let assembled = assemble_relocated_mov_immediate(
        &[0xba, 0x34, 0x12],
        0,
        0x5678,
        &[Register16::DX],
        "fixture",
    )
    .unwrap();

    assert_eq!(assembled.bytes(), [0xba, 0x78, 0x56]);
    assert_eq!(assembled.origin(), CodeLocation { seg: 0, off: 0x100 });
}

#[test]
fn relocated_immediate_rejects_a_different_register_or_instruction() {
    assert!(
        assemble_relocated_mov_immediate(
            &[0xbb, 0x34, 0x12],
            0,
            0x5678,
            &[Register16::DX],
            "fixture",
        )
        .is_err()
    );
    assert!(
        assemble_relocated_mov_immediate(
            &[0xba, 0x34, 0x12],
            0,
            0x5678,
            &[Register16::BX],
            "fixture",
        )
        .is_err()
    );
}

#[test]
fn relocated_segment_immediate_uses_the_file_offset_as_its_origin() {
    let assembled = assemble_relocated_segment_mov_immediate(
        &[0xba, 0x34, 0x12],
        0,
        0x5678,
        &[Register16::DX],
        "fixture",
    )
    .unwrap();

    assert_eq!(assembled.bytes(), [0xba, 0x78, 0x56]);
    assert_eq!(assembled.origin(), CodeLocation { seg: 0, off: 0 });
}

#[test]
fn relocated_direct_ax_store_preserves_the_accumulator_moffs_form() {
    let assembled =
        assemble_relocated_direct_ax_store(&[0xa3, 0x34, 0x12], 0, 0x5678, "fixture").unwrap();

    assert_eq!(assembled.bytes(), [0xa3, 0x78, 0x56]);
    assert_eq!(assembled.origin(), CodeLocation { seg: 0, off: 0x100 });
}

#[test]
fn relocated_direct_ax_store_rejects_a_non_accumulator_source() {
    assert!(
        assemble_relocated_direct_ax_store(&[0x89, 0x1e, 0x34, 0x12], 0, 0x5678, "fixture")
            .is_err()
    );
}
