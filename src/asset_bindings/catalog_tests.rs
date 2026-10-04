use super::*;

#[test]
fn catalog_fails_before_publishing_incomplete_domain_bindings() {
    let error = catalog_asset_bindings(
        &vec![0; 0xd300],
        &BTreeMap::new(),
        &BTreeMap::new(),
        &BTreeMap::new(),
    )
    .unwrap_err();

    assert!(!error.to_string().is_empty());
}
