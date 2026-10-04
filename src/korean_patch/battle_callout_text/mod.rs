mod compile;
mod hook;
mod model;
mod records;

pub(in crate::korean_patch) use compile::compile_battle_callouts;
pub(in crate::korean_patch) use hook::{
    BATTLE_WRAPPER_SOURCE_ID, BattleCalloutHookLayout, call_source_id, compile_battle_callout_hook,
};
pub use model::{BattleCalloutEntryPatchReport, BattleCalloutPatchReport};
pub(in crate::korean_patch) use model::{CompiledBattleCalloutHook, CompiledBattleCalloutText};

pub(in crate::korean_patch) use hook::assemble_banked_text_wrapper;
