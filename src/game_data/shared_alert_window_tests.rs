use super::*;

#[test]
fn every_consumer_of_the_shared_frame_moves_with_the_frame() {
    let sites = shared_alert_sites();

    assert_eq!(
        sites
            .iter()
            .map(|site| site.state_value)
            .collect::<Vec<_>>(),
        [0x0e, 0x10, 0x11, 0x1f, 0x16]
    );
    assert!(sites.iter().all(|site| {
        site.text_position_instruction_file_offset + 3 == site.text_address_instruction_file_offset
            && site.text_address_instruction_file_offset + 5 == site.text_renderer_call_file_offset
    }));
}
