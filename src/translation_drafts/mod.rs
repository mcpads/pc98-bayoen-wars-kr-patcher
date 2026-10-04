mod model;
mod proposal;
mod publish;
mod validate;

use std::path::Path;

use anyhow::Result;

pub(crate) use model::{
    DevelopmentPolicy, DraftStatus, TranslationDraftEntry, TranslationDraftIndex,
    TranslationDraftSegment, TranslationDraftSegmentReference,
};
pub use model::{TranslationDraftReport, TranslationSurface};

pub(super) fn publish_translation_drafts(
    protected_workspace: &Path,
    proposal_directory: &Path,
    output_directory: &Path,
) -> Result<TranslationDraftReport> {
    let protected = proposal::read_protected_workspace(protected_workspace)?;
    let proposals = proposal::read_reviewed_proposals(proposal_directory)?;
    validate::validate_proposals(&protected, &proposals)?;
    publish::publish_drafts(&protected, &proposals, output_directory)
}

#[cfg(test)]
#[path = "publish_tests.rs"]
mod publish_tests;
