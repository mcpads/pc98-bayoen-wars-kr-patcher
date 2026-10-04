use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Component, Path};

use anyhow::{Context, Result, ensure};
use serde::Deserialize;

use super::frame_placement::FramePlacement;
use crate::character_runtime::{
    CHARACTER_PALETTE_TABLE_FILE_OFFSET, SOURCE_CHARACTER_PALETTE_RGB4,
};
use crate::source_disk::SOURCE_DISK_SHA256;
use crate::source_disk::sha256_hex;

const MANIFEST_ID: &str = "arle-battle-character-asset";
const STATUS: &str = "development_only";
const APPROVAL_STATUS: &str = "needs_human_review";
const SOURCE_KIND: &str = "project-authored-normalized-pc98-rgb-sheet";
const SOURCE_PALETTE: &str = "pc98_daimadou_senryaku_runtime_palette";
const PALETTE: &str = "mad_com_character_runtime_palette_rgb4";
const QUANTIZATION: &str = "nearest_runtime_palette_after_frame_placement";
const PLACEMENT: &str = "frame_binding_policy";
const TARGET_ASSETS: [&str; 2] = ["C07", "C08"];

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ArleCharacterAssetManifest {
    #[serde(skip)]
    pub(super) manifest_sha256: String,
    pub(super) id: String,
    pub(super) status: String,
    pub(super) approval_status: String,
    pub(super) source: AuthoringSource,
    pub(super) consumer: ArleConsumer,
    pub(super) frame_bindings: Vec<FrameBinding>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct AuthoringSource {
    pub(super) kind: String,
    pub(super) origin_repository: String,
    pub(super) origin_commit: String,
    pub(super) origin_asset: String,
    pub(super) origin_manifest: String,
    pub(super) origin_manifest_sha256: String,
    pub(super) artwork_sha256: String,
    pub(super) authoring_asset: String,
    pub(super) authoring_sha256: String,
    pub(super) palette: String,
    pub(super) palette_rgb: [[u8; 3]; 16],
    pub(super) width: usize,
    pub(super) height: usize,
    pub(super) columns: usize,
    pub(super) rows: usize,
    pub(super) panel_width: usize,
    pub(super) panel_height: usize,
    pub(super) background_rgb: [u8; 3],
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ArleConsumer {
    pub(super) supported_source_sha256: String,
    pub(super) unit_name_table_file_offset: usize,
    pub(super) unit_name_index: usize,
    pub(super) unit_name_entry_id: String,
    pub(super) slot_table_file_offset: usize,
    pub(super) slot_index: usize,
    pub(super) primary_asset: String,
    pub(super) secondary_asset: String,
    pub(super) palette: String,
    pub(super) palette_table_file_offset: usize,
    pub(super) runtime_palette_rgb4: [[u8; 3]; 16],
    pub(super) quantization: String,
    pub(super) background_palette_index: u8,
    pub(super) placement: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct FrameBinding {
    pub(super) target_asset: String,
    pub(super) target_source_start: usize,
    pub(super) target_width: usize,
    pub(super) target_height: usize,
    pub(super) authoring_frame_index: usize,
    pub(super) role: String,
    pub(super) placement: FramePlacement,
    pub(super) source_coordinate_reference: Option<String>,
    pub(super) subject_scale: Option<SubjectScaleReference>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct SubjectScaleReference {
    pub(super) source_bounds: SubjectBounds,
    pub(super) target_height: usize,
    pub(super) target_baseline_y: usize,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct SubjectBounds {
    pub(super) x: usize,
    pub(super) y: usize,
    pub(super) width: usize,
    pub(super) height: usize,
}

pub(super) fn load_manifest(path: &Path) -> Result<ArleCharacterAssetManifest> {
    let bytes = fs::read(path)
        .with_context(|| format!("failed to read Arle character manifest {}", path.display()))?;
    let mut manifest: ArleCharacterAssetManifest =
        serde_json::from_slice(&bytes).context("failed to parse Arle character asset manifest")?;
    manifest.manifest_sha256 = sha256_hex(&bytes);
    validate_manifest(&manifest)?;
    validate_authoring_asset(path, &manifest.source)?;
    Ok(manifest)
}

fn validate_authoring_asset(path: &Path, source: &AuthoringSource) -> Result<()> {
    let declared_path = Path::new(&source.authoring_asset);
    ensure!(
        !source.authoring_asset.is_empty()
            && declared_path
                .components()
                .all(|component| matches!(component, Component::Normal(_))),
        "Arle authoring asset must be a child path relative to its manifest"
    );
    let authoring_path = path
        .parent()
        .context("Arle character manifest has no parent directory")?
        .join(declared_path);
    let bytes = fs::read(&authoring_path).with_context(|| {
        format!(
            "failed to read declared Arle authoring asset {}",
            authoring_path.display()
        )
    })?;
    ensure!(
        sha256_hex(&bytes) == source.authoring_sha256,
        "Arle authoring asset hash differs from its manifest"
    );
    Ok(())
}

fn validate_manifest(manifest: &ArleCharacterAssetManifest) -> Result<()> {
    ensure!(
        manifest.id == MANIFEST_ID,
        "unexpected Arle character manifest id"
    );
    ensure!(
        manifest.status == STATUS && manifest.approval_status == APPROVAL_STATUS,
        "Arle character asset must remain development_only and needs_human_review"
    );
    ensure!(
        manifest.source.kind == SOURCE_KIND,
        "unexpected Arle artwork kind"
    );
    ensure!(
        !manifest.source.origin_repository.is_empty()
            && is_lower_hex(&manifest.source.origin_commit, 40)
            && !manifest.source.origin_asset.is_empty()
            && !manifest.source.origin_manifest.is_empty()
            && is_lower_hex(&manifest.source.origin_manifest_sha256, 64)
            && is_lower_hex(&manifest.source.artwork_sha256, 64)
            && !manifest.source.authoring_asset.is_empty()
            && is_lower_hex(&manifest.source.authoring_sha256, 64),
        "Arle artwork provenance is incomplete"
    );
    ensure!(
        manifest.source.columns == 5
            && manifest.source.rows == 2
            && manifest.source.panel_width == 192
            && manifest.source.panel_height == 192
            && manifest.source.width == manifest.source.columns * manifest.source.panel_width
            && manifest.source.height == manifest.source.rows * manifest.source.panel_height,
        "Arle artwork sheet geometry changed"
    );
    ensure!(
        manifest.source.background_rgb == [0, 0, 0],
        "Arle artwork panels must use exact black as their declared background"
    );
    ensure!(
        manifest.source.palette == SOURCE_PALETTE
            && manifest.source.palette_rgb[0] == manifest.source.background_rgb,
        "Arle normalized artwork source palette changed"
    );
    ensure!(
        manifest.consumer.supported_source_sha256 == SOURCE_DISK_SHA256,
        "Arle character asset targets a different source disk"
    );
    ensure!(
        manifest.consumer.unit_name_table_file_offset == 0x42d6
            && manifest.consumer.unit_name_index == 6
            && manifest.consumer.unit_name_entry_id == "interface-text-013"
            && manifest.consumer.slot_table_file_offset == 0x7bdd
            && manifest.consumer.slot_index == 6
            && manifest.consumer.primary_asset == "C07"
            && manifest.consumer.secondary_asset == "C08",
        "Arle character consumer binding changed"
    );
    ensure!(
        manifest.consumer.palette == PALETTE
            && manifest.consumer.palette_table_file_offset == CHARACTER_PALETTE_TABLE_FILE_OFFSET
            && manifest.consumer.runtime_palette_rgb4 == SOURCE_CHARACTER_PALETTE_RGB4
            && manifest.consumer.quantization == QUANTIZATION
            && manifest.consumer.background_palette_index == 0
            && manifest.consumer.placement == PLACEMENT,
        "Arle character palette or placement policy changed"
    );
    ensure!(
        manifest.frame_bindings.len() == 8,
        "Arle character manifest needs eight Bayoen frame bindings"
    );
    let identities = manifest
        .frame_bindings
        .iter()
        .map(|binding| (binding.target_asset.as_str(), binding.target_source_start))
        .collect::<BTreeSet<_>>();
    ensure!(
        identities.len() == manifest.frame_bindings.len(),
        "Arle frame bindings are duplicated"
    );
    ensure!(
        manifest.frame_bindings.iter().all(|binding| {
            TARGET_ASSETS.contains(&binding.target_asset.as_str())
                && binding.target_width > 0
                && binding.target_width.is_multiple_of(8)
                && binding.target_height > 0
                && binding.authoring_frame_index < manifest.source.columns * manifest.source.rows
                && !binding.role.is_empty()
        }),
        "Arle frame binding contains an unsupported target or geometry"
    );
    ensure!(
        manifest.frame_bindings.iter().all(|binding| {
            match binding.target_asset.as_str() {
                "C07" => matches!(
                    binding.placement,
                    FramePlacement::ContainConsumerVisibleBounds
                        | FramePlacement::PreserveSharedSourceCoordinatesCropFrame
                ),
                "C08" => {
                    binding.placement
                        == FramePlacement::NormalizeSubjectHeightAlignBaselineCropXOverflow
                }
                _ => false,
            }
        }),
        "Arle frame placement does not match its target asset"
    );
    let mut prior_roles = BTreeMap::new();
    for binding in &manifest.frame_bindings {
        match (
            binding.placement,
            binding.source_coordinate_reference.as_deref(),
        ) {
            (FramePlacement::PreserveSharedSourceCoordinatesCropFrame, Some(reference)) => {
                ensure!(
                    prior_roles.get(reference) == Some(&binding.target_asset.as_str()),
                    "Arle shared source coordinates must reference a prior role in the same target asset"
                );
            }
            (FramePlacement::PreserveSharedSourceCoordinatesCropFrame, None) => {
                anyhow::bail!("Arle shared source coordinate placement requires a reference role");
            }
            (_, None) => {}
            (_, Some(_)) => {
                anyhow::bail!(
                    "Arle source coordinate reference requires shared coordinate placement"
                );
            }
        }
        ensure!(
            prior_roles
                .insert(binding.role.as_str(), binding.target_asset.as_str())
                .is_none(),
            "Arle frame roles are duplicated"
        );
    }
    ensure!(
        manifest.frame_bindings.iter().all(|binding| {
            match (&binding.target_asset[..], &binding.subject_scale) {
                ("C07", None) => true,
                ("C08", Some(reference)) => {
                    let bounds = &reference.source_bounds;
                    bounds.width > 0
                        && bounds.height > 0
                        && bounds.x + bounds.width <= manifest.source.panel_width
                        && bounds.y + bounds.height <= manifest.source.panel_height
                        && reference.target_height == 156
                        && reference.target_baseline_y == 178
                        && reference.target_baseline_y < binding.target_height
                }
                _ => false,
            }
        }),
        "Arle subject scale references do not match the normalized artwork"
    );
    Ok(())
}

fn is_lower_hex(value: &str, expected_len: usize) -> bool {
    value.len() == expected_len
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

#[cfg(test)]
#[path = "manifest_tests.rs"]
mod manifest_tests;
