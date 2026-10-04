use serde::Serialize;

use crate::translation_drafts::TranslationSurface;

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct TranslationGlyphAuditReport {
    pub supported_source_sha256: String,
    pub status: TranslationAnalysisStatus,
    pub translation_unit_count: usize,
    pub translated_text_unit_count: usize,
    pub preserved_source_control_unit_count: usize,
    pub retained_source_audio_unit_count: usize,
    pub render_paths: Vec<TranslationGlyphPathReport>,
    pub shared_gaiji_demand_sets: Vec<SharedGaijiDemandSetReport>,
    pub segments: Vec<TranslationGlyphSegmentReport>,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TranslationAnalysisStatus {
    NeedsHumanReview,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct TranslationGlyphPathReport {
    pub render_path: TranslationRenderPath,
    pub entry_count: usize,
    pub text_span_count: usize,
    pub demand: TranslationGlyphDemand,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct TranslationGlyphSegmentReport {
    pub id: String,
    pub render_path: TranslationRenderPath,
    pub entry_count: usize,
    pub text_span_count: usize,
    pub demand: TranslationGlyphDemand,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct SharedGaijiDemandSetReport {
    pub scope: SharedGaijiDemandScope,
    pub member_segment_ids: Vec<String>,
    pub entry_count: usize,
    pub text_span_count: usize,
    pub demand: TranslationGlyphDemand,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SharedGaijiDemandScope {
    MadFixedText,
    MadInterfaceText,
    MadDialogueText,
    MadSystemAndInterface,
    MadInterfaceAndDialogue,
    MadAllKnownText,
    MadBattleAndUnitList,
    BatchDriverText,
}

#[derive(Debug, Clone, Copy, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TranslationRenderPath {
    SharedPc98Gaiji,
    OpeningMonochromeSprites,
    EndingMonochromeSprites,
    BakedPlanarGraphics,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct TranslationGlyphDemand {
    pub unique_rendered_character_count: usize,
    pub unique_hangul_syllable_count: usize,
    pub slot_glyph_count: Option<usize>,
    pub verified_physical_capacity: Option<usize>,
    pub reserved_source_glyph_count: usize,
    pub available_translation_capacity: Option<usize>,
    pub physical_capacity_headroom: Option<isize>,
    pub physical_capacity_fit: Option<bool>,
    pub available_capacity_headroom: Option<isize>,
    pub available_capacity_fit: Option<bool>,
    pub rendered_characters: Vec<String>,
    pub slot_glyph_characters: Vec<String>,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct TranslationLayoutAuditReport {
    pub supported_source_sha256: String,
    pub status: TranslationAnalysisStatus,
    pub translation_unit_count: usize,
    pub measured_text_unit_count: usize,
    pub in_game_text_unit_count: usize,
    pub excluded_non_game_text_unit_count: usize,
    pub hard_layout_fit: bool,
    pub hard_violation_count: usize,
    pub source_growth_advisory_count: usize,
    pub open_consumer_gate_count: usize,
    pub scope_exclusion_count: usize,
    pub segments: Vec<TranslationLayoutSegmentReport>,
    pub findings: Vec<TranslationLayoutFinding>,
    pub open_consumer_gates: Vec<TranslationLayoutOpenGate>,
    pub scope_exclusions: Vec<TranslationLayoutScopeExclusion>,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct TranslationLayoutSegmentReport {
    pub id: String,
    pub surface: TranslationSurface,
    pub measured_entry_count: usize,
    pub max_source_line_count: usize,
    pub max_korean_line_count: usize,
    pub max_source_line_cells: usize,
    pub max_korean_line_cells: usize,
    pub hard_violation_count: usize,
    pub source_growth_advisory_count: usize,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct TranslationLayoutFinding {
    pub kind: TranslationLayoutFindingKind,
    pub segment_id: String,
    pub entry_id: String,
    pub line_index: Option<usize>,
    pub measured: usize,
    pub limit: usize,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TranslationLayoutFindingKind {
    FixedGridLineOverflow,
    FixedGridColumnOverflow,
    MonochromePageLineOverflow,
    MonochromePageColumnOverflow,
    BakedRegionLineOverflow,
    BakedRegionColumnOverflow,
    EmbeddedLineBreak,
    SourceLineCountGrowth,
    SourceLineWidthGrowth,
    SourceStorageGrowth,
}

impl TranslationLayoutFindingKind {
    pub(crate) fn is_hard_violation(self) -> bool {
        matches!(
            self,
            Self::FixedGridLineOverflow
                | Self::FixedGridColumnOverflow
                | Self::MonochromePageLineOverflow
                | Self::MonochromePageColumnOverflow
                | Self::BakedRegionLineOverflow
                | Self::BakedRegionColumnOverflow
                | Self::EmbeddedLineBreak
        )
    }
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct TranslationLayoutOpenGate {
    pub segment_id: String,
    pub surface: TranslationSurface,
    pub affected_entry_count: usize,
    pub reason: String,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct TranslationLayoutScopeExclusion {
    pub segment_id: String,
    pub surface: TranslationSurface,
    pub affected_entry_count: usize,
    pub reason: String,
}
