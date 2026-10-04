use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::{Cursor, Read};
use std::path::Path;

use anyhow::{Context, Result, ensure};
use fatfs::{FatType, FileSystem, FsOptions};
use serde::Serialize;

use crate::lha_sfx::payload_write_order;
use crate::source_disk::{
    BOOT_SECTOR_SIZE, PRESERVED_BOOT_FILES, REQUIRED_RUNTIME_FILES, SOURCE_DISK_SHA256,
    SOURCE_DISK_SIZE, SourceFiles, fatfs_mount_copy, sha256_hex,
};
use crate::{SourceVerification, write_new_output};

const PATCH_FORMAT: &str = "retrogame-patcher-pc98-fat12-raw-sfn-file-bps";
const INSTALLER_NAME: &str = "DMADOU.EXE";
const PATCH_ID: &str = "bayoen-wars-pc98-ko-1.0.0";
const PATCH_TITLE: &str = "바요엔 워즈 한국어 패치 1.0.0";
const PATCH_OUTPUT_FILENAME: &str = "bayoen-wars-ko-1.0.0.hdm";
const IN_GAME_PATCH_FILES: [&str; 7] = [
    "EDM.DAT",
    "GAIJI.COM",
    "MAD.COM",
    "OPM.DAT",
    "SEL1.DAT",
    "SEL3.DAT",
    "TITLE.DAT",
];

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct RetroPatcherPlanReport {
    pub source_sha256: String,
    pub content_image_sha256: String,
    pub plan_sha256: String,
    pub root_file_count: usize,
    pub retained_file_count: usize,
    pub copied_file_count: usize,
    pub patched_file_count: usize,
    pub patched_files: Vec<String>,
    pub build_status: String,
    pub approval_status: String,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct RetroPatcherResultVerificationReport {
    pub source_sha256: String,
    pub content_image_sha256: String,
    pub candidate_image_sha256: String,
    pub logical_file_count: usize,
    pub logical_files_match: bool,
    pub source_boot_sector_preserved: bool,
}

#[derive(Serialize)]
struct PatchAuthorPlan {
    format: &'static str,
    id: &'static str,
    title: &'static str,
    output_filename: &'static str,
    source: PlanSource,
    assembly: PlanAssembly,
}

#[derive(Serialize)]
struct PlanSource {
    size: usize,
    sha256: String,
    geometry: Fat12Geometry,
    mount_policy: &'static str,
}

#[derive(Serialize)]
struct Fat12Geometry {
    bytes_per_sector: usize,
    sectors_per_cluster: usize,
    reserved_sectors: usize,
    fat_count: usize,
    root_entries: usize,
    total_sectors: usize,
    media_descriptor: usize,
    sectors_per_fat: usize,
    sectors_per_track: usize,
    heads: usize,
}

#[derive(Serialize)]
struct PlanAssembly {
    retained_files: Vec<ExactFile>,
    placed_files: Vec<PlacedFile>,
}

#[derive(Serialize)]
struct ExactFile {
    name: String,
    size: usize,
    sha256: String,
}

#[derive(Serialize)]
struct PlacedFile {
    patch_key: String,
    name: String,
    source: FileSource,
    source_size: usize,
    source_sha256: String,
    transform: FileTransform,
}

#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum FileSource {
    RootFile { name: String },
    MzLhaMember { container: String, member: String },
}

#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum FileTransform {
    Copy,
    Bps,
}

