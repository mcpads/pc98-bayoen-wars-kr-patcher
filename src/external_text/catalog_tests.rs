use std::collections::BTreeMap;

use super::catalog_external_text;

#[test]
fn missing_required_program_fails_before_partial_catalog_publication() {
    assert!(catalog_external_text(&BTreeMap::new(), &BTreeMap::new(), &BTreeMap::new()).is_err());
}
