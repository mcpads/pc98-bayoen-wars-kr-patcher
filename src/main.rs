use std::path::PathBuf;

use anyhow::Result;
use bayoen_wars_builder::{
    ArleCharacterAssetSelection, IntegratedLocalizationDevelopmentInputs, audit_translation_glyphs,
    audit_translation_layout, build_arle_character_asset_development_image,
    build_dialogue_text_development_image, build_fixed_gaiji_text_development_image,
    build_hangul_visibility_image, build_integrated_localization_development_image,
    build_interface_text_development_image, build_mad_scene_narrative_development_image,
    build_mad_scene_narrative_with_title_artwork_development_image,
    build_mad_scene_text_development_image, build_mad_system_interface_text_development_image,
    build_mad_system_text_development_image, build_menu_text_development_image,
    build_monochrome_text_development_image, build_mouse_driver_text_development_image,
    build_narrative_graphics_development_image, build_playback_driver_text_development_image,
    build_sampling_driver_text_development_image, build_shell_text_development_image,
    build_standalone_image, build_system_loader_text_development_image,
    build_title_artwork_development_image, default_arle_character_asset_development_output_path,
    default_dialogue_text_development_output_path,
    default_fixed_gaiji_text_development_output_path, default_hangul_visibility_output_path,
    default_integrated_localization_development_output_path,
    default_interface_text_development_output_path,
    default_mad_scene_narrative_development_output_path,
    default_mad_scene_text_development_output_path,
    default_mad_system_interface_text_development_output_path,
    default_mad_system_text_development_output_path, default_menu_text_development_output_path,
    default_monochrome_text_development_output_path,
    default_mouse_driver_text_development_output_path,
    default_narrative_graphics_development_output_path, default_output_path,
    default_playback_driver_text_development_output_path,
    default_sampling_driver_text_development_output_path,
    default_shell_text_development_output_path, default_system_loader_text_development_output_path,
    default_title_artwork_development_output_path, publish_translation_drafts, survey_source_path,
    verify_in_game_retro_patcher_result, verify_source_path,
    write_arle_character_asset_development_audit, write_audio_audit,
    write_dialogue_text_development_audit, write_fixed_gaiji_text_development_audit,
    write_graphics_audit, write_in_game_retro_patcher_plan, write_interface_text_development_audit,
    write_localization_glyph_comparison_audit, write_monochrome_text_development_audit,
    write_narrative_graphics_development_audit, write_title_artwork_development_audit,
    write_translation_workspace,
};
use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(
    name = "bayoen-wars-builder",
    about = "Build Bayoen Wars: Daimadou Senryaku Monogatari from Disc Station Vol. 05 Disk 1"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Verify that a disk is the supported Disc Station Vol. 05 Disk 1 image.
    VerifySource {
        #[arg(long, value_name = "DISK1.HDM")]
        source: PathBuf,
    },
    /// Verify the source and report the consumer-linked text and GAIJI structure as JSON.
    SurveySource {
        #[arg(long, value_name = "DISK1.HDM")]
        source: PathBuf,
    },
    /// Render consumer-proven graphics layouts as diagnostic PNG previews.
    GraphicsAudit {
        #[arg(long, value_name = "DISK1.HDM")]
        source: PathBuf,
        #[arg(long, value_name = "DIRECTORY")]
        output_directory: PathBuf,
    },
    /// Decode every consumer-linked 4-bit SAMPA stream as a diagnostic WAV.
    AudioAudit {
        #[arg(long, value_name = "DISK1.HDM")]
        source: PathBuf,
        #[arg(long, value_name = "DIRECTORY")]
        output_directory: PathBuf,
    },
    /// Extract protected source text and target-file coverage into a local workspace.
    ExtractTranslationWorkspace {
        #[arg(long, value_name = "DISK1.HDM")]
        source: PathBuf,
        #[arg(long, value_name = "DIRECTORY", default_value = "work/translations")]
        output_directory: PathBuf,
    },
    /// Publish independently reviewed, source-free Korean drafts bound to a protected workspace.
    PublishTranslationDrafts {
        #[arg(long, value_name = "DIRECTORY")]
        protected_workspace: PathBuf,
        #[arg(long, value_name = "DIRECTORY")]
        proposal_directory: PathBuf,
        #[arg(long, value_name = "DIRECTORY", default_value = "assets/translations")]
        output_directory: PathBuf,
    },
    /// Audit glyph demand from the source-free needs_human_review translation corpus.
    AuditTranslationGlyphs {
        #[arg(long, value_name = "DIRECTORY", default_value = "assets/translations")]
        translations: PathBuf,
    },
    /// Audit reviewed text against protected source layout and storage evidence.
    AuditTranslationLayout {
        #[arg(long, value_name = "DIRECTORY")]
        protected_workspace: PathBuf,
        #[arg(long, value_name = "DIRECTORY", default_value = "assets/translations")]
        translations: PathBuf,
    },
    /// Render reviewed opening and ending development pages through the source consumer format.
    AuditMonochromeTextDevelopment {
        #[arg(long, value_name = "DISK1.HDM")]
        source: PathBuf,
        #[arg(long, value_name = "DIRECTORY", default_value = "assets/translations")]
        translations: PathBuf,
        #[arg(long, value_name = "DIRECTORY")]
        output_directory: PathBuf,
    },
    /// Render all reviewed narrative graphics through the source consumer formats.
    AuditNarrativeGraphicsDevelopment {
        #[arg(long, value_name = "DISK1.HDM")]
        source: PathBuf,
        #[arg(long, value_name = "DIRECTORY", default_value = "assets/translations")]
        translations: PathBuf,
        #[arg(long, value_name = "DIRECTORY")]
        output_directory: PathBuf,
    },
    /// Render a source-derived generated title master through the TITLE.DAT consumer format.
    AuditTitleArtworkDevelopment {
        #[arg(long, value_name = "DISK1.HDM")]
        source: PathBuf,
        #[arg(long, value_name = "DIRECTORY", default_value = "assets/translations")]
        translations: PathBuf,
        #[arg(
            long,
            value_name = "MANIFEST.JSON",
            default_value = "assets/title_art/development-title-art.json"
        )]
        manifest: PathBuf,
        #[arg(long, value_name = "SOURCE-TITLE.PNG")]
        source_preview: PathBuf,
        #[arg(
            long,
            value_name = "PROJECT-TITLE.PNG",
            default_value = "assets/title_art/title-authored-frame.png"
        )]
        artwork: PathBuf,
        #[arg(long, value_name = "DIRECTORY")]
        output_directory: PathBuf,
    },
    /// Compare source and converted Arle C07/C08 frames through the verified consumer format.
    AuditArleCharacterAssetDevelopment {
        #[arg(long, value_name = "DISK1.HDM")]
        source: PathBuf,
        #[arg(
            long,
            value_name = "MANIFEST.JSON",
            default_value = "assets/characters/arle/battle-sprites.json"
        )]
        manifest: PathBuf,
        #[arg(
            long,
            value_name = "ARLE-SHEET.PNG",
            default_value = "assets/characters/arle/battle-sprites.pc98.png"
        )]
        artwork: PathBuf,
        #[arg(long, value_name = "DIRECTORY")]
        output_directory: PathBuf,
    },
    /// Compare representative original and current glyphs through their actual consumer formats.
    AuditLocalizationGlyphComparison {
        #[arg(long, value_name = "DISK1.HDM")]
        source: PathBuf,
        #[arg(long, value_name = "DIRECTORY", default_value = "assets/translations")]
        translations: PathBuf,
        #[arg(long, value_name = "FONT.BMP")]
        pc98_font_bmp: PathBuf,
        #[arg(
            long,
            value_name = "MANIFEST.JSON",
            default_value = "assets/title_art/development-title-art.json"
        )]
        manifest: PathBuf,
        #[arg(long, value_name = "SOURCE-TITLE.PNG")]
        source_preview: PathBuf,
        #[arg(
            long,
            value_name = "PROJECT-TITLE.PNG",
            default_value = "assets/title_art/title-authored-frame.png"
        )]
        artwork: PathBuf,
        #[arg(long, value_name = "DIRECTORY")]
        output_directory: PathBuf,
    },
    /// Render the ten reviewed fixed 6x7 GAIJI text drafts from a patched payload.
    AuditFixedGaijiTextDevelopment {
        #[arg(long, value_name = "DISK1.HDM")]
        source: PathBuf,
        #[arg(long, value_name = "DIRECTORY", default_value = "assets/translations")]
        translations: PathBuf,
        #[arg(long, value_name = "DIRECTORY")]
        output_directory: PathBuf,
    },
    /// Render all 88 reviewed interface records after relocation and readback.
    AuditInterfaceTextDevelopment {
        #[arg(long, value_name = "DISK1.HDM")]
        source: PathBuf,
        #[arg(long, value_name = "DIRECTORY", default_value = "assets/translations")]
        translations: PathBuf,
        #[arg(long, value_name = "DIRECTORY")]
        output_directory: PathBuf,
    },
    /// Render all 55 reviewed dialogue records after relocation and readback.
    AuditDialogueTextDevelopment {
        #[arg(long, value_name = "DISK1.HDM")]
        source: PathBuf,
        #[arg(long, value_name = "DIRECTORY", default_value = "assets/translations")]
        translations: PathBuf,
        #[arg(long, value_name = "DIRECTORY")]
        output_directory: PathBuf,
    },
    /// Build a standalone game disk without modifying the supplied source image.
    Build {
        #[arg(long, value_name = "DISK1.HDM")]
        source: PathBuf,
        #[arg(long, value_name = "OUTPUT.HDM", default_value_os_t = default_output_path())]
        output: PathBuf,
    },
    /// Build a development disk that routes interface and dialogue codes through a Hangul GAIJI.
    BuildHangulVisibility {
        #[arg(long, value_name = "DISK1.HDM")]
        source: PathBuf,
        #[arg(
            long,
            value_name = "OUTPUT.HDM",
            default_value_os_t = default_hangul_visibility_output_path()
        )]
        output: PathBuf,
    },
    /// Build a development disk with reviewed opening and ending monochrome text drafts.
    BuildMonochromeTextDevelopment {
        #[arg(long, value_name = "DISK1.HDM")]
        source: PathBuf,
        #[arg(long, value_name = "DIRECTORY", default_value = "assets/translations")]
        translations: PathBuf,
        #[arg(
            long,
            value_name = "OUTPUT.HDM",
            default_value_os_t = default_monochrome_text_development_output_path()
        )]
        output: PathBuf,
    },
    /// Build a development disk with the ten reviewed fixed 6x7 GAIJI text drafts.
    BuildFixedGaijiTextDevelopment {
        #[arg(long, value_name = "DISK1.HDM")]
        source: PathBuf,
        #[arg(long, value_name = "DIRECTORY", default_value = "assets/translations")]
        translations: PathBuf,
        #[arg(
            long,
            value_name = "OUTPUT.HDM",
            default_value_os_t = default_fixed_gaiji_text_development_output_path()
        )]
        output: PathBuf,
    },
    /// Build a development disk with all 88 reviewed interface text drafts.
    BuildInterfaceTextDevelopment {
        #[arg(long, value_name = "DISK1.HDM")]
        source: PathBuf,
        #[arg(long, value_name = "DIRECTORY", default_value = "assets/translations")]
        translations: PathBuf,
        #[arg(
            long,
            value_name = "OUTPUT.HDM",
            default_value_os_t = default_interface_text_development_output_path()
        )]
        output: PathBuf,
    },
    /// Build a development disk with all ten reviewed MAD DOS system-text drafts.
    BuildMadSystemTextDevelopment {
        #[arg(long, value_name = "DISK1.HDM")]
        source: PathBuf,
        #[arg(long, value_name = "DIRECTORY", default_value = "assets/translations")]
        translations: PathBuf,
        #[arg(
            long,
            value_name = "OUTPUT.HDM",
            default_value_os_t = default_mad_system_text_development_output_path()
        )]
        output: PathBuf,
    },
    /// Build one development disk with MAD system and interface text sharing one GAIJI bank.
    BuildMadSystemInterfaceTextDevelopment {
        #[arg(long, value_name = "DISK1.HDM")]
        source: PathBuf,
        #[arg(long, value_name = "DIRECTORY", default_value = "assets/translations")]
        translations: PathBuf,
        #[arg(
            long,
            value_name = "OUTPUT.HDM",
            default_value_os_t = default_mad_system_interface_text_development_output_path()
        )]
        output: PathBuf,
    },
    /// Build an isolated dialogue disk only when the source 170-slot bank is sufficient.
    BuildDialogueTextDevelopment {
        #[arg(long, value_name = "DISK1.HDM")]
        source: PathBuf,
        #[arg(long, value_name = "DIRECTORY", default_value = "assets/translations")]
        translations: PathBuf,
        #[arg(
            long,
            value_name = "OUTPUT.HDM",
            default_value_os_t = default_dialogue_text_development_output_path()
        )]
        output: PathBuf,
    },
    /// Build one development disk that switches GAIJI banks across MAD scene transitions.
    BuildMadSceneTextDevelopment {
        #[arg(long, value_name = "DISK1.HDM")]
        source: PathBuf,
        #[arg(long, value_name = "DIRECTORY", default_value = "assets/translations")]
        translations: PathBuf,
        #[arg(
            long,
            value_name = "OUTPUT.HDM",
            default_value_os_t = default_mad_scene_text_development_output_path()
        )]
        output: PathBuf,
    },
    /// Build scene-banked MAD text plus opening, ending, title, and selection graphics.
    BuildMadSceneNarrativeDevelopment {
        #[arg(long, value_name = "DISK1.HDM")]
        source: PathBuf,
        #[arg(long, value_name = "DIRECTORY", default_value = "assets/translations")]
        translations: PathBuf,
        #[arg(
            long,
            value_name = "MANIFEST.JSON",
            default_value = "assets/title_art/development-title-art.json"
        )]
        title_artwork_manifest: PathBuf,
        /// Use the project-authored title while keeping non-game DOS programs unchanged.
        #[arg(long, value_name = "SOURCE-TITLE.PNG")]
        title_source_preview: Option<PathBuf>,
        #[arg(
            long,
            value_name = "PROJECT-TITLE.PNG",
            default_value = "assets/title_art/title-authored-frame.png"
        )]
        title_artwork: PathBuf,
        #[arg(
            long,
            value_name = "OUTPUT.HDM",
            default_value_os_t = default_mad_scene_narrative_development_output_path()
        )]
        output: PathBuf,
    },
    /// Write a Retro Patcher author plan for an exact game-only development content HDM.
    WriteInGameRetroPatcherPlan {
        #[arg(long, value_name = "DISK1.HDM")]
        source: PathBuf,
        #[arg(long, value_name = "CONTENT.HDM")]
        content: PathBuf,
        #[arg(long, value_name = "PLAN.JSON")]
        output: PathBuf,
    },
    /// Verify an applied Retro Patcher HDM against the game-only logical content build.
    VerifyInGameRetroPatcherResult {
        #[arg(long, value_name = "DISK1.HDM")]
        source: PathBuf,
        #[arg(long, value_name = "CONTENT.HDM")]
        content: PathBuf,
        #[arg(long, value_name = "CANDIDATE.HDM")]
        candidate: PathBuf,
    },
    /// Build one development disk containing every adopted localization producer.
    BuildIntegratedLocalizationDevelopment {
        #[arg(long, value_name = "DISK1.HDM")]
        source: PathBuf,
        #[arg(long, value_name = "DIRECTORY", default_value = "assets/translations")]
        translations: PathBuf,
        #[arg(
            long,
            value_name = "MANIFEST.JSON",
            default_value = "assets/title_art/development-title-art.json"
        )]
        title_artwork_manifest: PathBuf,
        #[arg(long, value_name = "SOURCE-TITLE.PNG")]
        title_source_preview: PathBuf,
        #[arg(
            long,
            value_name = "PROJECT-TITLE.PNG",
            default_value = "assets/title_art/title-authored-frame.png"
        )]
        title_artwork: PathBuf,
        /// Use a complete custom Arle source-asset set instead of the tracked set.
        #[arg(
            long,
            value_name = "DIRECTORY",
            conflicts_with = "preserve_original_arle"
        )]
        arle_assets: Option<PathBuf>,
        /// Leave the original C07/C08 graphics unchanged while applying all translations.
        #[arg(long, conflicts_with = "arle_assets")]
        preserve_original_arle: bool,
        #[arg(
            long,
            value_name = "OUTPUT.HDM",
            default_value_os_t = default_integrated_localization_development_output_path()
        )]
        output: PathBuf,
    },
    /// Build a development disk with all eight reviewed DSH boot-shell text drafts.
    BuildShellTextDevelopment {
        #[arg(long, value_name = "DISK1.HDM")]
        source: PathBuf,
        #[arg(long, value_name = "DIRECTORY", default_value = "assets/translations")]
        translations: PathBuf,
        #[arg(
            long,
            value_name = "OUTPUT.HDM",
            default_value_os_t = default_shell_text_development_output_path()
        )]
        output: PathBuf,
    },
    /// Build a development disk with all seven reviewed MEGDOS system-loader text drafts.
    BuildSystemLoaderTextDevelopment {
        #[arg(long, value_name = "DISK1.HDM")]
        source: PathBuf,
        #[arg(long, value_name = "DIRECTORY", default_value = "assets/translations")]
        translations: PathBuf,
        #[arg(
            long,
            value_name = "OUTPUT.HDM",
            default_value_os_t = default_system_loader_text_development_output_path()
        )]
        output: PathBuf,
    },
    /// Build a development disk with all three reviewed BSAMP sampling-driver text drafts.
    BuildSamplingDriverTextDevelopment {
        #[arg(long, value_name = "DISK1.HDM")]
        source: PathBuf,
        #[arg(long, value_name = "DIRECTORY", default_value = "assets/translations")]
        translations: PathBuf,
        #[arg(
            long,
            value_name = "OUTPUT.HDM",
            default_value_os_t = default_sampling_driver_text_development_output_path()
        )]
        output: PathBuf,
    },
    /// Build a development disk with all fifteen reviewed NMOUSE mouse-driver text drafts.
    BuildMouseDriverTextDevelopment {
        #[arg(long, value_name = "DISK1.HDM")]
        source: PathBuf,
        #[arg(long, value_name = "DIRECTORY", default_value = "assets/translations")]
        translations: PathBuf,
        #[arg(
            long,
            value_name = "OUTPUT.HDM",
            default_value_os_t = default_mouse_driver_text_development_output_path()
        )]
        output: PathBuf,
    },
    /// Build a development disk with independent BPLAY6 and FPLAY6 Korean text banks.
    BuildPlaybackDriverTextDevelopment {
        #[arg(long, value_name = "DISK1.HDM")]
        source: PathBuf,
        #[arg(long, value_name = "DIRECTORY", default_value = "assets/translations")]
        translations: PathBuf,
        #[arg(
            long,
            value_name = "OUTPUT.HDM",
            default_value_os_t = default_playback_driver_text_development_output_path()
        )]
        output: PathBuf,
    },
    /// Build a development disk with all 47 reviewed MENU text drafts.
    BuildMenuTextDevelopment {
        #[arg(long, value_name = "DISK1.HDM")]
        source: PathBuf,
        #[arg(long, value_name = "DIRECTORY", default_value = "assets/translations")]
        translations: PathBuf,
        #[arg(
            long,
            value_name = "OUTPUT.HDM",
            default_value_os_t = default_menu_text_development_output_path()
        )]
        output: PathBuf,
    },
    /// Build a development disk with reviewed monochrome and baked graphics drafts.
    BuildNarrativeGraphicsDevelopment {
        #[arg(long, value_name = "DISK1.HDM")]
        source: PathBuf,
        #[arg(long, value_name = "DIRECTORY", default_value = "assets/translations")]
        translations: PathBuf,
        #[arg(
            long,
            value_name = "OUTPUT.HDM",
            default_value_os_t = default_narrative_graphics_development_output_path()
        )]
        output: PathBuf,
    },
    /// Build a development disk with a source-derived generated Korean title master.
    BuildTitleArtworkDevelopment {
        #[arg(long, value_name = "DISK1.HDM")]
        source: PathBuf,
        #[arg(long, value_name = "DIRECTORY", default_value = "assets/translations")]
        translations: PathBuf,
        #[arg(
            long,
            value_name = "MANIFEST.JSON",
            default_value = "assets/title_art/development-title-art.json"
        )]
        manifest: PathBuf,
        #[arg(long, value_name = "SOURCE-TITLE.PNG")]
        source_preview: PathBuf,
        #[arg(
            long,
            value_name = "PROJECT-TITLE.PNG",
            default_value = "assets/title_art/title-authored-frame.png"
        )]
        artwork: PathBuf,
        #[arg(
            long,
            value_name = "OUTPUT.HDM",
            default_value_os_t = default_title_artwork_development_output_path()
        )]
        output: PathBuf,
    },
    /// Build a development disk with the project-authored Arle C07/C08 frame set.
    BuildArleCharacterAssetDevelopment {
        #[arg(long, value_name = "DISK1.HDM")]
        source: PathBuf,
        #[arg(
            long,
            value_name = "MANIFEST.JSON",
            default_value = "assets/characters/arle/battle-sprites.json"
        )]
        manifest: PathBuf,
        #[arg(
            long,
            value_name = "ARLE-SHEET.PNG",
            default_value = "assets/characters/arle/battle-sprites.pc98.png"
        )]
        artwork: PathBuf,
        #[arg(
            long,
            value_name = "OUTPUT.HDM",
            default_value_os_t = default_arle_character_asset_development_output_path()
        )]
        output: PathBuf,
    },
}

