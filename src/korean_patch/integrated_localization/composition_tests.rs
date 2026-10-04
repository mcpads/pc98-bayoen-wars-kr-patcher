use super::*;

fn files(entries: &[(&str, &[u8])]) -> BTreeMap<String, Vec<u8>> {
    entries
        .iter()
        .map(|(name, bytes)| ((*name).to_owned(), bytes.to_vec()))
        .collect()
}

fn reported_write(file_name: &str) -> PayloadWriteReport {
    PayloadWriteReport {
        id: format!("{file_name}-write"),
        file_name: file_name.to_owned(),
        owner: "fixture-producer".to_owned(),
        offset: 0,
        byte_size: 1,
        intent: "data".to_owned(),
    }
}

#[test]
fn composition_adopts_disjoint_verified_candidates_and_preserves_unowned_files() {
    let baseline = files(&[("A.COM", b"a"), ("B.COM", b"b"), ("KEEP", b"k")]);
    let first = files(&[("A.COM", b"A"), ("B.COM", b"b"), ("KEEP", b"k")]);
    let second = files(&[("A.COM", b"a"), ("B.COM", b"BB"), ("KEEP", b"k")]);
    let first_writes = [reported_write("A.COM")];
    let second_writes = [reported_write("B.COM")];

    let composed = compose_file_family(
        &baseline,
        &[
            LocalizationComponentCandidate {
                id: "first-component",
                files: &first,
                writes: &first_writes,
                modified_files: &["A.COM"],
            },
            LocalizationComponentCandidate {
                id: "second-component",
                files: &second,
                writes: &second_writes,
                modified_files: &["B.COM"],
            },
        ],
    )
    .unwrap();

    assert_eq!(composed.files["A.COM"], b"A");
    assert_eq!(composed.files["B.COM"], b"BB");
    assert_eq!(composed.files["KEEP"], b"k");
    assert_eq!(composed.writes.len(), 2);
}

#[test]
fn composition_rejects_an_unreported_candidate_change() {
    let baseline = files(&[("A.COM", b"a")]);
    let candidate = files(&[("A.COM", b"A")]);

    let error = compose_file_family(
        &baseline,
        &[LocalizationComponentCandidate {
            id: "unreported-component",
            files: &candidate,
            writes: &[],
            modified_files: &["A.COM"],
        }],
    )
    .unwrap_err();

    assert!(error.to_string().contains("reported Expected Write files"));
}

#[test]
fn composition_serializes_a_verified_shrinking_candidate() {
    let baseline = files(&[("ATLAS.DAT", b"source atlas")]);
    let candidate = files(&[("ATLAS.DAT", b"small")]);
    let writes = [reported_write("ATLAS.DAT")];

    let composed = compose_file_family(
        &baseline,
        &[LocalizationComponentCandidate {
            id: "atlas-component",
            files: &candidate,
            writes: &writes,
            modified_files: &["ATLAS.DAT"],
        }],
    )
    .unwrap();

    assert_eq!(composed.files["ATLAS.DAT"], b"small");
}

#[test]
fn composition_rejects_two_components_that_own_the_same_file() {
    let baseline = files(&[("A.COM", b"a")]);
    let first = files(&[("A.COM", b"A")]);
    let second = files(&[("A.COM", b"B")]);
    let first_writes = [reported_write("A.COM")];
    let second_writes = [reported_write("A.COM")];

    let error = compose_file_family(
        &baseline,
        &[
            LocalizationComponentCandidate {
                id: "first-component",
                files: &first,
                writes: &first_writes,
                modified_files: &["A.COM"],
            },
            LocalizationComponentCandidate {
                id: "second-component",
                files: &second,
                writes: &second_writes,
                modified_files: &["A.COM"],
            },
        ],
    )
    .unwrap_err();

    assert!(error.to_string().contains("both own A.COM"));
}
