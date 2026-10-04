use std::collections::BTreeMap;

use anyhow::{Context, Result, ensure};
use serde::Serialize;

use crate::localization_assets::decode_all_streams;

const SAMPLE_ASSET: &str = "SAMPA";
const PLAYBACK_DRIVER: &str = "BSAMP.COM";
const PLAYBACK_DRIVER_SIZE: usize = 5_921;
const STREAM_COUNT: usize = 22;
const TIMER_SETUP_FILE_OFFSET: usize = 0x0442;
const INTERRUPT_HANDLER_FILE_OFFSET: usize = 0x02cf;
const NIBBLE_SELECT_FILE_OFFSET: usize = 0x034f;
const INTERRUPT_TICKS_PER_SAMPLE: u32 = 2;
pub(crate) const SAMPLE_RATE_HZ: u32 = 9_600;

const CLOCK_PROFILES: [(u32, u16); 2] = [(2_457_600, 128), (1_996_800, 104)];

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct SampleClockProfile {
    pub timer_clock_hz: u32,
    pub timer_divisor: u16,
    pub interrupt_rate_hz: u32,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct SamplePlaybackCatalog {
    pub source_asset: String,
    pub playback_driver: String,
    pub stream_count: usize,
    pub total_decoded_bytes: usize,
    pub timer_setup_file_offset: usize,
    pub interrupt_handler_file_offset: usize,
    pub nibble_select_file_offset: usize,
    pub clock_profiles: Vec<SampleClockProfile>,
    pub interrupt_ticks_per_sample: u32,
    pub sample_rate_hz: u32,
    pub sample_format: String,
}

pub(super) fn catalog_sample_playback(
    installer_payload: &BTreeMap<String, Vec<u8>>,
    runtime_files: &BTreeMap<String, Vec<u8>>,
) -> Result<SamplePlaybackCatalog> {
    let packed = required(installer_payload, SAMPLE_ASSET, "installer payload")?;
    let streams = decode_all_streams(packed)?;
    ensure!(
        streams.len() == STREAM_COUNT,
        "{SAMPLE_ASSET} does not contain {STREAM_COUNT} packed sample streams"
    );

    let driver = required(runtime_files, PLAYBACK_DRIVER, "runtime file population")?;
    ensure!(
        driver.len() == PLAYBACK_DRIVER_SIZE,
        "{PLAYBACK_DRIVER} does not match the complete supported driver size"
    );
    ensure_bytes(
        driver,
        TIMER_SETUP_FILE_OFFSET,
        &[
            0xb0, 0x34, 0xe6, 0x77, 0xbb, 0x74, 0x13, 0x89, 0x1e, 0xd8, 0x15, 0xb8, 0x00, 0x00,
            0x8e, 0xc0, 0xb0, 0x80, 0x26, 0xf6, 0x06, 0x01, 0x05, 0x80, 0x74, 0x09, 0xb0, 0x68,
            0xbb, 0x84, 0x13, 0x89, 0x1e, 0xd8, 0x15, 0xe6, 0x71, 0xb0, 0x00, 0xe6, 0x71,
        ],
        "dual-clock timer setup",
    )?;
    ensure_bytes(
        driver,
        INTERRUPT_HANDLER_FILE_OFFSET,
        &[
            0x2e, 0x80, 0x3e, 0xe2, 0x15, 0x00, 0x74, 0x01, 0xcf, 0x2e, 0xc6, 0x06, 0xe2, 0x15,
            0x01,
        ],
        "sample interrupt handler",
    )?;
    ensure_bytes(
        driver,
        NIBBLE_SELECT_FILE_OFFSET,
        &[
            0xc4, 0x1e, 0xb0, 0x15, 0x26, 0x8a, 0x1f, 0xa8, 0x02, 0x75, 0x35, 0xc0, 0xeb, 0x04,
            0x2a, 0xff,
        ],
        "high-then-low nibble selection",
    )?;
    ensure_bytes(
        driver,
        0x02ea,
        &[
            0x2e, 0xa0, 0xd6, 0x15, 0xa8, 0x01, 0x74, 0x52, 0x2e, 0xa0, 0xd4, 0x15, 0xee, 0x2e,
            0xa0, 0xd6, 0x15, 0xfe, 0xc0, 0x24, 0x03,
        ],
        "two-interrupt sample hold",
    )?;

    let clock_profiles = CLOCK_PROFILES
        .into_iter()
        .map(|(timer_clock_hz, timer_divisor)| {
            let interrupt_rate_hz = timer_clock_hz / u32::from(timer_divisor);
            ensure!(
                timer_clock_hz.is_multiple_of(u32::from(timer_divisor)),
                "{PLAYBACK_DRIVER} timer clock does not divide exactly"
            );
            ensure!(
                interrupt_rate_hz / INTERRUPT_TICKS_PER_SAMPLE == SAMPLE_RATE_HZ,
                "{PLAYBACK_DRIVER} timer profile does not produce {SAMPLE_RATE_HZ} Hz samples"
            );
            Ok(SampleClockProfile {
                timer_clock_hz,
                timer_divisor,
                interrupt_rate_hz,
            })
        })
        .collect::<Result<Vec<_>>>()?;

    Ok(SamplePlaybackCatalog {
        source_asset: SAMPLE_ASSET.to_owned(),
        playback_driver: PLAYBACK_DRIVER.to_owned(),
        stream_count: streams.len(),
        total_decoded_bytes: streams.iter().map(|stream| stream.output.len()).sum(),
        timer_setup_file_offset: TIMER_SETUP_FILE_OFFSET,
        interrupt_handler_file_offset: INTERRUPT_HANDLER_FILE_OFFSET,
        nibble_select_file_offset: NIBBLE_SELECT_FILE_OFFSET,
        clock_profiles,
        interrupt_ticks_per_sample: INTERRUPT_TICKS_PER_SAMPLE,
        sample_rate_hz: SAMPLE_RATE_HZ,
        sample_format: "unsigned_4_bit_pcm_high_nibble_first".to_owned(),
    })
}

fn required<'a>(
    files: &'a BTreeMap<String, Vec<u8>>,
    name: &str,
    population: &str,
) -> Result<&'a [u8]> {
    files
        .get(name)
        .map(Vec::as_slice)
        .with_context(|| format!("{population} is missing {name}"))
}

fn ensure_bytes(bytes: &[u8], offset: usize, expected: &[u8], role: &str) -> Result<()> {
    ensure!(
        bytes.get(offset..offset + expected.len()) == Some(expected),
        "{PLAYBACK_DRIVER} {role} does not match at file offset {offset:#x}"
    );
    Ok(())
}

#[cfg(test)]
#[path = "sample_playback_tests.rs"]
mod sample_playback_tests;
