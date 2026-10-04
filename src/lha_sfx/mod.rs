mod payload_manifest;

use std::collections::{BTreeMap, BTreeSet};
use std::io::{Cursor, Read};

use anyhow::{Context, Result, bail, ensure};
use delharc::LhaDecodeReader;

use payload_manifest::EXPECTED_PAYLOAD;
pub(crate) use payload_manifest::payload_write_order;

use crate::source_disk::sha256_hex;

pub(crate) fn extract_installer_payload(installer: &[u8]) -> Result<BTreeMap<String, Vec<u8>>> {
    let archive_offset = mz_executable_length(installer)?;
    let archive = installer
        .get(archive_offset..)
        .context("installer executable does not contain an LHa archive body")?;
    ensure!(
        archive
            .get(2..7)
            .is_some_and(|method| method.starts_with(b"-lh")),
        "installer archive does not start at the MZ-declared executable boundary"
    );

    let cursor = Cursor::new(archive);
    let mut reader = LhaDecodeReader::new(cursor)
        .map_err(|error| anyhow::anyhow!("failed to parse installer LHa header: {error}"))?;
    let mut files = BTreeMap::new();

    loop {
        let name = reader.header().parse_pathname_to_str().to_ascii_uppercase();
        ensure!(!name.is_empty(), "installer contains an empty file name");
        ensure!(
            !name.contains('/') && !name.contains('\\'),
            "installer contains an unsupported nested path: {name}"
        );
        ensure!(
            reader.is_decoder_supported(),
            "installer uses an unsupported LHa method for {name}: {:?}",
            reader.header().compression_method()
        );

        let mut bytes = Vec::with_capacity(reader.header().original_size as usize);
        reader
            .read_to_end(&mut bytes)
            .with_context(|| format!("failed to decode installer entry: {name}"))?;
        reader
            .crc_check()
            .map_err(|error| anyhow::anyhow!("LHa CRC check failed for {name}: {error}"))?;
        if files.insert(name.clone(), bytes).is_some() {
            bail!("installer contains a duplicate file name: {name}");
        }

        if !reader
            .seek_next_file()
            .map_err(|error| anyhow::anyhow!("failed to parse the next installer entry: {error}"))?
        {
            break;
        }
    }

    validate_payload(&files)?;
    Ok(files)
}

fn validate_payload(files: &BTreeMap<String, Vec<u8>>) -> Result<()> {
    let actual_names: BTreeSet<_> = files.keys().map(String::as_str).collect();
    let expected_names: BTreeSet<_> = EXPECTED_PAYLOAD.iter().map(|file| file.name).collect();
    ensure!(
        actual_names == expected_names,
        "installer payload file set differs from the supported profile: expected {expected_names:?}, got {actual_names:?}"
    );

    for expected in EXPECTED_PAYLOAD {
        let bytes = files
            .get(expected.name)
            .with_context(|| format!("installer payload is missing {}", expected.name))?;
        ensure!(
            bytes.len() == expected.size,
            "installer payload {} has size {}, expected {}",
            expected.name,
            bytes.len(),
            expected.size
        );
        let actual_sha256 = sha256_hex(bytes);
        ensure!(
            actual_sha256 == expected.sha256,
            "installer payload {} failed SHA-256 verification: expected {}, got {}",
            expected.name,
            expected.sha256,
            actual_sha256
        );
    }
    Ok(())
}

fn mz_executable_length(executable: &[u8]) -> Result<usize> {
    ensure!(executable.len() >= 6, "installer has a truncated MZ header");
    ensure!(
        &executable[..2] == b"MZ",
        "installer is not an MZ executable"
    );
    let last_page_bytes = u16::from_le_bytes([executable[2], executable[3]]) as usize;
    let page_count = u16::from_le_bytes([executable[4], executable[5]]) as usize;
    ensure!(page_count > 0, "installer MZ header declares zero pages");
    ensure!(
        last_page_bytes <= 512,
        "installer MZ header has an invalid last-page size"
    );
    let length = if last_page_bytes == 0 {
        page_count
            .checked_mul(512)
            .context("installer MZ length overflow")?
    } else {
        page_count
            .checked_sub(1)
            .and_then(|pages| pages.checked_mul(512))
            .and_then(|bytes| bytes.checked_add(last_page_bytes))
            .context("installer MZ length overflow")?
    };
    ensure!(
        length < executable.len(),
        "installer MZ header does not leave an appended archive"
    );
    Ok(length)
}

#[cfg(test)]
#[path = "lha_sfx_tests.rs"]
mod lha_sfx_tests;
