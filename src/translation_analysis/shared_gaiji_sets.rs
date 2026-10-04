use super::glyph_demand::DemandAccumulator;
use super::model::{SharedGaijiDemandScope, SharedGaijiDemandSetReport, TranslationRenderPath};
use crate::game_data::{GAIJI_READINESS_GLYPH_COUNT, GRAPHIC_GLYPH_COUNT};
use crate::korean_patch::{DIALOGUE_GAIJI_PHYSICAL_CAPACITY, DIALOGUE_GAIJI_PUNCTUATION};
use crate::translation_drafts::{DevelopmentPolicy, TranslationDraftSegment, TranslationSurface};

const SCOPES: [SharedGaijiDemandScope; 8] = [
    SharedGaijiDemandScope::MadFixedText,
    SharedGaijiDemandScope::MadInterfaceText,
    SharedGaijiDemandScope::MadDialogueText,
    SharedGaijiDemandScope::MadSystemAndInterface,
    SharedGaijiDemandScope::MadInterfaceAndDialogue,
    SharedGaijiDemandScope::MadAllKnownText,
    SharedGaijiDemandScope::BatchDriverText,
    SharedGaijiDemandScope::MadBattleAndUnitList,
];

pub(super) fn audit_shared_gaiji_demand_sets(
    segments: &[TranslationDraftSegment],
) -> Vec<SharedGaijiDemandSetReport> {
    SCOPES
        .into_iter()
        .map(|scope| {
            let members = segments
                .iter()
                .filter(|segment| includes(scope, segment))
                .collect::<Vec<_>>();
            let mut demand = DemandAccumulator::default();
            for segment in &members {
                for entry in &segment.entries {
                    if entry.development_policy == DevelopmentPolicy::Translate {
                        demand.add_translation(&entry.korean_text);
                    }
                }
                if segment.surface == TranslationSurface::Dialogue {
                    demand.force_shared_gaiji(&DIALOGUE_GAIJI_PUNCTUATION);
                }
            }
            let entry_count = demand.entry_count();
            let text_span_count = demand.text_span_count();
            let demand_report = if scope == SharedGaijiDemandScope::MadDialogueText {
                demand.report_with_capacity(
                    TranslationRenderPath::SharedPc98Gaiji,
                    Some(DIALOGUE_GAIJI_PHYSICAL_CAPACITY),
                    GRAPHIC_GLYPH_COUNT + GAIJI_READINESS_GLYPH_COUNT,
                )
            } else {
                demand.report(TranslationRenderPath::SharedPc98Gaiji)
            };
            SharedGaijiDemandSetReport {
                scope,
                member_segment_ids: members.iter().map(|segment| segment.id.clone()).collect(),
                entry_count,
                text_span_count,
                demand: demand_report,
            }
        })
        .collect()
}

fn includes(scope: SharedGaijiDemandScope, segment: &TranslationDraftSegment) -> bool {
    if segment.id == "mad-unit-list-status"
        && matches!(
            scope,
            SharedGaijiDemandScope::MadInterfaceText
                | SharedGaijiDemandScope::MadSystemAndInterface
                | SharedGaijiDemandScope::MadInterfaceAndDialogue
        )
    {
        return false;
    }
    match scope {
        SharedGaijiDemandScope::MadBattleAndUnitList => {
            segment.surface == TranslationSurface::BattleCallout
                || segment.id == "mad-unit-list-status"
        }
        SharedGaijiDemandScope::MadFixedText => {
            segment.surface == TranslationSurface::FixedGaijiText
        }
        SharedGaijiDemandScope::MadInterfaceText => {
            segment.surface == TranslationSurface::InterfaceText
        }
        SharedGaijiDemandScope::MadDialogueText => segment.surface == TranslationSurface::Dialogue,
        SharedGaijiDemandScope::MadSystemAndInterface => matches!(
            segment.surface,
            TranslationSurface::DosSystemText | TranslationSurface::InterfaceText
        ),
        SharedGaijiDemandScope::MadInterfaceAndDialogue => matches!(
            segment.surface,
            TranslationSurface::InterfaceText | TranslationSurface::Dialogue
        ),
        SharedGaijiDemandScope::MadAllKnownText => matches!(
            segment.surface,
            TranslationSurface::DosSystemText
                | TranslationSurface::FixedGaijiText
                | TranslationSurface::InterfaceText
                | TranslationSurface::BattleCallout
                | TranslationSurface::Dialogue
        ),
        SharedGaijiDemandScope::BatchDriverText => matches!(
            segment.id.as_str(),
            "external-bplay6" | "external-bsamp" | "external-fplay6" | "external-nmouse"
        ),
    }
}

#[cfg(test)]
#[path = "shared_gaiji_sets_tests.rs"]
mod shared_gaiji_sets_tests;
