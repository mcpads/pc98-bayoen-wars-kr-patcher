use std::collections::BTreeMap;

use serde::Serialize;

use super::super::baked_graphics::TitleArtworkPatchReport;
use super::super::character_assets::ArleCharacterAssetPatchReport;
use super::super::font_catalog::LocalizationFontProfileReport;
use super::super::monochrome_text::DevelopmentBuildStatus;
use super::super::payload_writes::PayloadWriteReport;

#[derive(Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum IntegratedLocalizationFileDisposition {
    PatchedDevelopment,
    PreservedOriginalAsset,
    RetainedSourceAudio,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "disposition", rename_all = "snake_case")]
pub enum ArleCharacterAssetBuildReport {
    OriginalPreserved,
    Replaced {
        report: Box<ArleCharacterAssetPatchReport>,
    },
}

impl ArleCharacterAssetBuildReport {
    pub fn replacement(&self) -> Option<&ArleCharacterAssetPatchReport> {
        match self {
            Self::OriginalPreserved => None,
            Self::Replaced { report } => Some(report),
        }
    }

    pub fn original_is_preserved(&self) -> bool {
        matches!(self, Self::OriginalPreserved)
    }
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct IntegratedLocalizationComponentReport {
    pub id: String,
    pub modified_files: Vec<String>,
    pub final_expected_write_count: usize,
    pub verified_inner_write_count: usize,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct IntegratedLocalizationFileReport {
    pub file_name: String,
    pub disposition: IntegratedLocalizationFileDisposition,
    pub producer: String,
    pub source_size: usize,
    pub output_size: usize,
    pub output_sha256: String,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct IntegratedLocalizationPatchReport {
    pub supported_source_sha256: String,
    pub translation_status: String,
    pub build_status: DevelopmentBuildStatus,
    pub translation_unit_count: usize,
    pub translated_text_unit_count: usize,
    pub preserved_source_control_unit_count: usize,
    pub retained_source_audio_unit_count: usize,
    pub target_file_count: usize,
    pub modified_file_count: usize,
    pub retained_source_audio_file_count: usize,
    pub font_profiles: Vec<LocalizationFontProfileReport>,
    pub title_artwork: Option<TitleArtworkPatchReport>,
    pub arle_character_asset: ArleCharacterAssetBuildReport,
    pub components: Vec<IntegratedLocalizationComponentReport>,
    pub files: Vec<IntegratedLocalizationFileReport>,
    pub composition_writes: Vec<PayloadWriteReport>,
}

pub(crate) struct PatchedIntegratedLocalizationFiles {
    pub boot_files: BTreeMap<String, Vec<u8>>,
    pub runtime_files: BTreeMap<String, Vec<u8>>,
    pub installer_payload: BTreeMap<String, Vec<u8>>,
    pub report: IntegratedLocalizationPatchReport,
}
