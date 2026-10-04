use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct TranslationDraftReport {
    pub output_directory: PathBuf,
    pub supported_source_sha256: String,
    pub segment_count: usize,
    pub translation_unit_count: usize,
    pub translated_text_unit_count: usize,
    pub preserved_source_control_unit_count: usize,
    pub retained_source_audio_unit_count: usize,
    pub unresolved_question_count: usize,
}

#[derive(Debug, Deserialize)]
pub(super) struct ProtectedWorkspaceIndex {
    pub supported_source_sha256: String,
    pub translation_segmentation_complete: bool,
    pub segments: Vec<ProtectedSegmentReference>,
}

#[derive(Debug, Deserialize)]
pub(super) struct ProtectedSegmentReference {
    pub id: String,
    pub surface: TranslationSurface,
    pub path: String,
    pub entry_count: usize,
    pub content_sha256: String,
}

#[derive(Debug)]
pub(super) struct ProtectedWorkspace {
    pub root: PathBuf,
    pub index: ProtectedWorkspaceIndex,
    pub entry_ids_by_segment: std::collections::BTreeMap<String, Vec<String>>,
    pub control_only_entry_ids: std::collections::BTreeSet<String>,
}

#[derive(Debug, Deserialize)]
pub(super) struct ProposalBatch {
    pub batch_id: String,
    pub entries: Vec<ProposalEntry>,
    pub review: ProposalReview,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub(super) struct ProposalEntry {
    pub segment_id: String,
    pub id: String,
    pub korean_text: Vec<String>,
    pub notes: Option<String>,
    #[serde(default)]
    pub questions: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub(super) struct ProposalReview {
    pub method: ReviewMethod,
    pub outcome: ReviewOutcome,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum ReviewMethod {
    IndependentSecondLlm,
    CurrentAgentFirstDraft,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum ReviewOutcome {
    ReadyForHumanReview,
    HumanDecisionRequired,
}

#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct TranslationDraftIndex {
    pub(crate) supported_source_sha256: String,
    pub(crate) protected_index_sha256: String,
    pub(crate) status: DraftStatus,
    pub(crate) translation_unit_count: usize,
    pub(crate) translated_text_unit_count: usize,
    pub(crate) preserved_source_control_unit_count: usize,
    pub(crate) retained_source_audio_unit_count: usize,
    pub(crate) producer_evidence_segments: Vec<String>,
    pub(crate) segments: Vec<TranslationDraftSegmentReference>,
}

#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct TranslationDraftSegmentReference {
    pub(crate) id: String,
    pub(crate) surface: TranslationSurface,
    pub(crate) path: String,
    pub(crate) entry_count: usize,
    pub(crate) protected_segment_sha256: String,
    pub(crate) draft_sha256: String,
}

#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct TranslationDraftSegment {
    pub(crate) id: String,
    pub(crate) surface: TranslationSurface,
    pub(crate) protected_segment_sha256: String,
    pub(crate) draft_sha256: String,
    pub(crate) entries: Vec<TranslationDraftEntry>,
    pub(crate) review: TranslationDraftReview,
}

#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct TranslationDraftEntry {
    pub(crate) id: String,
    pub(crate) korean_text: Vec<String>,
    pub(crate) development_policy: DevelopmentPolicy,
    pub(crate) status: DraftStatus,
    pub(crate) notes: Option<String>,
    pub(crate) questions: Vec<String>,
}

#[derive(Debug, Clone, Copy, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum DevelopmentPolicy {
    Translate,
    PreserveSourceControl,
    RetainSourceAudio,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TranslationSurface {
    DosSystemText,
    InterfaceText,
    BattleCallout,
    FixedGaijiText,
    Dialogue,
    MonochromeGlyphAtlas,
    OpeningGlyphIndexPages,
    EndingGlyphIndexPages,
    MenuText,
    ExternalProgramText,
    JapaneseVoicePerformance,
    BakedGraphicsText,
}

#[derive(Debug, Clone, Copy, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum DraftStatus {
    NeedsHumanReview,
}

#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct TranslationDraftReview {
    pub(crate) reviewed_draft_sha256: String,
    pub(super) method: ReviewMethod,
    pub(super) outcome: ReviewOutcome,
    pub(super) notes: Vec<String>,
}
