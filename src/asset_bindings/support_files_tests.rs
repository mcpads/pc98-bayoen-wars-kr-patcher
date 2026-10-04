use super::*;

#[test]
fn support_catalog_requires_complete_file_contents() {
    let mut installer = BTreeMap::from([
        ("DMADOU.BAT".to_owned(), DMADOU_BAT.to_vec()),
        ("DATA.DAT".to_owned(), DATA_DAT.to_vec()),
    ]);
    let boot = BTreeMap::from([("CONFIG.SYS".to_owned(), CONFIG_SYS.to_vec())]);

    assert_eq!(
        catalog_support_files(&installer, &boot).unwrap().file_count,
        3
    );
    installer.get_mut("DMADOU.BAT").unwrap()[0] ^= 1;
    assert!(catalog_support_files(&installer, &boot).is_err());
}
