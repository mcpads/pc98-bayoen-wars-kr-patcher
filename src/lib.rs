mod asset_bindings;
mod audio_audit;
mod audit_directory;
mod byte_string;
mod character_runtime;
mod dos_program;
mod external_text;
mod game_data;
mod graphics_audit;
mod korean_patch;
mod lha_sfx;
mod localization_assets;
mod pc98_graphics;
mod reconstruction_diff;
mod retro_patcher_plan;
mod source_disk;
mod standalone_image;
mod title_runtime;
mod title_screen;
mod translation_analysis;
mod translation_drafts;
mod translation_workspace;

use std::collections::BTreeMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use serde::Serialize;
use tempfile::NamedTempFile;

pub use asset_bindings::{
    AssetBindingCatalog, BakedTextCatalog, BakedTextConsumerBlock, BakedTextScreenRegion,
    BakedTextSourceFragment, BakedTextUnit, BattleBackgroundAsset, BattleBackgroundCatalog,
    BattleGraphicsCatalog, BattleMapAsset, BattleMapCatalog, BattleMapDuplicate,
    BattleWindowBinding, BattleWindowTransfer, BoundByteRange, CharacterSpriteAsset,
    CharacterSpriteCatalog, CharacterSpriteSlot, CharacterSpriteTransfer, CompactTileSetBinding,
    MaskedBattleSpriteAsset, MaskedTileSetBinding, MonochromeGlyph, MonochromeSpriteAsset,
    MonochromeSpriteCatalog, MonochromeTextCatalog, MonochromeTextPage, MonochromeTextSequence,
    MovementTableCatalog, MovementTablePointer, MovementTableSection, SampleClockProfile,
    SampleLanguageClassification, SampleLanguageReview, SampleLanguageReviewCatalog,
    SamplePlaybackCatalog, SampleTranscriptStatus, SongSequenceBinding, SoundDataCatalog,
    SupportFileBinding, SupportFileCatalog, SystemIoCatalog, SystemIoDevice, SystemIoDiagnostic,
    SystemIoFlagLabels, TileGraphicsCatalog, TileMapBinding, VoiceSampleCatalog, VoiceSampleUnit,
};
pub use external_text::{
    ExternalProgramTextCatalog, ExternalTextCatalog, ExternalTextEntry, ExternalTextReference,
    ExternalTextStorage, ExternalTextTerminator,
};
pub use game_data::{
    BattleCalloutCall, BattleCalloutCatalog, BattleCalloutPointer, DialogueCatalog, DialogueEntry,
    DialogueGroup, DialogueNonReferenceOccurrence, DialogueReference, DialogueReferenceCatalog,
    DialogueReferenceKind, FixedGaijiTextCatalog, FixedGaijiTextRuntimeCall,
    FixedGaijiTextRuntimeCatalog, FixedGaijiTextSlot, GaijiCatalog, GaijiGlyph, GaijiGlyphMeaning,
    GaijiInstallerCatalog, GaijiTextCell, GameDataCatalog, InterfaceTextCatalog,
    InterfaceTextEntry, InterfaceTextReference, InterfaceTextReferenceCatalog,
    InterfaceTextReferenceKind, InterfaceTextReferenceTable, InterfaceTextToken,
    MadEndingTransitionCatalog, MadMemoryCatalog, MadProgramCatalog, MadSceneTransitionCall,
    MadSceneTransitionCatalog, MadSystemRuntimeCatalog, MadSystemRuntimeReference,
    MadSystemRuntimeReferenceKind, MadSystemTextCatalog, MadSystemTextEntry,
    MadSystemTextReference, MenuEntryRuntimeCatalog, MenuProgramCatalog, MenuRuntimeCatalog,
    MenuRuntimeInsertReference, MenuTextCatalog, MenuTextEntry, MenuTextNonReferenceOccurrence,
    MenuTextReference, MenuTextRuntimeReference, MenuTextRuntimeReferenceKind, RuntimeTextInsert,
    SharedAlertWindowCatalog, SharedAlertWindowState, SpringCaptureWindowCatalog,
    SpringRecoveryWindowCatalog, TextControlCatalog,
};
pub use korean_patch::{
    ARLE_CHARACTER_ASSET_SET_FILENAMES, ArleCharacterAssetBuildReport,
    ArleCharacterAssetPatchReport, ArleCharacterAssetSelection, ArleCharacterAssetSet,
    ArleCharacterFileReport, ArleCharacterFrameReport, BakedGraphicsAssetPatchReport,
    BakedGraphicsTextPatchReport, BattleCalloutEntryPatchReport, BattleCalloutPatchReport,
    DevelopmentBuildStatus, DialogueTextEntryPatchReport, DialogueTextPatchReport,
    FixedGaijiTextPatchReport, FixedGaijiTextSlotPatchReport, HangulVisibilityPatchReport,
    IntegratedLocalizationComponentReport, IntegratedLocalizationFileDisposition,
    IntegratedLocalizationFileReport, IntegratedLocalizationPatchReport,
    InterfaceTextEntryPatchReport, InterfaceTextPatchReport, LocalizationFontProfileReport,
    MadSceneNarrativePatchReport, MadSceneTextPatchReport, MadSceneTransitionPatchReport,
    MadSystemInterfaceTextPatchReport, MadSystemTextDisposition, MadSystemTextEntryPatchReport,
    MadSystemTextPatchReport, MenuTextEntryPatchReport, MenuTextPatchReport, MonochromeGlyphSlot,
    MonochromeSequencePatchReport, MonochromeTextPatchReport, MouseDriverTextEntryPatchReport,
    MouseDriverTextPatchReport, PatchedDialogueComponentReport, PatchedInterfaceComponentReport,
    PatchedSystemComponentReport, PayloadWriteReport, PlaybackDriverPatchReport,
    PlaybackDriverTextEntryPatchReport, PlaybackDriverTextPatchReport,
    SamplingDriverTextEntryPatchReport, SamplingDriverTextPatchReport,
    SharedAlertWindowPatchReport, SharedGaijiGlyph, ShellTextEntryPatchReport,
    ShellTextPatchReport, SpringCaptureWindowPatchReport, SpringRecoveryWindowPatchReport,
    SystemLoaderTextEntryPatchReport, SystemLoaderTextPatchReport, TitleArtworkPatchReport,
    UnpackedWriteReport,
};
pub use localization_assets::{
    AssetOrigin, AssetReviewStatus, CompileLzCatalog, CompileLzConsumer, LocalizationAssetCatalog,
    LocalizationAssetFile, PackedAssetFile, PackedStreamSummary,
};
pub use retro_patcher_plan::{RetroPatcherPlanReport, RetroPatcherResultVerificationReport};
pub use source_disk::{SOURCE_DISK_SHA256, SOURCE_DISK_SIZE};
pub use translation_analysis::{
    SharedGaijiDemandScope, SharedGaijiDemandSetReport, TranslationAnalysisStatus,
    TranslationGlyphAuditReport, TranslationGlyphDemand, TranslationGlyphPathReport,
    TranslationGlyphSegmentReport, TranslationLayoutAuditReport, TranslationLayoutFinding,
    TranslationLayoutFindingKind, TranslationLayoutOpenGate, TranslationLayoutScopeExclusion,
    TranslationLayoutSegmentReport, TranslationRenderPath, audit_translation_glyphs,
    audit_translation_layout,
};
pub use translation_drafts::{TranslationDraftReport, TranslationSurface};
pub use translation_workspace::TranslationWorkspaceReport;

