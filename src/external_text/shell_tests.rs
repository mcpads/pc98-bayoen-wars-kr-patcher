use super::*;

#[test]
fn shell_catalog_rejects_unrelated_programs() {
    let error = catalog_shell(&vec![0; 0x500]).unwrap_err();

    assert!(
        error
            .to_string()
            .contains("interrupt-error output consumer")
    );
}
