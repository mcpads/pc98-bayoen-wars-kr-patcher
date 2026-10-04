use std::collections::BTreeMap;

use anyhow::{Context, Result, ensure};
use serde::Serialize;

use crate::dos_program::unpack_self_expanding_com;

const COM_ORIGIN: usize = 0x100;
const DRIVER_NAME: &str = "FPLAY6.COM";
const SONG_NAME: &str = "SONG.DAT";
const SONG_SIZE: usize = 14_800;
const SONG_SIGNATURE_OFFSET: usize = 0x0d;
const SONG_SIGNATURE: &[u8] = b"MAIKO-HOSHINO ";
const DRIVER_LOAD_CALL_OFFSET: usize = 0x2846;
const DRIVER_LOAD_SERVICE: u8 = 0x09;
const DRIVER_LOAD_INTERRUPT: u8 = 0x7f;

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct SongSequenceBinding {
    pub name: String,
    pub byte_size: usize,
    pub signature_offset: usize,
    pub signature: String,
    pub driver_name: String,
    pub driver_file_name_offset: usize,
    pub driver_runtime_address: usize,
    pub driver_reference_offsets: Vec<usize>,
    pub load_service_function: u8,
    pub load_service_interrupt: u8,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct SoundDataCatalog {
    pub song_sequence: SongSequenceBinding,
}

pub(super) fn catalog_sound_data(
    installer_payload: &BTreeMap<String, Vec<u8>>,
) -> Result<SoundDataCatalog> {
    let song = required(installer_payload, SONG_NAME)?;
    ensure!(
        song.len() == SONG_SIZE,
        "{SONG_NAME} size does not match the consumer-bound sequence file"
    );
    ensure!(
        song.get(SONG_SIGNATURE_OFFSET..SONG_SIGNATURE_OFFSET + SONG_SIGNATURE.len())
            == Some(SONG_SIGNATURE),
        "{SONG_NAME} composer signature does not match"
    );

    let packed_driver = required(installer_payload, DRIVER_NAME)?;
    let driver = unpack_self_expanding_com(packed_driver)?;
    let file_name_offsets = find_all(&driver.unpacked, SONG_NAME.as_bytes());
    ensure!(
        file_name_offsets.len() == 1,
        "unpacked {DRIVER_NAME} must contain exactly one {SONG_NAME} name"
    );
    let driver_file_name_offset = file_name_offsets[0];
    let driver_runtime_address = driver_file_name_offset
        .checked_add(COM_ORIGIN)
        .context("sound-driver file-name runtime address overflow")?;
    let runtime_address = u16::try_from(driver_runtime_address)
        .context("sound-driver file-name runtime address exceeds a COM segment")?;
    let [address_low, address_high] = runtime_address.to_le_bytes();
    let driver_reference_offsets = find_all(&driver.unpacked, &[0xba, address_low, address_high]);
    ensure!(
        !driver_reference_offsets.is_empty(),
        "unpacked {DRIVER_NAME} has no direct MOV DX reference to {SONG_NAME}"
    );
    ensure!(
        driver_reference_offsets == [DRIVER_LOAD_CALL_OFFSET + 2],
        "unpacked {DRIVER_NAME} does not use the expected single {SONG_NAME} loader call"
    );
    let load_call = [
        0xb4,
        DRIVER_LOAD_SERVICE,
        0xba,
        address_low,
        address_high,
        0xcd,
        DRIVER_LOAD_INTERRUPT,
        0x22,
        0xc0,
        0x75,
        0x07,
    ];
    ensure!(
        driver
            .unpacked
            .get(DRIVER_LOAD_CALL_OFFSET..DRIVER_LOAD_CALL_OFFSET + load_call.len())
            == Some(load_call.as_slice()),
        "unpacked {DRIVER_NAME} {SONG_NAME} loader signature does not match"
    );
    ensure!(
        driver
            .unpacked
            .get(driver_file_name_offset + SONG_NAME.len())
            == Some(&0),
        "unpacked {DRIVER_NAME} {SONG_NAME} is not null-terminated"
    );

    Ok(SoundDataCatalog {
        song_sequence: SongSequenceBinding {
            name: SONG_NAME.to_owned(),
            byte_size: song.len(),
            signature_offset: SONG_SIGNATURE_OFFSET,
            signature: String::from_utf8(SONG_SIGNATURE.to_vec())
                .expect("the declared song signature is ASCII"),
            driver_name: DRIVER_NAME.to_owned(),
            driver_file_name_offset,
            driver_runtime_address,
            driver_reference_offsets,
            load_service_function: DRIVER_LOAD_SERVICE,
            load_service_interrupt: DRIVER_LOAD_INTERRUPT,
        },
    })
}

fn required<'a>(files: &'a BTreeMap<String, Vec<u8>>, name: &str) -> Result<&'a [u8]> {
    files
        .get(name)
        .map(Vec::as_slice)
        .with_context(|| format!("sound data population is missing {name}"))
}

fn find_all(haystack: &[u8], needle: &[u8]) -> Vec<usize> {
    if needle.is_empty() || needle.len() > haystack.len() {
        return Vec::new();
    }
    haystack
        .windows(needle.len())
        .enumerate()
        .filter_map(|(offset, candidate)| (candidate == needle).then_some(offset))
        .collect()
}

#[cfg(test)]
#[path = "sound_data_tests.rs"]
mod sound_data_tests;