pub fn write_in_game_plan(
    verification: &SourceVerification,
    source_image: &[u8],
    source_files: &SourceFiles,
    installer_payload: &BTreeMap<String, Vec<u8>>,
    content_image_path: &Path,
    output_path: &Path,
) -> Result<RetroPatcherPlanReport> {
    ensure!(
        !output_path.exists(),
        "refusing to overwrite existing output: {}",
        output_path.display()
    );
    let content_image = fs::read(content_image_path).with_context(|| {
        format!(
            "failed to read in-game localization content image: {}",
            content_image_path.display()
        )
    })?;
    require_image_envelope(source_image, &content_image, "content image")?;
    let content_files = read_root_files(&content_image)?;
    let plan = create_plan(
        verification,
        source_files,
        installer_payload,
        &content_files,
    )?;
    let plan_bytes = format!("{}\n", serde_json::to_string_pretty(&plan)?).into_bytes();
    let patched_files = patched_file_names(&plan);
    let report = RetroPatcherPlanReport {
        source_sha256: verification.sha256.clone(),
        content_image_sha256: sha256_hex(&content_image),
        plan_sha256: sha256_hex(&plan_bytes),
        root_file_count: content_files.len(),
        retained_file_count: plan.assembly.retained_files.len(),
        copied_file_count: plan
            .assembly
            .placed_files
            .iter()
            .filter(|file| matches!(file.transform, FileTransform::Copy))
            .count(),
        patched_file_count: patched_files.len(),
        patched_files,
        build_status: "development_only".to_owned(),
        approval_status: "needs_human_review".to_owned(),
    };
    write_new_output(output_path, &plan_bytes)?;
    Ok(report)
}

pub fn verify_applied_result(
    source_image: &[u8],
    content_image_path: &Path,
    candidate_image_path: &Path,
) -> Result<RetroPatcherResultVerificationReport> {
    let content_image = fs::read(content_image_path).with_context(|| {
        format!(
            "failed to read content image: {}",
            content_image_path.display()
        )
    })?;
    let candidate_image = fs::read(candidate_image_path).with_context(|| {
        format!(
            "failed to read applied candidate: {}",
            candidate_image_path.display()
        )
    })?;
    require_image_envelope(source_image, &content_image, "content image")?;
    require_image_envelope(source_image, &candidate_image, "applied candidate")?;
    let content_files = read_root_files(&content_image)?;
    let candidate_files = read_root_files(&candidate_image)?;
    ensure!(
        candidate_files == content_files,
        "Retro Patcher result logical files differ from the in-game content build"
    );
    Ok(RetroPatcherResultVerificationReport {
        source_sha256: sha256_hex(source_image),
        content_image_sha256: sha256_hex(&content_image),
        candidate_image_sha256: sha256_hex(&candidate_image),
        logical_file_count: candidate_files.len(),
        logical_files_match: true,
        source_boot_sector_preserved: true,
    })
}

fn create_plan(
    verification: &SourceVerification,
    source_files: &SourceFiles,
    installer_payload: &BTreeMap<String, Vec<u8>>,
    content_files: &BTreeMap<String, Vec<u8>>,
) -> Result<PatchAuthorPlan> {
    ensure!(
        verification.size == SOURCE_DISK_SIZE && verification.sha256 == SOURCE_DISK_SHA256,
        "Retro Patcher plan targets a different source image"
    );
    let baseline = standalone_source_files(source_files, installer_payload)?;
    ensure!(
        content_files.keys().eq(baseline.keys()),
        "in-game content root population differs from the verified 73-file standalone game"
    );
    let changed = content_files
        .iter()
        .filter_map(|(name, bytes)| (baseline[name] != *bytes).then_some(name.as_str()))
        .collect::<BTreeSet<_>>();
    let expected_changed = IN_GAME_PATCH_FILES.into_iter().collect::<BTreeSet<_>>();
    ensure!(
        changed == expected_changed,
        "in-game content changed the wrong logical files: expected {expected_changed:?}, got {changed:?}"
    );

    let retained_files = PRESERVED_BOOT_FILES
        .iter()
        .map(|name| exact_file(name, &source_files.boot_files[*name]))
        .collect::<Vec<_>>();
    let mut placed_files =
        Vec::with_capacity(REQUIRED_RUNTIME_FILES.len() + payload_write_order().len());
    for name in REQUIRED_RUNTIME_FILES {
        let source = &source_files.runtime_files[name];
        ensure!(
            content_files[name] == *source,
            "in-game content changed excluded runtime file {name}"
        );
        placed_files.push(placed_file(
            name,
            source,
            FileSource::RootFile {
                name: name.to_owned(),
            },
            FileTransform::Copy,
        ));
    }
    for &name in payload_write_order() {
        let source = &installer_payload[name];
        let transform = if content_files[name] == *source {
            FileTransform::Copy
        } else {
            FileTransform::Bps
        };
        placed_files.push(placed_file(
            name,
            source,
            FileSource::MzLhaMember {
                container: INSTALLER_NAME.to_owned(),
                member: name.to_owned(),
            },
            transform,
        ));
    }
    ensure!(
        retained_files.len() + placed_files.len() == 73,
        "Retro Patcher plan no longer covers the complete 73-file game"
    );
    Ok(PatchAuthorPlan {
        format: PATCH_FORMAT,
        id: PATCH_ID,
        title: PATCH_TITLE,
        output_filename: PATCH_OUTPUT_FILENAME,
        source: PlanSource {
            size: verification.size,
            sha256: verification.sha256.clone(),
            geometry: Fat12Geometry {
                bytes_per_sector: 1024,
                sectors_per_cluster: 1,
                reserved_sectors: 1,
                fat_count: 2,
                root_entries: 192,
                total_sectors: 1232,
                media_descriptor: 0xfe,
                sectors_per_fat: 2,
                sectors_per_track: 8,
                heads: 2,
            },
            mount_policy: "pc98_dos3",
        },
        assembly: PlanAssembly {
            retained_files,
            placed_files,
        },
    })
}

