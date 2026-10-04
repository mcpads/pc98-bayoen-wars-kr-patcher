use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};

use crate::source_disk::sha256_hex;

const PROFILE_SCHEMA: &str = "bayoen_wars.font_profile";
const RASTERIZER: &str = "fontdue 0.9.3";

/// Font files and their OFL texts are user-supplied inputs read from this
/// directory, or from `assets/fonts` in the crate root when it is unset.
const FONT_DIRECTORY_VARIABLE: &str = "BAYOEN_WARS_FONT_DIR";

const BODY_PROFILE: &str = include_str!("../../assets/fonts/body16-galmuri14.json");
const NARRATIVE_PROFILE: &str = include_str!("../../assets/fonts/narrative32-galmuri14.json");
const BAKED_DISPLAY_PROFILE: &str =
    include_str!("../../assets/fonts/baked-display16-mulmaru-mono.json");
const DIFFICULTY_PROFILE: &str =
    include_str!("../../assets/fonts/difficulty32-galmuri11-bold.json");
const TITLE_PRIMARY_PROFILE: &str =
    include_str!("../../assets/fonts/title-primary16-galmuri11-bold.json");

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub(crate) enum FontTarget {
    Body16,
    Narrative32,
    BakedDisplay16,
    Difficulty32,
    TitlePrimary16,
}

