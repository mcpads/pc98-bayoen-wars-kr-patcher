use super::*;

struct PlanFixture {
    verification: SourceVerification,
    source_files: SourceFiles,
    installer_payload: BTreeMap<String, Vec<u8>>,
    content: BTreeMap<String, Vec<u8>>,
}

fn fixture() -> PlanFixture {
    let boot_files = PRESERVED_BOOT_FILES
        .iter()
        .map(|name| ((*name).to_owned(), format!("boot-{name}").into_bytes()))
        .collect::<BTreeMap<_, _>>();
    let runtime_files = REQUIRED_RUNTIME_FILES
        .iter()
        .map(|name| ((*name).to_owned(), format!("runtime-{name}").into_bytes()))
        .collect::<BTreeMap<_, _>>();
    let installer_payload = payload_write_order()
        .iter()
        .map(|name| ((*name).to_owned(), format!("payload-{name}").into_bytes()))
        .collect::<BTreeMap<_, _>>();
    let source_files = SourceFiles {
        installer: Vec::new(),
        boot_files,
        runtime_files,
    };
    let mut content = standalone_source_files(&source_files, &installer_payload).unwrap();
    for name in IN_GAME_PATCH_FILES {
        content.get_mut(name).unwrap().extend_from_slice(b"-ko");
    }
    PlanFixture {
        verification: SourceVerification {
            sha256: SOURCE_DISK_SHA256.to_owned(),
            size: SOURCE_DISK_SIZE,
        },
        source_files,
        installer_payload,
        content,
    }
}

#[test]
fn plan_covers_complete_game_and_only_patches_in_game_files() {
    let fixture = fixture();
    let plan = create_plan(
        &fixture.verification,
        &fixture.source_files,
        &fixture.installer_payload,
        &fixture.content,
    )
    .unwrap();

    assert_eq!(plan.assembly.retained_files.len(), 4);
    assert_eq!(plan.assembly.placed_files.len(), 69);
    assert_eq!(
        patched_file_names(&plan)
            .into_iter()
            .collect::<BTreeSet<_>>(),
        IN_GAME_PATCH_FILES
            .map(str::to_owned)
            .into_iter()
            .collect::<BTreeSet<_>>()
    );
    let json = serde_json::to_value(plan).unwrap();
    assert_eq!(json["format"], PATCH_FORMAT);
    assert_eq!(json["id"], "bayoen-wars-pc98-ko-1.0.0");
    assert_eq!(json["title"], "바요엔 워즈 한국어 패치 1.0.0");
    assert_eq!(json["output_filename"], "bayoen-wars-ko-1.0.0.hdm");
    assert_eq!(json["source"]["mount_policy"], "pc98_dos3");
    assert!(json.get("target").is_none());
}

#[test]
fn plan_rejects_a_change_to_the_excluded_pre_game_menu() {
    let mut fixture = fixture();
    fixture
        .content
        .get_mut("MENU.COM")
        .unwrap()
        .extend_from_slice(b"-ko");

    let error = create_plan(
        &fixture.verification,
        &fixture.source_files,
        &fixture.installer_payload,
        &fixture.content,
    )
    .err()
    .expect("excluded MENU.COM change must fail")
    .to_string();

    assert!(error.contains("changed the wrong logical files"));
    assert!(error.contains("MENU.COM"));
}