#[derive(Debug, Eq, PartialEq)]
pub struct SourceVerification {
    pub sha256: String,
    pub size: usize,
}

#[derive(Debug, Eq, PartialEq)]
pub struct BuildReport {
    pub source_sha256: String,
    pub output_sha256: String,
    pub output_size: usize,
    pub file_count: usize,
}

#[derive(Debug, Eq, PartialEq)]
pub struct HangulVisibilityBuildReport {
    pub build: BuildReport,
    pub patch: HangulVisibilityPatchReport,
}

#[derive(Debug, Eq, PartialEq)]
pub struct MonochromeTextDevelopmentBuildReport {
    pub build: BuildReport,
    pub patch: MonochromeTextPatchReport,
}

#[derive(Debug, Eq, PartialEq)]
pub struct FixedGaijiTextDevelopmentBuildReport {
    pub build: BuildReport,
    pub patch: FixedGaijiTextPatchReport,
}

#[derive(Debug, Eq, PartialEq)]
pub struct InterfaceTextDevelopmentBuildReport {
    pub build: BuildReport,
    pub patch: InterfaceTextPatchReport,
}

#[derive(Debug, Eq, PartialEq)]
pub struct MadSystemTextDevelopmentBuildReport {
    pub build: BuildReport,
    pub patch: MadSystemTextPatchReport,
}

#[derive(Debug, Eq, PartialEq)]
pub struct MadSystemInterfaceTextDevelopmentBuildReport {
    pub build: BuildReport,
    pub patch: MadSystemInterfaceTextPatchReport,
}

#[derive(Debug, Eq, PartialEq)]
pub struct MadSceneTextDevelopmentBuildReport {
    pub build: BuildReport,
    pub patch: MadSceneTextPatchReport,
}

#[derive(Debug, Eq, PartialEq)]
pub struct MadSceneNarrativeDevelopmentBuildReport {
    pub build: BuildReport,
    pub patch: MadSceneNarrativePatchReport,
}

#[derive(Debug, Eq, PartialEq)]
pub struct IntegratedLocalizationDevelopmentBuildReport {
    pub build: BuildReport,
    pub patch: IntegratedLocalizationPatchReport,
}

#[derive(Debug, Clone, Copy)]
pub struct IntegratedLocalizationDevelopmentInputs<'a> {
    pub translations: &'a Path,
    pub title_artwork_manifest: &'a Path,
    pub title_source_preview: &'a Path,
    pub title_artwork: &'a Path,
    pub arle_character_assets: &'a ArleCharacterAssetSelection,
}

#[derive(Debug, Eq, PartialEq)]
pub struct DialogueTextDevelopmentBuildReport {
    pub build: BuildReport,
    pub patch: DialogueTextPatchReport,
}

#[derive(Debug, Eq, PartialEq)]
pub struct ShellTextDevelopmentBuildReport {
    pub build: BuildReport,
    pub patch: ShellTextPatchReport,
}

#[derive(Debug, Eq, PartialEq)]
pub struct SystemLoaderTextDevelopmentBuildReport {
    pub build: BuildReport,
    pub patch: SystemLoaderTextPatchReport,
}

#[derive(Debug, Eq, PartialEq)]
pub struct SamplingDriverTextDevelopmentBuildReport {
    pub build: BuildReport,
    pub patch: SamplingDriverTextPatchReport,
}

#[derive(Debug, Eq, PartialEq)]
pub struct MouseDriverTextDevelopmentBuildReport {
    pub build: BuildReport,
    pub patch: MouseDriverTextPatchReport,
}

#[derive(Debug, Eq, PartialEq)]
pub struct PlaybackDriverTextDevelopmentBuildReport {
    pub build: BuildReport,
    pub patch: PlaybackDriverTextPatchReport,
}

