use expected_write::{ExpectedWrite, ImageRegion, MachineCodeProvenance, RegionKind, ResizePlan};

use super::*;

fn data_plan(expected_original: Vec<u8>) -> PayloadFileWritePlan {
    PayloadFileWritePlan {
        file_name: "MAD.COM",
        plan: WritePlan::new()
            .region(ImageRegion {
                id: "dialogue-byte-range".to_owned(),
                range: 1..3,
                kind: RegionKind::Data,
                reason: "fixture dialogue code".to_owned(),
            })
            .write(ExpectedWrite {
                id: "dialogue-character-code".to_owned(),
                owner: "dialogue-encoder".to_owned(),
                purpose: "replace one fixture character".to_owned(),
                offset: 1,
                expected_original,
                replacement: vec![0xec, 0x4a],
                intent: WriteIntent::Data,
            }),
    }
}

#[test]
fn exact_data_write_is_applied_without_changing_other_files() {
    let payload = BTreeMap::from([
        ("MAD.COM".to_owned(), vec![0, 0x82, 0xf1, 0]),
        ("MENU.COM".to_owned(), vec![7, 8]),
    ]);

    let applied = apply_payload_write_plans(&payload, vec![data_plan(vec![0x82, 0xf1])]).unwrap();

    assert_eq!(applied.files["MAD.COM"], [0, 0xec, 0x4a, 0]);
    assert_eq!(applied.files["MENU.COM"], payload["MENU.COM"]);
    assert_eq!(applied.report.len(), 1);
    assert_eq!(applied.report[0].intent, "data");
}

#[test]
fn expected_original_mismatch_is_rejected() {
    let payload = BTreeMap::from([("MAD.COM".to_owned(), vec![0, 0x82, 0xf1, 0])]);

    let error = apply_payload_write_plans(&payload, vec![data_plan(vec![0, 0])]).unwrap_err();

    assert!(error.to_string().contains("apply Expected Writes"));
}

#[test]
fn two_plans_cannot_own_the_same_file() {
    let payload = BTreeMap::from([("MAD.COM".to_owned(), vec![0, 0x82, 0xf1, 0])]);

    let error = apply_payload_write_plans(
        &payload,
        vec![data_plan(vec![0x82, 0xf1]), data_plan(vec![0x82, 0xf1])],
    )
    .unwrap_err();

    assert!(error.to_string().contains("multiple Expected Write plans"));
}

#[test]
fn explicitly_combined_disjoint_writes_share_one_immutable_baseline() {
    let mut files = BTreeMap::new();
    files.insert("MAD.COM".to_owned(), vec![0, 0x82, 0xf1, 4]);
    let first = data_plan(vec![0x82, 0xf1]);
    let second = PayloadFileWritePlan {
        file_name: "MAD.COM",
        plan: WritePlan::new()
            .region(ImageRegion {
                id: "second-region".to_owned(),
                range: 3..4,
                kind: RegionKind::Data,
                reason: "second observable byte".to_owned(),
            })
            .write(ExpectedWrite {
                id: "second-write".to_owned(),
                owner: "second-producer".to_owned(),
                purpose: "change a disjoint byte".to_owned(),
                offset: 3,
                expected_original: vec![4],
                replacement: vec![8],
                intent: WriteIntent::Data,
            }),
    };

    let combined = combine_payload_write_plans(vec![first, second]).unwrap();
    let applied = apply_payload_write_plans(&files, combined).unwrap();

    assert_eq!(applied.files["MAD.COM"], [0, 0xec, 0x4a, 8]);
    assert_eq!(applied.report.len(), 2);
}

#[test]
fn explicitly_combined_overlapping_writes_are_rejected() {
    let mut files = BTreeMap::new();
    files.insert("MAD.COM".to_owned(), vec![0, 0x82, 0xf1, 0]);
    let first = data_plan(vec![0x82, 0xf1]);
    let second = data_plan(vec![0x82, 0xf1]);

    let combined = combine_payload_write_plans(vec![first, second]).unwrap();
    let error = apply_payload_write_plans(&files, combined).unwrap_err();

    assert!(
        error
            .to_string()
            .contains("apply Expected Writes to MAD.COM")
    );
}

#[test]
fn explicitly_combined_duplicate_resizes_are_rejected() {
    let resize_plan = |owner: &str| PayloadFileWritePlan {
        file_name: "GAIJI.COM",
        plan: WritePlan::new().resize(ResizePlan {
            owner: owner.to_owned(),
            purpose: "fixture file growth".to_owned(),
            expected_input_len: 4,
            output_len: 8,
        }),
    };

    let error = combine_payload_write_plans(vec![
        resize_plan("first-producer"),
        resize_plan("second-producer"),
    ])
    .err()
    .expect("two resize owners must be rejected");

    assert!(
        error
            .to_string()
            .contains("multiple Expected Write resize plans")
    );
}

#[test]
fn executable_write_without_a_typed_isa_verifier_is_rejected() {
    let payload = BTreeMap::from([("MAD.COM".to_owned(), vec![0x90])]);
    let file_plan = PayloadFileWritePlan {
        file_name: "MAD.COM",
        plan: WritePlan::new()
            .region(ImageRegion {
                id: "fixture-instruction".to_owned(),
                range: 0..1,
                kind: RegionKind::MachineCode,
                reason: "fixture executable byte".to_owned(),
            })
            .write(ExpectedWrite {
                id: "fixture-executable-write".to_owned(),
                owner: "fixture-typed-assembler".to_owned(),
                purpose: "prove that raw executable bytes cannot enter the patch path".to_owned(),
                offset: 0,
                expected_original: vec![0x90],
                replacement: vec![0xc3],
                intent: WriteIntent::MachineCode(MachineCodeProvenance {
                    assembly_source_id: "fixture-source".to_owned(),
                    isa_profile_id: "v30".to_owned(),
                }),
            }),
    };

    let error = apply_payload_write_plans(&payload, vec![file_plan]).unwrap_err();

    assert!(error.chain().any(|cause| {
        cause
            .to_string()
            .contains("machine code without a verifier")
    }));
}
