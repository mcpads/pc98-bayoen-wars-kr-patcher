use std::collections::BTreeMap;

use anyhow::{Context, Result, ensure};
use serde::Serialize;

use crate::source_disk::sha256_hex;

use super::compile_lz::decode_all_streams;

const FILE_LOAD_AND_DECODE_OFFSET: usize = 0x2ed0;
const STREAM_DECODER_OFFSET: usize = 0xd235;
const STREAM_COMMAND_OFFSET: usize = 0xd267;

const STREAM_DECODER_SIGNATURE: &[u8] = &[
    0x8a, 0x07, // mov al,[bx]
    0x22, 0xc0, // and al,al
    0x75, 0x15, // jne command
    0xe8, 0x79, 0x00, // advance packed input
    0x8c, 0xd8, 0x8e, 0xc0, 0x1f, 0x5f, 0x5e, 0x2e, 0x89, 0x1e, 0xd9, 0xd4, 0x2e, 0x8c, 0x06, 0xdb,
    0xd4, 0xc3, 0x8a, 0xe0, // mov ah,al
    0x24, 0x7f, // and al,0x7f
    0xf6, 0xc4, 0x80, // test ah,0x80
];

const FILE_LOAD_AND_DECODE_SIGNATURE: &[u8] = &[
    0x8c, 0xc8, // mov ax,cs
    0x8e, 0xd8, // mov ds,ax
    0x2e, 0x89, 0x16, 0xc7, 0xd5, // preserve filename pointer
    0x53, // push bx
    0x2e, 0x8b, 0x1e, 0x8d, 0xd5, // packed input segment
];

pub(super) const CONSUMER_LINKED_PACKED_ASSETS: [&str; 58] = [
    "B04",
    "B05",
    "BG__.DAT",
    "BO.DAT",
    "BW.DAT",
    "BWM1.DAT",
    "BWM2.DAT",
    "BWM3.DAT",
    "BWM4.DAT",
    "BWM5.DAT",
    "BWM6.DAT",
    "BWM7.DAT",
    "BWM8.DAT",
    "BWM9.DAT",
    "BWM10.DAT",
    "C00",
    "C01",
    "C02",
    "C03",
    "C04",
    "C05",
    "C06",
    "C07",
    "C08",
    "C09",
    "C10",
    "C11",
    "C12",
    "C13",
    "C15",
    "C16",
    "C17",
    "C18",
    "C19",
    "C20",
    "C21",
    "CAR.DAT",
    "DATE_.DAT",
    "DEFEAT.DAT",
    "ED1.DAT",
    "ED2.DAT",
    "EDM.DAT",
    "KAO",
    "MOUSE.DAT",
    "OP1.DAT",
    "OP2.DAT",
    "OP3.DAT",
    "OPM.DAT",
    "SAMPA",
    "SEL1.DAT",
    "SEL2.DAT",
    "SEL3.DAT",
    "ST",
    "TITLE.DAT",
    "TITLE2.DAT",
    "UN.DAT",
    "WAKU_IMG.DAT",
    "WAKU_P.DAT",
];

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct CompileLzConsumer {
    pub file_load_and_decode_offset: usize,
    pub stream_decoder_offset: usize,
    pub stream_command_offset: usize,
    pub terminator: u8,
    pub literal_command_range: String,
    pub back_reference_command_range: String,
    pub back_reference_size: String,
    pub back_reference_distance: String,
    pub before_output_start: String,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct PackedStreamSummary {
    pub packed_offset: usize,
    pub packed_size: usize,
    pub output_size: usize,
    pub command_count: usize,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct PackedAssetFile {
    pub name: String,
    pub sha256: String,
    pub packed_size: usize,
    pub stream_count: usize,
    pub total_output_size: usize,
    pub streams: Vec<PackedStreamSummary>,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct CompileLzCatalog {
    pub consumer: CompileLzConsumer,
    pub consumer_linked_file_count: usize,
    pub files: Vec<PackedAssetFile>,
}

pub(crate) fn catalog_compile_lz_assets(
    mad_com: &[u8],
    installer_payload: &BTreeMap<String, Vec<u8>>,
) -> Result<CompileLzCatalog> {
    require_signature(
        mad_com,
        FILE_LOAD_AND_DECODE_OFFSET,
        FILE_LOAD_AND_DECODE_SIGNATURE,
        "file load and decode routine",
    )?;
    require_signature(
        mad_com,
        STREAM_COMMAND_OFFSET,
        STREAM_DECODER_SIGNATURE,
        "Compile LZ stream decoder",
    )?;

    let mut files = Vec::with_capacity(CONSUMER_LINKED_PACKED_ASSETS.len());
    for name in CONSUMER_LINKED_PACKED_ASSETS {
        let bytes = installer_payload.get(name).with_context(|| {
            format!("verified installer payload is missing packed asset {name}")
        })?;
        let streams = decode_all_streams(bytes)?;
        let stream_summaries: Vec<_> = streams
            .into_iter()
            .map(|stream| PackedStreamSummary {
                packed_offset: stream.packed_offset,
                packed_size: stream.packed_size,
                output_size: stream.output.len(),
                command_count: stream.command_count,
            })
            .collect();
        let total_output_size = stream_summaries
            .iter()
            .map(|stream| stream.output_size)
            .sum();
        files.push(PackedAssetFile {
            name: name.to_owned(),
            sha256: sha256_hex(bytes),
            packed_size: bytes.len(),
            stream_count: stream_summaries.len(),
            total_output_size,
            streams: stream_summaries,
        });
    }

    Ok(CompileLzCatalog {
        consumer: CompileLzConsumer {
            file_load_and_decode_offset: FILE_LOAD_AND_DECODE_OFFSET,
            stream_decoder_offset: STREAM_DECODER_OFFSET,
            stream_command_offset: STREAM_COMMAND_OFFSET,
            terminator: 0,
            literal_command_range: "0x01..=0x7F".to_owned(),
            back_reference_command_range: "0x80..=0xFF".to_owned(),
            back_reference_size: "(command & 0x7F) + 3".to_owned(),
            back_reference_distance: "distance_byte + 1".to_owned(),
            before_output_start: "zero_fill".to_owned(),
        },
        consumer_linked_file_count: files.len(),
        files,
    })
}

fn require_signature(program: &[u8], offset: usize, signature: &[u8], role: &str) -> Result<()> {
    let actual = program.get(offset..offset + signature.len());
    ensure!(
        actual == Some(signature),
        "MAD.COM {role} signature does not match at file offset 0x{offset:X}"
    );
    Ok(())
}

#[cfg(test)]
#[path = "packed_assets_tests.rs"]
mod packed_assets_tests;
