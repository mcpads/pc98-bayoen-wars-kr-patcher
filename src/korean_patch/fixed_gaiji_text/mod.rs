mod compile;
mod model;
mod pages;
mod plans;
mod readback;
mod report;

use std::collections::BTreeMap;
use std::path::Path;

use anyhow::{Context, Result, ensure};

use self::model::PatchedFixedGaijiTextPayload;
use self::readback::verify_fixed_gaiji_text_payload;
use super::font_rasterizer::font_provenance;
use super::payload_writes::apply_payload_write_plans;
use super::shared_text::gaiji_bank_plan;
use crate::game_data::catalog_game_data;
use crate::source_disk::SOURCE_DISK_SHA256;
use crate::translation_analysis::load_translation_corpus;

const SEGMENT_ID: &str = "mad-fixed-gaiji";

pub(in crate::korean_patch) use compile::compile_fixed_gaiji_text;
pub(in crate::korean_patch) use model::CompiledFixedGaijiText;
pub use model::{FixedGaijiTextPatchReport, FixedGaijiTextSlotPatchReport};
pub(in crate::korean_patch) use plans::fixed_slots_plan;
pub(in crate::korean_patch) use readback::verify_fixed_gaiji_text_component;
pub(in crate::korean_patch) use report::fixed_gaiji_text_report;

pub(crate) fn build_fixed_gaiji_text_payload(
    installer_payload: &BTreeMap<String, Vec<u8>>,
    translations: &Path,
) -> Result<PatchedFixedGaijiTextPayload> {
    let corpus = load_translation_corpus(translations)?;
    ensure!(
        corpus.index.supported_source_sha256 == SOURCE_DISK_SHA256,
        "translation corpus targets a different source disk"
    );
    let mad_com = installer_payload
        .get("MAD.COM")
        .context("verified installer payload is missing MAD.COM")?;
    let gaiji_com = installer_payload
        .get("GAIJI.COM")
        .context("verified installer payload is missing GAIJI.COM")?;
    let menu_com = installer_payload
        .get("MENU.COM")
        .context("verified installer payload is missing MENU.COM")?;
    let source = catalog_game_data(mad_com, gaiji_com, menu_com)?;
    let compiled = compile_fixed_gaiji_text(gaiji_com, &source.mad_com, &source.gaiji, &corpus)?;
    let plans = vec![
        gaiji_bank_plan(
            &compiled.bank,
            "fixed-text",
            "fixed-text-gaiji-bank",
            "install reviewed fixed-text glyphs",
        ),
        fixed_slots_plan(mad_com, &compiled)?,
    ];
    let applied = apply_payload_write_plans(installer_payload, plans)?;
    verify_fixed_gaiji_text_payload(&applied.files, &source.gaiji, &compiled)?;
    let font = font_provenance()?;
    Ok(PatchedFixedGaijiTextPayload {
        report: fixed_gaiji_text_report(
            corpus.index.supported_source_sha256,
            font,
            &compiled,
            applied.report,
        ),
        files: applied.files,
    })
}

#[cfg(test)]
#[path = "fixed_gaiji_text_tests.rs"]
mod fixed_gaiji_text_tests;