fn standalone_source_files(
    source_files: &SourceFiles,
    installer_payload: &BTreeMap<String, Vec<u8>>,
) -> Result<BTreeMap<String, Vec<u8>>> {
    let mut files = source_files.boot_files.clone();
    files.extend(source_files.runtime_files.clone());
    files.extend(installer_payload.clone());
    ensure!(
        files.len() == 73,
        "verified standalone source population changed"
    );
    Ok(files)
}

fn exact_file(name: &str, bytes: &[u8]) -> ExactFile {
    ExactFile {
        name: name.to_owned(),
        size: bytes.len(),
        sha256: sha256_hex(bytes),
    }
}

fn placed_file(
    name: &str,
    source: &[u8],
    file_source: FileSource,
    transform: FileTransform,
) -> PlacedFile {
    PlacedFile {
        patch_key: name.to_owned(),
        name: name.to_owned(),
        source: file_source,
        source_size: source.len(),
        source_sha256: sha256_hex(source),
        transform,
    }
}

fn patched_file_names(plan: &PatchAuthorPlan) -> Vec<String> {
    plan.assembly
        .placed_files
        .iter()
        .filter(|file| matches!(file.transform, FileTransform::Bps))
        .map(|file| file.name.clone())
        .collect()
}

fn require_image_envelope(source: &[u8], image: &[u8], role: &str) -> Result<()> {
    ensure!(
        image.len() == source.len(),
        "{role} size differs from the supported source"
    );
    ensure!(
        image.get(..BOOT_SECTOR_SIZE) == source.get(..BOOT_SECTOR_SIZE),
        "{role} changed the supported source boot sector"
    );
    Ok(())
}

fn read_root_files(image: &[u8]) -> Result<BTreeMap<String, Vec<u8>>> {
    let cursor = Cursor::new(fatfs_mount_copy(image)?);
    let filesystem =
        FileSystem::new(cursor, FsOptions::new()).context("failed to mount FAT12 image")?;
    ensure!(
        matches!(filesystem.fat_type(), FatType::Fat12),
        "image is not FAT12"
    );
    let root = filesystem.root_dir();
    let mut files = BTreeMap::new();
    for entry in root.iter() {
        let entry = entry.context("failed to enumerate FAT12 root")?;
        ensure!(
            !entry.is_dir(),
            "standalone image contains an unexpected directory"
        );
        let name = entry.file_name().to_ascii_uppercase();
        let mut file = root
            .open_file(&name)
            .with_context(|| format!("failed to open FAT12 file {name}"))?;
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes)
            .with_context(|| format!("failed to read FAT12 file {name}"))?;
        ensure!(
            files.insert(name.clone(), bytes).is_none(),
            "duplicate FAT12 file {name}"
        );
    }
    Ok(files)
}

#[cfg(test)]
#[path = "retro_patcher_plan_tests.rs"]
mod retro_patcher_plan_tests;
