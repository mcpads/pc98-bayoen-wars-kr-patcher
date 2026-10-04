use std::collections::BTreeMap;

use super::*;

#[test]
fn population_reconciles_every_status_with_the_denominator() {
    let installer = files(&[("MAD.COM", b"mad"), ("TITLE.DAT", b"title")]);
    let runtime = files(&[("NMOUSE.COM", b"mouse")]);
    let boot = files(&[("IO98.SYS", b"boot")]);

    let catalog = catalog_localization_assets(&installer, &runtime, &boot).unwrap();

    assert_eq!(catalog.total_files, 4);
    assert_eq!(catalog.confirmed_localization_targets, 3);
    assert_eq!(catalog.evidence_based_exclusions, 1);
    assert_eq!(catalog.unresolved, 0);
    assert_eq!(
        catalog.confirmed_localization_targets
            + catalog.evidence_based_exclusions
            + catalog.unresolved,
        catalog.total_files
    );
}

#[test]
fn duplicate_names_across_origins_are_rejected() {
    let installer = files(&[("MAD.COM", b"installer")]);
    let runtime = files(&[("MAD.COM", b"runtime")]);

    let error = catalog_localization_assets(&installer, &runtime, &BTreeMap::new()).unwrap_err();

    assert!(
        error
            .to_string()
            .contains("population contains duplicate file name: MAD.COM")
    );
}

fn files(entries: &[(&str, &[u8])]) -> BTreeMap<String, Vec<u8>> {
    entries
        .iter()
        .map(|(name, bytes)| ((*name).to_owned(), bytes.to_vec()))
        .collect()
}
