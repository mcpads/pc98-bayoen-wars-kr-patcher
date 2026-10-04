use super::{catalog_sampling_driver_lifetime, collect_direct_dos_outputs};
use crate::external_text::catalog::ExternalTextStorage;
use crate::external_text::program_text::ProgramTextBuilder;

#[test]
fn direct_scanner_recognizes_both_instruction_orders_and_only_dos_output() {
    let mut bytes = vec![0; 0x140];
    bytes[0x10..0x17].copy_from_slice(&[0xb4, 0x09, 0xba, 0x20, 0x01, 0xcd, 0x21]);
    bytes[0x20..0x24].copy_from_slice(b"one$");
    bytes[0x30..0x37].copy_from_slice(&[0xba, 0x50, 0x01, 0xb4, 0x09, 0xcd, 0x21]);
    bytes[0x50..0x54].copy_from_slice(b"two$");
    bytes[0x60..0x67].copy_from_slice(&[0xb4, 0x09, 0xba, 0x50, 0x01, 0xcd, 0x7f]);

    let mut builder = ProgramTextBuilder::new(
        "TEST.COM",
        ExternalTextStorage::OriginalFile,
        bytes.len(),
        &bytes,
    );
    collect_direct_dos_outputs(&bytes, 0..0x70, &mut builder).unwrap();
    let catalog = builder.finish().unwrap();

    assert_eq!(catalog.unique_text_count, 2);
    assert_eq!(catalog.reference_count, 2);
    assert_eq!(catalog.entries[1].references[0].role, "direct_dos_output");
}

#[test]
fn sampling_driver_lifetime_keeps_appended_data_outside_the_resident_image() {
    let mut bytes = vec![0_u8; 0x1570];
    bytes[..4].copy_from_slice(&[0xfa, 0xe9, 0xe8, 0x14]);
    bytes[0x14ec] = 0xfb;
    bytes[0x1562..0x1570].copy_from_slice(&[
        0xba, 0xec, 0x15, 0x83, 0xc2, 0x0f, 0xc1, 0xea, 0x04, 0xb8, 0x00, 0x31, 0xcd, 0x21,
    ]);

    let lifetime = catalog_sampling_driver_lifetime(&bytes).unwrap();

    assert_eq!(lifetime.entry_jump_offset, 1);
    assert_eq!(lifetime.transient_entry_com_address, 0x15ec);
    assert_eq!(lifetime.resident_end_com_address, 0x15ec);
    assert!(
        0x100 + bytes.len() > usize::from(lifetime.resident_end_com_address),
        "the fixture must contain a non-resident tail"
    );

    bytes[0x1562] = 0xbb;
    assert!(catalog_sampling_driver_lifetime(&bytes).is_err());
}
