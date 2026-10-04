use super::*;

#[test]
fn catalog_requires_the_target_consumer_signatures() {
    let mut program = vec![0; STREAM_COMMAND_OFFSET + STREAM_DECODER_SIGNATURE.len()];
    program[FILE_LOAD_AND_DECODE_OFFSET
        ..FILE_LOAD_AND_DECODE_OFFSET + FILE_LOAD_AND_DECODE_SIGNATURE.len()]
        .copy_from_slice(FILE_LOAD_AND_DECODE_SIGNATURE);
    program[STREAM_COMMAND_OFFSET..STREAM_COMMAND_OFFSET + STREAM_DECODER_SIGNATURE.len()]
        .copy_from_slice(STREAM_DECODER_SIGNATURE);
    let payload = packed_fixture_payload();

    let catalog = catalog_compile_lz_assets(&program, &payload).unwrap();

    assert_eq!(catalog.consumer_linked_file_count, 58);
    assert!(catalog.files.iter().all(|file| file.stream_count == 1));
    assert!(catalog.files.iter().all(|file| file.total_output_size == 1));
}

#[test]
fn catalog_fails_closed_when_a_consumer_linked_asset_is_missing() {
    let mut program = vec![0; STREAM_COMMAND_OFFSET + STREAM_DECODER_SIGNATURE.len()];
    program[FILE_LOAD_AND_DECODE_OFFSET
        ..FILE_LOAD_AND_DECODE_OFFSET + FILE_LOAD_AND_DECODE_SIGNATURE.len()]
        .copy_from_slice(FILE_LOAD_AND_DECODE_SIGNATURE);
    program[STREAM_COMMAND_OFFSET..STREAM_COMMAND_OFFSET + STREAM_DECODER_SIGNATURE.len()]
        .copy_from_slice(STREAM_DECODER_SIGNATURE);
    let mut payload = packed_fixture_payload();
    payload.remove("TITLE.DAT");

    let error = catalog_compile_lz_assets(&program, &payload).unwrap_err();

    assert!(error.to_string().contains("missing packed asset TITLE.DAT"));
}

fn packed_fixture_payload() -> BTreeMap<String, Vec<u8>> {
    CONSUMER_LINKED_PACKED_ASSETS
        .iter()
        .map(|name| ((*name).to_owned(), vec![1, 0x5a, 0]))
        .collect()
}