impl FontTarget {
    pub(crate) const fn id(self) -> &'static str {
        match self {
            Self::Body16 => "body16",
            Self::Narrative32 => "narrative32",
            Self::BakedDisplay16 => "baked-display16",
            Self::Difficulty32 => "difficulty32",
            Self::TitlePrimary16 => "title-primary16",
        }
    }

    pub(crate) const fn cell_size(self) -> usize {
        match self {
            Self::Narrative32 | Self::Difficulty32 => 32,
            Self::Body16 | Self::BakedDisplay16 | Self::TitlePrimary16 => 16,
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub(crate) struct FontProvenance {
    pub profile_id: String,
    pub font_sha256: String,
    pub font_version: String,
    pub source: String,
    pub upstream_revision: String,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct LocalizationFontProfileReport {
    pub consumer_target: String,
    pub font_profile: String,
    pub font_sha256: String,
    pub font_version: String,
    pub font_source: String,
    pub font_upstream_revision: String,
}

pub(crate) struct LoadedFontProfile {
    pub font_bytes: &'static [u8],
    pub profile: FontProfile,
}

#[derive(Debug, Deserialize)]
pub(crate) struct FontProfile {
    schema: String,
    pub id: String,
    target: String,
    font: String,
    pub font_sha256: String,
    pub font_version: String,
    pub font_size: u16,
    pub baseline_y: u8,
    pub threshold: u8,
    rasterizer: String,
    license: String,
    pub source: String,
    pub upstream_revision: String,
}

struct EmbeddedFont {
    profile_json: &'static str,
    font_name: &'static str,
    font_bytes: &'static [u8],
    license_name: &'static str,
    license_text: &'static str,
}

struct ExternalFont {
    profile_json: &'static str,
    font_name: &'static str,
    license_name: &'static str,
}

fn font_directory() -> PathBuf {
    std::env::var_os(FONT_DIRECTORY_VARIABLE)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets/fonts"))
}

fn read_font_input(name: &str) -> Result<&'static [u8]> {
    static CACHE: OnceLock<Mutex<BTreeMap<PathBuf, &'static [u8]>>> = OnceLock::new();
    let path = font_directory().join(name);
    let mut cache = CACHE
        .get_or_init(|| Mutex::new(BTreeMap::new()))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if let Some(bytes) = cache.get(&path) {
        return Ok(bytes);
    }
    let bytes = std::fs::read(&path).with_context(|| {
        format!(
            "required font input {} is unavailable (set {FONT_DIRECTORY_VARIABLE} or place it in assets/fonts)",
            path.display()
        )
    })?;
    let bytes: &'static [u8] = Box::leak(bytes.into_boxed_slice());
    cache.insert(path, bytes);
    Ok(bytes)
}

fn embedded_font(target: FontTarget) -> Result<EmbeddedFont> {
    let external = external_font(target);
    let license_bytes = read_font_input(external.license_name)?;
    let license_text = std::str::from_utf8(license_bytes)
        .with_context(|| format!("{} is not UTF-8 text", external.license_name))?;
    Ok(EmbeddedFont {
        profile_json: external.profile_json,
        font_name: external.font_name,
        font_bytes: read_font_input(external.font_name)?,
        license_name: external.license_name,
        license_text,
    })
}

pub(crate) fn load_font_profile(target: FontTarget) -> Result<LoadedFontProfile> {
    let embedded = embedded_font(target)?;
    let profile: FontProfile = serde_json::from_str(embedded.profile_json)
        .with_context(|| format!("failed to parse {} font profile", target.id()))?;
    validate_profile(target, &profile, &embedded)?;
    Ok(LoadedFontProfile {
        font_bytes: embedded.font_bytes,
        profile,
    })
}

pub(crate) fn font_provenance_for(target: FontTarget) -> Result<FontProvenance> {
    let profile = load_font_profile(target)?.profile;
    Ok(FontProvenance {
        profile_id: profile.id,
        font_sha256: profile.font_sha256,
        font_version: profile.font_version,
        source: profile.source,
        upstream_revision: profile.upstream_revision,
    })
}

pub(crate) fn font_profile_report_for(target: FontTarget) -> Result<LocalizationFontProfileReport> {
    let font = font_provenance_for(target)?;
    Ok(LocalizationFontProfileReport {
        consumer_target: target.id().to_owned(),
        font_profile: font.profile_id,
        font_sha256: font.font_sha256,
        font_version: font.font_version,
        font_source: font.source,
        font_upstream_revision: font.upstream_revision,
    })
}

fn external_font(target: FontTarget) -> ExternalFont {
    match target {
        FontTarget::Body16 => ExternalFont {
            profile_json: BODY_PROFILE,
            font_name: "Galmuri14.ttf",
            license_name: "Galmuri-OFL.txt",
        },
        FontTarget::Narrative32 => ExternalFont {
            profile_json: NARRATIVE_PROFILE,
            font_name: "Galmuri14.ttf",
            license_name: "Galmuri-OFL.txt",
        },
        FontTarget::BakedDisplay16 => ExternalFont {
            profile_json: BAKED_DISPLAY_PROFILE,
            font_name: "MulmaruMono.ttf",
            license_name: "Mulmaru-OFL.txt",
        },
        FontTarget::Difficulty32 => ExternalFont {
            profile_json: DIFFICULTY_PROFILE,
            font_name: "Galmuri11-Bold.ttf",
            license_name: "Galmuri-OFL.txt",
        },
        FontTarget::TitlePrimary16 => ExternalFont {
            profile_json: TITLE_PRIMARY_PROFILE,
            font_name: "Galmuri11-Bold.ttf",
            license_name: "Galmuri-OFL.txt",
        },
    }
}

fn validate_profile(
    target: FontTarget,
    profile: &FontProfile,
    embedded: &EmbeddedFont,
) -> Result<()> {
    ensure!(
        profile.schema == PROFILE_SCHEMA,
        "unsupported embedded font profile schema {:?}",
        profile.schema
    );
    ensure!(
        profile.target == target.id(),
        "{} profile declares target {:?}",
        target.id(),
        profile.target
    );
    ensure!(
        profile.font == embedded.font_name,
        "{} profile names an unexpected font {:?}",
        target.id(),
        profile.font
    );
    ensure!(
        profile.license == embedded.license_name
            && embedded.license_text.contains("SIL OPEN FONT LICENSE"),
        "{} profile has no matching OFL license",
        target.id()
    );
    ensure!(
        profile.rasterizer == RASTERIZER,
        "{} profile names an unexpected rasterizer",
        target.id()
    );
    let actual_sha256 = sha256_hex(embedded.font_bytes);
    ensure!(
        actual_sha256 == profile.font_sha256,
        "{} font SHA-256 {actual_sha256} does not match profile {}",
        target.id(),
        profile.font_sha256
    );
    ensure!(
        (1..=target.cell_size()).contains(&usize::from(profile.font_size)),
        "{} font size is outside its {}px consumer cell",
        target.id(),
        target.cell_size(),
    );
    ensure!(
        usize::from(profile.baseline_y) <= target.cell_size(),
        "{} font baseline is outside its {}px consumer cell",
        target.id(),
        target.cell_size()
    );
    ensure!(
        profile.threshold > 0,
        "{} font threshold must be nonzero",
        target.id()
    );
    Ok(())
}

#[cfg(test)]
#[path = "font_catalog_tests.rs"]
mod font_catalog_tests;
