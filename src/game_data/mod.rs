mod battle_callout;
mod unit_list_status;
pub use unit_list_status::UnitListStatusCatalog;
mod binary;
mod dialogue;
mod fixed_gaiji_text;
mod gaiji;
mod gaiji_installer;
mod gaiji_meaning;
mod gaiji_readiness;
mod interface_text;
mod interface_window_binary;
mod mad_ending_transition;
mod mad_memory;
mod mad_scene_transition;
mod mad_system_runtime;
mod mad_system_text;
mod menu;
mod menu_runtime;
mod shared_alert_window;
mod spring_capture_window;
mod spring_recovery_window;
mod stage_result_window;

use anyhow::Result;
use serde::Serialize;

use crate::source_disk::sha256_hex;

pub use battle_callout::{BattleCalloutCall, BattleCalloutCatalog, BattleCalloutPointer};
pub(crate) use dialogue::parse_dialogue_text_controls;
pub use dialogue::{
    DialogueCatalog, DialogueEntry, DialogueGroup, DialogueNonReferenceOccurrence,
    DialogueReference, DialogueReferenceCatalog, DialogueReferenceKind,
};
pub use fixed_gaiji_text::{
    FixedGaijiTextCatalog, FixedGaijiTextRuntimeCall, FixedGaijiTextRuntimeCatalog,
    FixedGaijiTextSlot, GaijiTextCell,
};
pub use gaiji::{GaijiCatalog, GaijiGlyph};
pub(crate) use gaiji::{jis_row_cell_to_shift_jis, parse_gaiji_program};
pub use gaiji_installer::GaijiInstallerCatalog;
pub(crate) use gaiji_meaning::GRAPHIC_GLYPH_COUNT;
pub use gaiji_meaning::GaijiGlyphMeaning;
pub(crate) use gaiji_readiness::GAIJI_READINESS_GLYPH_COUNT;
pub use gaiji_readiness::GaijiReadinessCatalog;
pub(crate) use interface_text::parse_interface_text_record;
pub use interface_text::{
    InterfaceTextCatalog, InterfaceTextEntry, InterfaceTextReference,
    InterfaceTextReferenceCatalog, InterfaceTextReferenceKind, InterfaceTextReferenceTable,
    InterfaceTextToken,
};
pub use mad_ending_transition::MadEndingTransitionCatalog;
pub use mad_memory::MadMemoryCatalog;
pub use mad_scene_transition::{MadSceneTransitionCall, MadSceneTransitionCatalog};
pub use mad_system_runtime::{
    MadSystemRuntimeCatalog, MadSystemRuntimeReference, MadSystemRuntimeReferenceKind,
};
pub use mad_system_text::{
    MadSystemTextCatalog, MadSystemTextEntry, MadSystemTextReference, RuntimeTextInsert,
};
pub use menu::{MenuProgramCatalog, MenuTextCatalog, MenuTextEntry, MenuTextReference};
pub use menu_runtime::{
    MenuEntryRuntimeCatalog, MenuRuntimeCatalog, MenuRuntimeInsertReference,
    MenuTextNonReferenceOccurrence, MenuTextRuntimeReference, MenuTextRuntimeReferenceKind,
};
pub use shared_alert_window::{SharedAlertWindowCatalog, SharedAlertWindowState};
pub use spring_capture_window::SpringCaptureWindowCatalog;
pub use spring_recovery_window::SpringRecoveryWindowCatalog;

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct GameDataCatalog {
    pub mad_com: MadProgramCatalog,
    pub gaiji: GaijiCatalog,
    pub menu: MenuProgramCatalog,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct MadProgramCatalog {
    pub sha256: String,
    pub file_size: usize,
    pub memory: MadMemoryCatalog,
    pub ending_transition: MadEndingTransitionCatalog,
    pub scene_transition: MadSceneTransitionCatalog,
    pub battle_callouts: BattleCalloutCatalog,
    pub unit_list_status: UnitListStatusCatalog,
    pub shared_alert_window: SharedAlertWindowCatalog,
    pub stage_result_window: SharedAlertWindowCatalog,
    pub spring_capture_window: SpringCaptureWindowCatalog,
    pub spring_recovery_window: SpringRecoveryWindowCatalog,
    pub text_controls: TextControlCatalog,
    pub system_text: MadSystemTextCatalog,
    pub system_runtime: MadSystemRuntimeCatalog,
    pub interface_text: InterfaceTextCatalog,
    pub fixed_gaiji_text: FixedGaijiTextCatalog,
    pub dialogue: DialogueCatalog,
    pub gaiji_readiness: GaijiReadinessCatalog,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct TextControlCatalog {
    pub terminator_hex: String,
    pub line_break_hex: String,
    pub fixed_text_renderer_offset: usize,
}

pub(crate) fn catalog_game_data(
    mad_com: &[u8],
    gaiji_com: &[u8],
    menu_com: &[u8],
) -> Result<GameDataCatalog> {
    let gaiji = gaiji::parse_gaiji_program(gaiji_com)?;
    let fixed_gaiji_text = fixed_gaiji_text::parse_fixed_gaiji_text(mad_com, &gaiji)?;
    let dialogue = dialogue::parse_dialogue_catalog(mad_com)?;
    let system_text = mad_system_text::parse_mad_system_text(mad_com)?;
    let system_runtime = mad_system_runtime::catalog_mad_system_runtime(mad_com, &system_text)?;
    let interface_text = interface_text::parse_interface_text_catalog(mad_com, &gaiji)?;
    let gaiji_readiness = gaiji_readiness::parse_gaiji_readiness(mad_com, gaiji_com, &gaiji)?;
    let memory = mad_memory::parse_mad_memory(mad_com)?;
    let scene_transition = mad_scene_transition::catalog_mad_scene_transition(mad_com)?;
    let unit_list_status = unit_list_status::catalog_unit_list_status(mad_com, &gaiji)?;
    let battle_callouts = battle_callout::parse_battle_callouts(mad_com, &gaiji)?;
    let shared_alert_window =
        shared_alert_window::catalog_shared_alert_window(mad_com, &interface_text)?;
    let stage_result_window =
        stage_result_window::catalog_stage_result_window(mad_com, &interface_text)?;
    let spring_capture_window =
        spring_capture_window::catalog_spring_capture_window(mad_com, &interface_text)?;
    let spring_recovery_window =
        spring_recovery_window::catalog_spring_recovery_window(mad_com, &interface_text)?;
    let ending_transition = mad_ending_transition::catalog_mad_ending_transition(
        mad_com,
        &scene_transition,
        &dialogue.groups,
    )?;

    Ok(GameDataCatalog {
        mad_com: MadProgramCatalog {
            sha256: sha256_hex(mad_com),
            file_size: mad_com.len(),
            memory,
            ending_transition,
            scene_transition,
            battle_callouts,
            unit_list_status,
            shared_alert_window,
            stage_result_window,
            spring_capture_window,
            spring_recovery_window,
            text_controls: TextControlCatalog {
                terminator_hex: "24 24".to_owned(),
                line_break_hex: "24 30".to_owned(),
                fixed_text_renderer_offset: fixed_gaiji_text.runtime.renderer_file_offset,
            },
            system_text,
            system_runtime,
            interface_text,
            fixed_gaiji_text,
            dialogue,
            gaiji_readiness,
        },
        gaiji,
        menu: menu::parse_menu_program(menu_com)?,
    })
}