#[derive(Debug, Eq, PartialEq)]
pub struct MenuTextDevelopmentBuildReport {
    pub build: BuildReport,
    pub patch: MenuTextPatchReport,
}

#[derive(Debug, Eq, PartialEq)]
pub struct NarrativeGraphicsDevelopmentBuildReport {
    pub build: BuildReport,
    pub patch: NarrativeGraphicsPatchReport,
}

#[derive(Debug, Eq, PartialEq)]
pub struct ArleCharacterAssetDevelopmentBuildReport {
    pub build: BuildReport,
    pub patch: ArleCharacterAssetPatchReport,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct NarrativeGraphicsPatchReport {
    pub monochrome: MonochromeTextPatchReport,
    pub baked: BakedGraphicsTextPatchReport,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct GraphicsAuditReport {
    pub output_directory: PathBuf,
    pub preview_count: usize,
    pub previews: Vec<GraphicsAuditPreview>,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct GraphicsAuditPreview {
    pub source_asset: String,
    pub output_file: String,
    pub width: usize,
    pub height: usize,
    pub evidence: String,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct AudioAuditReport {
    pub output_directory: PathBuf,
    pub source_asset: String,
    pub sample_count: usize,
    pub entries: Vec<AudioAuditEntry>,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct AudioAuditEntry {
    pub stream_index: usize,
    pub packed_offset: usize,
    pub packed_size: usize,
    pub decoded_size: usize,
    pub header_size: usize,
    pub encoded_sample_bytes: usize,
    pub sample_count: usize,
    pub sample_rate: u32,
    pub duration_milliseconds: u64,
    pub output_file: String,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct SourceStructureReport {
    pub source_sha256: String,
    pub localization_assets: LocalizationAssetCatalog,
    pub compile_lz: CompileLzCatalog,
    pub asset_bindings: AssetBindingCatalog,
    pub game_data: GameDataCatalog,
    pub external_text: ExternalTextCatalog,
}

struct VerifiedSource {
    image: Vec<u8>,
    verification: SourceVerification,
    source_files: source_disk::SourceFiles,
    installer_payload: BTreeMap<String, Vec<u8>>,
}

pub fn verify_source_path(source_path: &Path) -> Result<SourceVerification> {
    let source = fs::read(source_path)
        .with_context(|| format!("failed to read source disk: {}", source_path.display()))?;
    source_disk::verify_source_bytes(&source)
}

pub fn build_standalone_image(source_path: &Path, output_path: &Path) -> Result<BuildReport> {
    if output_path.exists() {
        bail!(
            "refusing to overwrite existing output: {}",
            output_path.display()
        );
    }

    let verified_source = load_verified_source(source_path)?;
    build_verified_payload(
        &verified_source,
        &verified_source.installer_payload,
        output_path,
    )
}

pub fn build_hangul_visibility_image(
    source_path: &Path,
    output_path: &Path,
) -> Result<HangulVisibilityBuildReport> {
    if output_path.exists() {
        bail!(
            "refusing to overwrite existing output: {}",
            output_path.display()
        );
    }

    let verified_source = load_verified_source(source_path)?;
    let patched = korean_patch::build_hangul_visibility_payload(
        &verified_source.installer_payload,
        &verified_source.source_files.runtime_files,
        &verified_source.source_files.boot_files,
    )?;
    let build = build_verified_payload(&verified_source, &patched.files, output_path)?;
    Ok(HangulVisibilityBuildReport {
        build,
        patch: patched.report,
    })
}

pub fn build_monochrome_text_development_image(
    source_path: &Path,
    translations: &Path,
    output_path: &Path,
) -> Result<MonochromeTextDevelopmentBuildReport> {
    if output_path.exists() {
        bail!(
            "refusing to overwrite existing output: {}",
            output_path.display()
        );
    }

    let verified_source = load_verified_source(source_path)?;
    let patched = korean_patch::build_monochrome_text_payload(
        &verified_source.installer_payload,
        translations,
    )?;
    let build = build_verified_payload(&verified_source, &patched.files, output_path)?;
    Ok(MonochromeTextDevelopmentBuildReport {
        build,
        patch: patched.report,
    })
}

pub fn build_fixed_gaiji_text_development_image(
    source_path: &Path,
    translations: &Path,
    output_path: &Path,
) -> Result<FixedGaijiTextDevelopmentBuildReport> {
    if output_path.exists() {
        bail!(
            "refusing to overwrite existing output: {}",
            output_path.display()
        );
    }

    let verified_source = load_verified_source(source_path)?;
    let patched = korean_patch::build_fixed_gaiji_text_payload(
        &verified_source.installer_payload,
        translations,
    )?;
    let build = build_verified_payload(&verified_source, &patched.files, output_path)?;
    Ok(FixedGaijiTextDevelopmentBuildReport {
        build,
        patch: patched.report,
    })
}

pub fn build_interface_text_development_image(
    source_path: &Path,
    translations: &Path,
    output_path: &Path,
) -> Result<InterfaceTextDevelopmentBuildReport> {
    if output_path.exists() {
        bail!(
            "refusing to overwrite existing output: {}",
            output_path.display()
        );
    }

    let verified_source = load_verified_source(source_path)?;
    let patched = korean_patch::build_interface_text_payload(
        &verified_source.installer_payload,
        translations,
    )?;
    let build = build_verified_payload(&verified_source, &patched.files, output_path)?;
    Ok(InterfaceTextDevelopmentBuildReport {
        build,
        patch: patched.report,
    })
}

pub fn build_mad_system_text_development_image(
    source_path: &Path,
    translations: &Path,
    output_path: &Path,
) -> Result<MadSystemTextDevelopmentBuildReport> {
    if output_path.exists() {
        bail!(
            "refusing to overwrite existing output: {}",
            output_path.display()
        );
    }

    let verified_source = load_verified_source(source_path)?;
    let patched = korean_patch::build_mad_system_text_payload(
        &verified_source.installer_payload,
        translations,
    )?;
    let build = build_verified_payload(&verified_source, &patched.files, output_path)?;
    Ok(MadSystemTextDevelopmentBuildReport {
        build,
        patch: patched.report,
    })
}

pub fn build_mad_system_interface_text_development_image(
    source_path: &Path,
    translations: &Path,
    output_path: &Path,
) -> Result<MadSystemInterfaceTextDevelopmentBuildReport> {
    if output_path.exists() {
        bail!(
            "refusing to overwrite existing output: {}",
            output_path.display()
        );
    }

    let verified_source = load_verified_source(source_path)?;
    let patched = korean_patch::build_mad_system_interface_text_payload(
        &verified_source.installer_payload,
        translations,
    )?;
    let build = build_verified_payload(&verified_source, &patched.files, output_path)?;
    Ok(MadSystemInterfaceTextDevelopmentBuildReport {
        build,
        patch: patched.report,
    })
}

pub fn build_dialogue_text_development_image(
    source_path: &Path,
    translations: &Path,
    output_path: &Path,
) -> Result<DialogueTextDevelopmentBuildReport> {
    if output_path.exists() {
        bail!(
            "refusing to overwrite existing output: {}",
            output_path.display()
        );
    }

    let verified_source = load_verified_source(source_path)?;
    korean_patch::validate_dialogue_rom_layout(translations)?;
    let patched = korean_patch::build_dialogue_text_payload(
        &verified_source.installer_payload,
        translations,
    )?;
    let build = build_verified_payload(&verified_source, &patched.files, output_path)?;
    Ok(DialogueTextDevelopmentBuildReport {
        build,
        patch: patched.report,
    })
}

pub fn build_mad_scene_text_development_image(
    source_path: &Path,
    translations: &Path,
    output_path: &Path,
) -> Result<MadSceneTextDevelopmentBuildReport> {
    if output_path.exists() {
        bail!(
            "refusing to overwrite existing output: {}",
            output_path.display()
        );
    }

    let verified_source = load_verified_source(source_path)?;
    korean_patch::validate_dialogue_rom_layout(translations)?;
    let patched = korean_patch::build_mad_scene_text_payload(
        &verified_source.installer_payload,
        translations,
    )?;
    let build = build_verified_payload(&verified_source, &patched.files, output_path)?;
    Ok(MadSceneTextDevelopmentBuildReport {
        build,
        patch: patched.report,
    })
}

pub fn build_mad_scene_narrative_development_image(
    source_path: &Path,
    translations: &Path,
    output_path: &Path,
) -> Result<MadSceneNarrativeDevelopmentBuildReport> {
    if output_path.exists() {
        bail!(
            "refusing to overwrite existing output: {}",
            output_path.display()
        );
    }

    let verified_source = load_verified_source(source_path)?;
    korean_patch::validate_dialogue_rom_layout(translations)?;
    let patched = korean_patch::build_mad_scene_narrative_payload(
        &verified_source.installer_payload,
        translations,
    )?;
    let build = build_verified_payload(&verified_source, &patched.files, output_path)?;
    Ok(MadSceneNarrativeDevelopmentBuildReport {
        build,
        patch: patched.report,
    })
}

pub fn build_mad_scene_narrative_with_title_artwork_development_image(
    source_path: &Path,
    translations: &Path,
    title_artwork_manifest: &Path,
    title_source_preview: &Path,
    title_artwork: &Path,
    output_path: &Path,
) -> Result<MadSceneNarrativeDevelopmentBuildReport> {
    if output_path.exists() {
        bail!(
            "refusing to overwrite existing output: {}",
            output_path.display()
        );
    }

    let verified_source = load_verified_source(source_path)?;
    korean_patch::validate_dialogue_rom_layout(translations)?;
    let patched = korean_patch::build_mad_scene_narrative_payload_with_title_artwork(
        &verified_source.installer_payload,
        translations,
        title_artwork_manifest,
        title_source_preview,
        title_artwork,
    )?;
    let build = build_verified_payload(&verified_source, &patched.files, output_path)?;
    Ok(MadSceneNarrativeDevelopmentBuildReport {
        build,
        patch: patched.report,
    })
}

pub fn build_integrated_localization_development_image(
    source_path: &Path,
    inputs: IntegratedLocalizationDevelopmentInputs<'_>,
    output_path: &Path,
) -> Result<IntegratedLocalizationDevelopmentBuildReport> {
    if output_path.exists() {
        bail!(
            "refusing to overwrite existing output: {}",
            output_path.display()
        );
    }

    let verified_source = load_verified_source(source_path)?;
    korean_patch::validate_dialogue_rom_layout(inputs.translations)?;
    let patched = korean_patch::build_integrated_localization_files(
        &verified_source.installer_payload,
        &verified_source.source_files.boot_files,
        &verified_source.source_files.runtime_files,
        inputs,
    )?;
    let build = build_verified_files(
        &verified_source,
        &patched.boot_files,
        &patched.runtime_files,
        &patched.installer_payload,
        output_path,
    )?;
    Ok(IntegratedLocalizationDevelopmentBuildReport {
        build,
        patch: patched.report,
    })
}

pub fn build_shell_text_development_image(
    source_path: &Path,
    translations: &Path,
    output_path: &Path,
) -> Result<ShellTextDevelopmentBuildReport> {
    if output_path.exists() {
        bail!(
            "refusing to overwrite existing output: {}",
            output_path.display()
        );
    }

    let verified_source = load_verified_source(source_path)?;
    let patched = korean_patch::build_shell_text_files(
        &verified_source.installer_payload,
        &verified_source.source_files.boot_files,
        translations,
    )?;
    let build = build_verified_files(
        &verified_source,
        &patched.files,
        &verified_source.source_files.runtime_files,
        &verified_source.installer_payload,
        output_path,
    )?;
    Ok(ShellTextDevelopmentBuildReport {
        build,
        patch: patched.report,
    })
}

pub fn build_system_loader_text_development_image(
    source_path: &Path,
    translations: &Path,
    output_path: &Path,
) -> Result<SystemLoaderTextDevelopmentBuildReport> {
    if output_path.exists() {
        bail!(
            "refusing to overwrite existing output: {}",
            output_path.display()
        );
    }

    let verified_source = load_verified_source(source_path)?;
    let patched = korean_patch::build_system_loader_text_files(
        &verified_source.installer_payload,
        &verified_source.source_files.boot_files,
        translations,
    )?;
    let build = build_verified_files(
        &verified_source,
        &patched.files,
        &verified_source.source_files.runtime_files,
        &verified_source.installer_payload,
        output_path,
    )?;
    Ok(SystemLoaderTextDevelopmentBuildReport {
        build,
        patch: patched.report,
    })
}

pub fn build_sampling_driver_text_development_image(
    source_path: &Path,
    translations: &Path,
    output_path: &Path,
) -> Result<SamplingDriverTextDevelopmentBuildReport> {
    if output_path.exists() {
        bail!(
            "refusing to overwrite existing output: {}",
            output_path.display()
        );
    }

    let verified_source = load_verified_source(source_path)?;
    let patched = korean_patch::build_sampling_driver_text_files(
        &verified_source.installer_payload,
        &verified_source.source_files.runtime_files,
        translations,
    )?;
    let build = build_verified_files(
        &verified_source,
        &verified_source.source_files.boot_files,
        &patched.files,
        &verified_source.installer_payload,
        output_path,
    )?;
    Ok(SamplingDriverTextDevelopmentBuildReport {
        build,
        patch: patched.report,
    })
}

pub fn build_mouse_driver_text_development_image(
    source_path: &Path,
    translations: &Path,
    output_path: &Path,
) -> Result<MouseDriverTextDevelopmentBuildReport> {
    if output_path.exists() {
        bail!(
            "refusing to overwrite existing output: {}",
            output_path.display()
        );
    }

    let verified_source = load_verified_source(source_path)?;
    let patched = korean_patch::build_mouse_driver_text_files(
        &verified_source.installer_payload,
        &verified_source.source_files.runtime_files,
        translations,
    )?;
    let build = build_verified_files(
        &verified_source,
        &verified_source.source_files.boot_files,
        &patched.files,
        &verified_source.installer_payload,
        output_path,
    )?;
    Ok(MouseDriverTextDevelopmentBuildReport {
        build,
        patch: patched.report,
    })
}

pub fn build_playback_driver_text_development_image(
    source_path: &Path,
    translations: &Path,
    output_path: &Path,
) -> Result<PlaybackDriverTextDevelopmentBuildReport> {
    if output_path.exists() {
        bail!(
            "refusing to overwrite existing output: {}",
            output_path.display()
        );
    }

    let verified_source = load_verified_source(source_path)?;
    let patched = korean_patch::build_playback_driver_text_payload(
        &verified_source.installer_payload,
        translations,
    )?;
    let build = build_verified_payload(&verified_source, &patched.files, output_path)?;
    Ok(PlaybackDriverTextDevelopmentBuildReport {
        build,
        patch: patched.report,
    })
}

pub fn build_menu_text_development_image(
    source_path: &Path,
    translations: &Path,
    output_path: &Path,
) -> Result<MenuTextDevelopmentBuildReport> {
    if output_path.exists() {
        bail!(
            "refusing to overwrite existing output: {}",
            output_path.display()
        );
    }

    let verified_source = load_verified_source(source_path)?;
    let patched =
        korean_patch::build_menu_text_files(&verified_source.installer_payload, translations)?;
    let build = build_verified_payload(&verified_source, &patched.files, output_path)?;
    Ok(MenuTextDevelopmentBuildReport {
        build,
        patch: patched.report,
    })
}

pub fn build_narrative_graphics_development_image(
    source_path: &Path,
    translations: &Path,
    output_path: &Path,
) -> Result<NarrativeGraphicsDevelopmentBuildReport> {
    if output_path.exists() {
        bail!(
            "refusing to overwrite existing output: {}",
            output_path.display()
        );
    }

    let verified_source = load_verified_source(source_path)?;
    let monochrome = korean_patch::build_monochrome_text_payload(
        &verified_source.installer_payload,
        translations,
    )?;
    let baked = korean_patch::build_baked_graphics_payload(&monochrome.files, translations)?;
    let build = build_verified_payload(&verified_source, &baked.files, output_path)?;
    Ok(NarrativeGraphicsDevelopmentBuildReport {
        build,
        patch: NarrativeGraphicsPatchReport {
            monochrome: monochrome.report,
            baked: baked.report,
        },
    })
}

pub fn build_title_artwork_development_image(
    source_path: &Path,
    translations: &Path,
    manifest_path: &Path,
    source_preview_path: &Path,
    artwork_path: &Path,
    output_path: &Path,
) -> Result<NarrativeGraphicsDevelopmentBuildReport> {
    if output_path.exists() {
        bail!(
            "refusing to overwrite existing output: {}",
            output_path.display()
        );
    }

    let verified_source = load_verified_source(source_path)?;
    let monochrome = korean_patch::build_monochrome_text_payload(
        &verified_source.installer_payload,
        translations,
    )?;
    let baked = korean_patch::build_baked_graphics_payload_with_title_artwork(
        &monochrome.files,
        translations,
        manifest_path,
        source_preview_path,
        artwork_path,
    )?;
    let build = build_verified_payload(&verified_source, &baked.files, output_path)?;
    Ok(NarrativeGraphicsDevelopmentBuildReport {
        build,
        patch: NarrativeGraphicsPatchReport {
            monochrome: monochrome.report,
            baked: baked.report,
        },
    })
}

pub fn build_arle_character_asset_development_image(
    source_path: &Path,
    manifest_path: &Path,
    artwork_path: &Path,
    output_path: &Path,
) -> Result<ArleCharacterAssetDevelopmentBuildReport> {
    if output_path.exists() {
        bail!(
            "refusing to overwrite existing output: {}",
            output_path.display()
        );
    }

    let verified_source = load_verified_source(source_path)?;
    let patched = korean_patch::build_arle_character_asset_payload(
        &verified_source.installer_payload,
        manifest_path,
        artwork_path,
    )?;
    let build = build_verified_payload(&verified_source, &patched.files, output_path)?;
    Ok(ArleCharacterAssetDevelopmentBuildReport {
        build,
        patch: patched.report,
    })
}

fn build_verified_payload(
    verified_source: &VerifiedSource,
    installer_payload: &BTreeMap<String, Vec<u8>>,
    output_path: &Path,
) -> Result<BuildReport> {
    build_verified_files(
        verified_source,
        &verified_source.source_files.boot_files,
        &verified_source.source_files.runtime_files,
        installer_payload,
        output_path,
    )
}

fn build_verified_files(
    verified_source: &VerifiedSource,
    boot_files: &BTreeMap<String, Vec<u8>>,
    runtime_files: &BTreeMap<String, Vec<u8>>,
    installer_payload: &BTreeMap<String, Vec<u8>>,
    output_path: &Path,
) -> Result<BuildReport> {
    let candidate = standalone_image::assemble_standalone_image(
        &verified_source.image,
        &verified_source.source_files,
        boot_files,
        runtime_files,
        installer_payload,
    )?;
    let verified_file_count = standalone_image::verify_standalone_image(
        &verified_source.image,
        &candidate,
        boot_files,
        runtime_files,
        installer_payload,
    )?;
    let differences = reconstruction_diff::derive_reconstruction_diff(
        &verified_source.image,
        &candidate,
        0..source_disk::BOOT_SECTOR_SIZE,
    )?;
    let output =
        reconstruction_diff::apply_reconstruction_diff(&verified_source.image, &differences)?;
    if output != candidate {
        bail!("applied reconstruction diff differs from the verified candidate image");
    }
    let output_sha256 = source_disk::sha256_hex(&output);

    write_new_output(output_path, &output)?;

    Ok(BuildReport {
        source_sha256: verified_source.verification.sha256.clone(),
        output_sha256,
        output_size: output.len(),
        file_count: verified_file_count,
    })
}

pub fn survey_source_path(source_path: &Path) -> Result<SourceStructureReport> {
    let verified_source = load_verified_source(source_path)?;
    let mad_com = verified_source
        .installer_payload
        .get("MAD.COM")
        .context("verified installer payload is missing MAD.COM")?;
    let gaiji_com = verified_source
        .installer_payload
        .get("GAIJI.COM")
        .context("verified installer payload is missing GAIJI.COM")?;
    let menu_com = verified_source
        .installer_payload
        .get("MENU.COM")
        .context("verified installer payload is missing MENU.COM")?;

    Ok(SourceStructureReport {
        source_sha256: verified_source.verification.sha256,
        localization_assets: localization_assets::catalog_localization_assets(
            &verified_source.installer_payload,
            &verified_source.source_files.runtime_files,
            &verified_source.source_files.boot_files,
        )?,
        compile_lz: localization_assets::catalog_compile_lz_assets(
            mad_com,
            &verified_source.installer_payload,
        )?,
        asset_bindings: asset_bindings::catalog_asset_bindings(
            mad_com,
            &verified_source.installer_payload,
            &verified_source.source_files.runtime_files,
            &verified_source.source_files.boot_files,
        )?,
        game_data: game_data::catalog_game_data(mad_com, gaiji_com, menu_com)?,
        external_text: external_text::catalog_external_text(
            &verified_source.installer_payload,
            &verified_source.source_files.runtime_files,
            &verified_source.source_files.boot_files,
        )?,
    })
}

pub fn write_graphics_audit(
    source_path: &Path,
    output_directory: &Path,
) -> Result<GraphicsAuditReport> {
    let verified_source = load_verified_source(source_path)?;
    graphics_audit::write_graphics_audit(&verified_source.installer_payload, output_directory)
}

pub fn write_monochrome_text_development_audit(
    source_path: &Path,
    translations: &Path,
    output_directory: &Path,
) -> Result<GraphicsAuditReport> {
    let verified_source = load_verified_source(source_path)?;
    let patched = korean_patch::build_monochrome_text_payload(
        &verified_source.installer_payload,
        translations,
    )?;
    graphics_audit::write_monochrome_text_audit(&patched.files, output_directory)
}

pub fn write_fixed_gaiji_text_development_audit(
    source_path: &Path,
    translations: &Path,
    output_directory: &Path,
) -> Result<GraphicsAuditReport> {
    let verified_source = load_verified_source(source_path)?;
    let patched = korean_patch::build_fixed_gaiji_text_payload(
        &verified_source.installer_payload,
        translations,
    )?;
    graphics_audit::write_fixed_gaiji_text_audit(&patched.files, output_directory)
}

pub fn write_interface_text_development_audit(
    source_path: &Path,
    translations: &Path,
    output_directory: &Path,
) -> Result<GraphicsAuditReport> {
    let verified_source = load_verified_source(source_path)?;
    let patched = korean_patch::build_interface_text_payload(
        &verified_source.installer_payload,
        translations,
    )?;
    graphics_audit::write_interface_text_audit(&patched.report, output_directory)
}

pub fn write_dialogue_text_development_audit(
    source_path: &Path,
    translations: &Path,
    output_directory: &Path,
) -> Result<GraphicsAuditReport> {
    let verified_source = load_verified_source(source_path)?;
    let patched = korean_patch::build_dialogue_text_payload(
        &verified_source.installer_payload,
        translations,
    )?;
    graphics_audit::write_dialogue_text_audit(&patched.report, output_directory)
}

pub fn write_narrative_graphics_development_audit(
    source_path: &Path,
    translations: &Path,
    output_directory: &Path,
) -> Result<GraphicsAuditReport> {
    let verified_source = load_verified_source(source_path)?;
    let monochrome = korean_patch::build_monochrome_text_payload(
        &verified_source.installer_payload,
        translations,
    )?;
    let baked = korean_patch::build_baked_graphics_payload(&monochrome.files, translations)?;
    graphics_audit::write_narrative_graphics_audit(&baked.files, output_directory)
}

pub fn write_title_artwork_development_audit(
    source_path: &Path,
    translations: &Path,
    manifest_path: &Path,
    source_preview_path: &Path,
    artwork_path: &Path,
    output_directory: &Path,
) -> Result<GraphicsAuditReport> {
    let verified_source = load_verified_source(source_path)?;
    let monochrome = korean_patch::build_monochrome_text_payload(
        &verified_source.installer_payload,
        translations,
    )?;
    let baked = korean_patch::build_baked_graphics_payload_with_title_artwork(
        &monochrome.files,
        translations,
        manifest_path,
        source_preview_path,
        artwork_path,
    )?;
    graphics_audit::write_title_artwork_audit(&baked.files, output_directory)
}

pub fn write_arle_character_asset_development_audit(
    source_path: &Path,
    manifest_path: &Path,
    artwork_path: &Path,
    output_directory: &Path,
) -> Result<GraphicsAuditReport> {
    let verified_source = load_verified_source(source_path)?;
    let patched = korean_patch::build_arle_character_asset_payload(
        &verified_source.installer_payload,
        manifest_path,
        artwork_path,
    )?;
    graphics_audit::write_arle_character_asset_audit(
        &verified_source.installer_payload,
        &patched.files,
        output_directory,
    )
}

pub fn write_localization_glyph_comparison_audit(
    source_path: &Path,
    translations: &Path,
    pc98_font_bmp: &Path,
    manifest_path: &Path,
    source_preview_path: &Path,
    artwork_path: &Path,
    output_directory: &Path,
) -> Result<GraphicsAuditReport> {
    let verified_source = load_verified_source(source_path)?;
    let menu =
        korean_patch::build_menu_text_files(&verified_source.installer_payload, translations)?;
    let interface = korean_patch::build_interface_text_payload(
        &verified_source.installer_payload,
        translations,
    )?;
    let dialogue = korean_patch::build_dialogue_text_payload(
        &verified_source.installer_payload,
        translations,
    )?;
    let fixed = korean_patch::build_fixed_gaiji_text_payload(
        &verified_source.installer_payload,
        translations,
    )?;
    let monochrome = korean_patch::build_monochrome_text_payload(
        &verified_source.installer_payload,
        translations,
    )?;
    let baked = korean_patch::build_baked_graphics_payload_with_title_artwork(
        &monochrome.files,
        translations,
        manifest_path,
        source_preview_path,
        artwork_path,
    )?;
    graphics_audit::write_localization_comparison_audit(
        graphics_audit::LocalizationComparisonInputs {
            source: &verified_source.installer_payload,
            pc98_font_bmp,
            menu_files: &menu.files,
            menu_report: &menu.report,
            interface_files: &interface.files,
            interface_report: &interface.report,
            dialogue_files: &dialogue.files,
            dialogue_report: &dialogue.report,
            fixed_files: &fixed.files,
            monochrome_files: &monochrome.files,
            baked_files: &baked.files,
        },
        output_directory,
    )
}

pub fn write_audio_audit(source_path: &Path, output_directory: &Path) -> Result<AudioAuditReport> {
    let verified_source = load_verified_source(source_path)?;
    audio_audit::write_audio_audit(&verified_source.installer_payload, output_directory)
}

pub fn write_translation_workspace(
    source_path: &Path,
    output_directory: &Path,
) -> Result<TranslationWorkspaceReport> {
    let source = survey_source_path(source_path)?;
    translation_workspace::write_translation_workspace(&source, output_directory)
}

pub fn publish_translation_drafts(
    protected_workspace: &Path,
    proposal_directory: &Path,
    output_directory: &Path,
) -> Result<TranslationDraftReport> {
    translation_drafts::publish_translation_drafts(
        protected_workspace,
        proposal_directory,
        output_directory,
    )
}

pub fn write_in_game_retro_patcher_plan(
    source_path: &Path,
    content_image_path: &Path,
    output_path: &Path,
) -> Result<RetroPatcherPlanReport> {
    let verified_source = load_verified_source(source_path)?;
    retro_patcher_plan::write_in_game_plan(
        &verified_source.verification,
        &verified_source.image,
        &verified_source.source_files,
        &verified_source.installer_payload,
        content_image_path,
        output_path,
    )
}

pub fn verify_in_game_retro_patcher_result(
    source_path: &Path,
    content_image_path: &Path,
    candidate_image_path: &Path,
) -> Result<RetroPatcherResultVerificationReport> {
    let verified_source = load_verified_source(source_path)?;
    retro_patcher_plan::verify_applied_result(
        &verified_source.image,
        content_image_path,
        candidate_image_path,
    )
}

fn load_verified_source(source_path: &Path) -> Result<VerifiedSource> {
    let image = fs::read(source_path)
        .with_context(|| format!("failed to read source disk: {}", source_path.display()))?;
    let verification = source_disk::verify_source_bytes(&image)?;
    let source_files = source_disk::read_source_files(&image)?;
    let installer_payload = lha_sfx::extract_installer_payload(&source_files.installer)?;

    Ok(VerifiedSource {
        image,
        verification,
        source_files,
        installer_payload,
    })
}

fn write_new_output(output_path: &Path, bytes: &[u8]) -> Result<()> {
    let parent = output_path
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent)
        .with_context(|| format!("failed to create output directory: {}", parent.display()))?;

    let mut temporary = NamedTempFile::new_in(parent)
        .with_context(|| format!("failed to create temporary output in {}", parent.display()))?;
    temporary
        .write_all(bytes)
        .context("failed to write temporary output image")?;
    temporary
        .as_file()
        .sync_all()
        .context("failed to flush temporary output image")?;
    temporary.persist_noclobber(output_path).map_err(|error| {
        anyhow::anyhow!(
            "failed to publish output without overwriting {}: {}",
            output_path.display(),
            error.error
        )
    })?;
    Ok(())
}

pub fn default_output_path() -> PathBuf {
    PathBuf::from("out/Disc Station Vol. 05 (Bayoen Wars - Daimadou Senryaku Monogatari).hdm")
}

pub fn default_hangul_visibility_output_path() -> PathBuf {
    PathBuf::from("out/Disc Station Vol. 05 (Bayoen Wars - Hangul Visibility Development).hdm")
}

pub fn default_monochrome_text_development_output_path() -> PathBuf {
    PathBuf::from("out/Disc Station Vol. 05 (Bayoen Wars - Korean Monochrome Development).hdm")
}

pub fn default_fixed_gaiji_text_development_output_path() -> PathBuf {
    PathBuf::from("out/Disc Station Vol. 05 (Bayoen Wars - Korean Fixed Text Development).hdm")
}

pub fn default_interface_text_development_output_path() -> PathBuf {
    PathBuf::from("out/Disc Station Vol. 05 (Bayoen Wars - Korean Interface Development).hdm")
}

pub fn default_mad_system_text_development_output_path() -> PathBuf {
    PathBuf::from("out/Disc Station Vol. 05 (Bayoen Wars - Korean System Text Development).hdm")
}

pub fn default_mad_system_interface_text_development_output_path() -> PathBuf {
    PathBuf::from(
        "out/Disc Station Vol. 05 (Bayoen Wars - Korean System and Interface Development).hdm",
    )
}

pub fn default_dialogue_text_development_output_path() -> PathBuf {
    PathBuf::from("out/Disc Station Vol. 05 (Bayoen Wars - Korean Dialogue Development).hdm")
}

pub fn default_mad_scene_text_development_output_path() -> PathBuf {
    PathBuf::from("out/Disc Station Vol. 05 (Bayoen Wars - Korean Scene Text Development).hdm")
}

pub fn default_mad_scene_narrative_development_output_path() -> PathBuf {
    PathBuf::from("out/Disc Station Vol. 05 (Bayoen Wars - Korean Scene Narrative Development).hdm")
}

pub fn default_integrated_localization_development_output_path() -> PathBuf {
    PathBuf::from("out/Disc Station Vol. 05 (Bayoen Wars - Integrated Korean Development).hdm")
}

pub fn default_shell_text_development_output_path() -> PathBuf {
    PathBuf::from("out/Disc Station Vol. 05 (Bayoen Wars - Korean Shell Development).hdm")
}

pub fn default_system_loader_text_development_output_path() -> PathBuf {
    PathBuf::from("out/Disc Station Vol. 05 (Bayoen Wars - Korean System Loader Development).hdm")
}

pub fn default_sampling_driver_text_development_output_path() -> PathBuf {
    PathBuf::from("out/Disc Station Vol. 05 (Bayoen Wars - Korean Sampling Driver Development).hdm")
}

pub fn default_mouse_driver_text_development_output_path() -> PathBuf {
    PathBuf::from("out/Disc Station Vol. 05 (Bayoen Wars - Korean Mouse Driver Development).hdm")
}

pub fn default_playback_driver_text_development_output_path() -> PathBuf {
    PathBuf::from(
        "out/Disc Station Vol. 05 (Bayoen Wars - Korean Playback Drivers Development).hdm",
    )
}

pub fn default_menu_text_development_output_path() -> PathBuf {
    PathBuf::from("out/Disc Station Vol. 05 (Bayoen Wars - Korean Menu Development).hdm")
}

pub fn default_narrative_graphics_development_output_path() -> PathBuf {
    PathBuf::from("out/Disc Station Vol. 05 (Bayoen Wars - Korean Graphics Development).hdm")
}

pub fn default_title_artwork_development_output_path() -> PathBuf {
    PathBuf::from("out/Disc Station Vol. 05 (Bayoen Wars - Korean Title Artwork Development).hdm")
}

pub fn default_arle_character_asset_development_output_path() -> PathBuf {
    PathBuf::from("out/Disc Station Vol. 05 (Bayoen Wars - Arle Character Development).hdm")
}