fn select_arle_character_assets(
    directory: Option<PathBuf>,
    preserve_original: bool,
) -> ArleCharacterAssetSelection {
    if preserve_original {
        ArleCharacterAssetSelection::preserve_original()
    } else {
        directory
            .map(ArleCharacterAssetSelection::from_directory)
            .unwrap_or_else(ArleCharacterAssetSelection::tracked)
    }
}

fn main() -> Result<()> {
    match Cli::parse().command {
        Command::VerifySource { source } => {
            let verification = verify_source_path(&source)?;
            println!("supported source: {}", source.display());
            println!("size: {} bytes", verification.size);
            println!("sha256: {}", verification.sha256);
        }
        Command::SurveySource { source } => {
            let report = survey_source_path(&source)?;
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
        Command::GraphicsAudit {
            source,
            output_directory,
        } => {
            let report = write_graphics_audit(&source, &output_directory)?;
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
        Command::AudioAudit {
            source,
            output_directory,
        } => {
            let report = write_audio_audit(&source, &output_directory)?;
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
        Command::ExtractTranslationWorkspace {
            source,
            output_directory,
        } => {
            let report = write_translation_workspace(&source, &output_directory)?;
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
        Command::PublishTranslationDrafts {
            protected_workspace,
            proposal_directory,
            output_directory,
        } => {
            let report = publish_translation_drafts(
                &protected_workspace,
                &proposal_directory,
                &output_directory,
            )?;
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
        Command::AuditTranslationGlyphs { translations } => {
            let report = audit_translation_glyphs(&translations)?;
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
        Command::AuditTranslationLayout {
            protected_workspace,
            translations,
        } => {
            let report = audit_translation_layout(&protected_workspace, &translations)?;
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
        Command::AuditMonochromeTextDevelopment {
            source,
            translations,
            output_directory,
        } => {
            let report =
                write_monochrome_text_development_audit(&source, &translations, &output_directory)?;
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
        Command::AuditNarrativeGraphicsDevelopment {
            source,
            translations,
            output_directory,
        } => {
            let report = write_narrative_graphics_development_audit(
                &source,
                &translations,
                &output_directory,
            )?;
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
        Command::AuditTitleArtworkDevelopment {
            source,
            translations,
            manifest,
            source_preview,
            artwork,
            output_directory,
        } => {
            let report = write_title_artwork_development_audit(
                &source,
                &translations,
                &manifest,
                &source_preview,
                &artwork,
                &output_directory,
            )?;
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
        Command::AuditArleCharacterAssetDevelopment {
            source,
            manifest,
            artwork,
            output_directory,
        } => {
            let report = write_arle_character_asset_development_audit(
                &source,
                &manifest,
                &artwork,
                &output_directory,
            )?;
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
        Command::AuditLocalizationGlyphComparison {
            source,
            translations,
            pc98_font_bmp,
            manifest,
            source_preview,
            artwork,
            output_directory,
        } => {
            let report = write_localization_glyph_comparison_audit(
                &source,
                &translations,
                &pc98_font_bmp,
                &manifest,
                &source_preview,
                &artwork,
                &output_directory,
            )?;
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
        Command::AuditFixedGaijiTextDevelopment {
            source,
            translations,
            output_directory,
        } => {
            let report = write_fixed_gaiji_text_development_audit(
                &source,
                &translations,
                &output_directory,
            )?;
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
        Command::AuditInterfaceTextDevelopment {
            source,
            translations,
            output_directory,
        } => {
            let report =
                write_interface_text_development_audit(&source, &translations, &output_directory)?;
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
        Command::AuditDialogueTextDevelopment {
            source,
            translations,
            output_directory,
        } => {
            let report =
                write_dialogue_text_development_audit(&source, &translations, &output_directory)?;
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
        Command::Build { source, output } => {
            let report = build_standalone_image(&source, &output)?;
            println!("built: {}", output.display());
            println!("source sha256: {}", report.source_sha256);
            println!("output sha256: {}", report.output_sha256);
            println!("output size: {} bytes", report.output_size);
            println!("verified files: {}", report.file_count);
        }
        Command::BuildHangulVisibility { source, output } => {
            let report = build_hangul_visibility_image(&source, &output)?;
            println!("built: {}", output.display());
            println!("source sha256: {}", report.build.source_sha256);
            println!("output sha256: {}", report.build.output_sha256);
            println!("output size: {} bytes", report.build.output_size);
            println!("verified files: {}", report.build.file_count);
            println!(
                "Hangul route: {} -> {} (slot {})",
                report.patch.character, report.patch.shift_jis_code, report.patch.gaiji_slot
            );
            println!("Expected Writes: {}", report.patch.writes.len());
        }
        Command::BuildMonochromeTextDevelopment {
            source,
            translations,
            output,
        } => {
            let report = build_monochrome_text_development_image(&source, &translations, &output)?;
            println!("built: {}", output.display());
            println!("source sha256: {}", report.build.source_sha256);
            println!("output sha256: {}", report.build.output_sha256);
            println!("output size: {} bytes", report.build.output_size);
            println!("verified files: {}", report.build.file_count);
            println!("build status: development_only");
            println!("translation status: {}", report.patch.translation_status);
            println!("Expected Writes: {}", report.patch.writes.len());
            println!("{}", serde_json::to_string_pretty(&report.patch)?);
        }
        Command::BuildFixedGaijiTextDevelopment {
            source,
            translations,
            output,
        } => {
            let report = build_fixed_gaiji_text_development_image(&source, &translations, &output)?;
            println!("built: {}", output.display());
            println!("source sha256: {}", report.build.source_sha256);
            println!("output sha256: {}", report.build.output_sha256);
            println!("output size: {} bytes", report.build.output_size);
            println!("verified files: {}", report.build.file_count);
            println!("build status: development_only");
            println!("translation status: {}", report.patch.translation_status);
            println!("Expected Writes: {}", report.patch.writes.len());
            println!("{}", serde_json::to_string_pretty(&report.patch)?);
        }
        Command::BuildInterfaceTextDevelopment {
            source,
            translations,
            output,
        } => {
            let report = build_interface_text_development_image(&source, &translations, &output)?;
            println!("built: {}", output.display());
            println!("source sha256: {}", report.build.source_sha256);
            println!("output sha256: {}", report.build.output_sha256);
            println!("output size: {} bytes", report.build.output_size);
            println!("verified files: {}", report.build.file_count);
            println!("build status: development_only");
            println!("translation status: {}", report.patch.translation_status);
            println!("Expected Writes: {}", report.patch.writes.len());
            println!("{}", serde_json::to_string_pretty(&report.patch)?);
        }
        Command::BuildMadSystemTextDevelopment {
            source,
            translations,
            output,
        } => {
            let report = build_mad_system_text_development_image(&source, &translations, &output)?;
            println!("built: {}", output.display());
            println!("source sha256: {}", report.build.source_sha256);
            println!("output sha256: {}", report.build.output_sha256);
            println!("output size: {} bytes", report.build.output_size);
            println!("verified files: {}", report.build.file_count);
            println!("build status: development_only");
            println!("translation status: {}", report.patch.translation_status);
            println!("Expected Writes: {}", report.patch.writes.len());
            println!("{}", serde_json::to_string_pretty(&report.patch)?);
        }
        Command::BuildMadSystemInterfaceTextDevelopment {
            source,
            translations,
            output,
        } => {
            let report =
                build_mad_system_interface_text_development_image(&source, &translations, &output)?;
            println!("built: {}", output.display());
            println!("source sha256: {}", report.build.source_sha256);
            println!("output sha256: {}", report.build.output_sha256);
            println!("output size: {} bytes", report.build.output_size);
            println!("verified files: {}", report.build.file_count);
            println!("build status: development_only");
            println!("translation status: {}", report.patch.translation_status);
            println!("Expected Writes: {}", report.patch.writes.len());
            println!("{}", serde_json::to_string_pretty(&report.patch)?);
        }
        Command::BuildDialogueTextDevelopment {
            source,
            translations,
            output,
        } => {
            let report = build_dialogue_text_development_image(&source, &translations, &output)?;
            println!("built: {}", output.display());
            println!("source sha256: {}", report.build.source_sha256);
            println!("output sha256: {}", report.build.output_sha256);
            println!("output size: {} bytes", report.build.output_size);
            println!("verified files: {}", report.build.file_count);
            println!("build status: development_only");
            println!("translation status: {}", report.patch.translation_status);
            println!("Expected Writes: {}", report.patch.writes.len());
            println!("{}", serde_json::to_string_pretty(&report.patch)?);
        }
        Command::BuildMadSceneTextDevelopment {
            source,
            translations,
            output,
        } => {
            let report = build_mad_scene_text_development_image(&source, &translations, &output)?;
            println!("built: {}", output.display());
            println!("source sha256: {}", report.build.source_sha256);
            println!("output sha256: {}", report.build.output_sha256);
            println!("output size: {} bytes", report.build.output_size);
            println!("verified files: {}", report.build.file_count);
            println!("build status: development_only");
            println!("translation status: {}", report.patch.translation_status);
            println!("Expected Writes: {}", report.patch.writes.len());
            println!("{}", serde_json::to_string_pretty(&report.patch)?);
        }
        Command::BuildMadSceneNarrativeDevelopment {
            source,
            translations,
            title_artwork_manifest,
            title_source_preview,
            title_artwork,
            output,
        } => {
            let report = if let Some(title_source_preview) = title_source_preview {
                build_mad_scene_narrative_with_title_artwork_development_image(
                    &source,
                    &translations,
                    &title_artwork_manifest,
                    &title_source_preview,
                    &title_artwork,
                    &output,
                )?
            } else {
                build_mad_scene_narrative_development_image(&source, &translations, &output)?
            };
            println!("built: {}", output.display());
            println!("source sha256: {}", report.build.source_sha256);
            println!("output sha256: {}", report.build.output_sha256);
            println!("output size: {} bytes", report.build.output_size);
            println!("verified files: {}", report.build.file_count);
            println!("build status: development_only");
            println!("translation status: needs_human_review");
            println!(
                "system text disposition: {:?}",
                report.patch.scene_text.system_text_disposition
            );
            println!("Expected Writes: {}", report.patch.writes.len());
            println!("{}", serde_json::to_string_pretty(&report.patch)?);
        }
        Command::WriteInGameRetroPatcherPlan {
            source,
            content,
            output,
        } => {
            let report = write_in_game_retro_patcher_plan(&source, &content, &output)?;
            println!("wrote: {}", output.display());
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
        Command::VerifyInGameRetroPatcherResult {
            source,
            content,
            candidate,
        } => {
            let report = verify_in_game_retro_patcher_result(&source, &content, &candidate)?;
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
        Command::BuildIntegratedLocalizationDevelopment {
            source,
            translations,
            title_artwork_manifest,
            title_source_preview,
            title_artwork,
            arle_assets,
            preserve_original_arle,
            output,
        } => {
            let arle_character_assets =
                select_arle_character_assets(arle_assets, preserve_original_arle);
            let report = build_integrated_localization_development_image(
                &source,
                IntegratedLocalizationDevelopmentInputs {
                    translations: &translations,
                    title_artwork_manifest: &title_artwork_manifest,
                    title_source_preview: &title_source_preview,
                    title_artwork: &title_artwork,
                    arle_character_assets: &arle_character_assets,
                },
                &output,
            )?;
            println!("built: {}", output.display());
            println!("source sha256: {}", report.build.source_sha256);
            println!("output sha256: {}", report.build.output_sha256);
            println!("output size: {} bytes", report.build.output_size);
            println!("verified files: {}", report.build.file_count);
            println!("build status: development_only");
            println!("translation status: {}", report.patch.translation_status);
            println!("Arle character assets: {arle_character_assets}");
            println!(
                "component Expected Writes: {}",
                report
                    .patch
                    .components
                    .iter()
                    .map(|component| component.final_expected_write_count)
                    .sum::<usize>()
            );
            println!(
                "composition Expected Writes: {}",
                report.patch.composition_writes.len()
            );
            println!("{}", serde_json::to_string_pretty(&report.patch)?);
        }
        Command::BuildShellTextDevelopment {
            source,
            translations,
            output,
        } => {
            let report = build_shell_text_development_image(&source, &translations, &output)?;
            println!("built: {}", output.display());
            println!("source sha256: {}", report.build.source_sha256);
            println!("output sha256: {}", report.build.output_sha256);
            println!("output size: {} bytes", report.build.output_size);
            println!("verified files: {}", report.build.file_count);
            println!("build status: development_only");
            println!("translation status: {}", report.patch.translation_status);
            println!("Expected Writes: {}", report.patch.writes.len());
            println!("{}", serde_json::to_string_pretty(&report.patch)?);
        }
        Command::BuildSystemLoaderTextDevelopment {
            source,
            translations,
            output,
        } => {
            let report =
                build_system_loader_text_development_image(&source, &translations, &output)?;
            println!("built: {}", output.display());
            println!("source sha256: {}", report.build.source_sha256);
            println!("output sha256: {}", report.build.output_sha256);
            println!("output size: {} bytes", report.build.output_size);
            println!("verified files: {}", report.build.file_count);
            println!("build status: development_only");
            println!("translation status: {}", report.patch.translation_status);
            println!("Expected Writes: {}", report.patch.writes.len());
            println!(
                "resident Expected Writes: {}",
                report.patch.resident_writes.len()
            );
            println!("{}", serde_json::to_string_pretty(&report.patch)?);
        }
        Command::BuildSamplingDriverTextDevelopment {
            source,
            translations,
            output,
        } => {
            let report =
                build_sampling_driver_text_development_image(&source, &translations, &output)?;
            println!("built: {}", output.display());
            println!("source sha256: {}", report.build.source_sha256);
            println!("output sha256: {}", report.build.output_sha256);
            println!("output size: {} bytes", report.build.output_size);
            println!("verified files: {}", report.build.file_count);
            println!("build status: development_only");
            println!("translation status: {}", report.patch.translation_status);
            println!("Expected Writes: {}", report.patch.writes.len());
            println!("{}", serde_json::to_string_pretty(&report.patch)?);
        }
        Command::BuildMouseDriverTextDevelopment {
            source,
            translations,
            output,
        } => {
            let report =
                build_mouse_driver_text_development_image(&source, &translations, &output)?;
            println!("built: {}", output.display());
            println!("source sha256: {}", report.build.source_sha256);
            println!("output sha256: {}", report.build.output_sha256);
            println!("output size: {} bytes", report.build.output_size);
            println!("verified files: {}", report.build.file_count);
            println!("build status: development_only");
            println!("translation status: {}", report.patch.translation_status);
            println!("Expected Writes: {}", report.patch.writes.len());
            println!("{}", serde_json::to_string_pretty(&report.patch)?);
        }
        Command::BuildPlaybackDriverTextDevelopment {
            source,
            translations,
            output,
        } => {
            let report =
                build_playback_driver_text_development_image(&source, &translations, &output)?;
            println!("built: {}", output.display());
            println!("source sha256: {}", report.build.source_sha256);
            println!("output sha256: {}", report.build.output_sha256);
            println!("output size: {} bytes", report.build.output_size);
            println!("verified files: {}", report.build.file_count);
            println!("build status: development_only");
            println!("translation status: {}", report.patch.translation_status);
            println!("Expected Writes: {}", report.patch.writes.len());
            println!("{}", serde_json::to_string_pretty(&report.patch)?);
        }
        Command::BuildMenuTextDevelopment {
            source,
            translations,
            output,
        } => {
            let report = build_menu_text_development_image(&source, &translations, &output)?;
            println!("built: {}", output.display());
            println!("source sha256: {}", report.build.source_sha256);
            println!("output sha256: {}", report.build.output_sha256);
            println!("output size: {} bytes", report.build.output_size);
            println!("verified files: {}", report.build.file_count);
            println!("build status: development_only");
            println!("translation status: {}", report.patch.translation_status);
            println!("Expected Writes: {}", report.patch.writes.len());
            println!("{}", serde_json::to_string_pretty(&report.patch)?);
        }
        Command::BuildNarrativeGraphicsDevelopment {
            source,
            translations,
            output,
        } => {
            let report =
                build_narrative_graphics_development_image(&source, &translations, &output)?;
            println!("built: {}", output.display());
            println!("source sha256: {}", report.build.source_sha256);
            println!("output sha256: {}", report.build.output_sha256);
            println!("output size: {} bytes", report.build.output_size);
            println!("verified files: {}", report.build.file_count);
            println!("build status: development_only");
            println!("translation status: needs_human_review");
            println!(
                "Expected Writes: {}",
                report.patch.monochrome.writes.len() + report.patch.baked.writes.len()
            );
            println!("{}", serde_json::to_string_pretty(&report.patch)?);
        }
        Command::BuildTitleArtworkDevelopment {
            source,
            translations,
            manifest,
            source_preview,
            artwork,
            output,
        } => {
            let report = build_title_artwork_development_image(
                &source,
                &translations,
                &manifest,
                &source_preview,
                &artwork,
                &output,
            )?;
            println!("built: {}", output.display());
            println!("source sha256: {}", report.build.source_sha256);
            println!("output sha256: {}", report.build.output_sha256);
            println!("output size: {} bytes", report.build.output_size);
            println!("verified files: {}", report.build.file_count);
            println!("build status: development_only");
            println!("translation status: needs_human_review");
            println!(
                "Expected Writes: {}",
                report.patch.monochrome.writes.len() + report.patch.baked.writes.len()
            );
            println!("{}", serde_json::to_string_pretty(&report.patch)?);
        }
        Command::BuildArleCharacterAssetDevelopment {
            source,
            manifest,
            artwork,
            output,
        } => {
            let report = build_arle_character_asset_development_image(
                &source, &manifest, &artwork, &output,
            )?;
            println!("built: {}", output.display());
            println!("source sha256: {}", report.build.source_sha256);
            println!("output sha256: {}", report.build.output_sha256);
            println!("output size: {} bytes", report.build.output_size);
            println!("verified files: {}", report.build.file_count);
            println!("build status: development_only");
            println!("approval status: {}", report.patch.approval_status);
            println!("Expected Writes: {}", report.patch.writes.len());
            println!("{}", serde_json::to_string_pretty(&report.patch)?);
        }
    }
    Ok(())
}
