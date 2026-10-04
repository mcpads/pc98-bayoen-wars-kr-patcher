use super::catalog_system_loader;

#[test]
fn system_message_consumers_must_match_before_text_is_cataloged() {
    assert!(catalog_system_loader(&vec![0; 0x1000]).is_err());
}
