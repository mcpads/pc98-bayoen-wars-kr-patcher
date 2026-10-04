use anyhow::Result;

use super::Preview;
use super::contact_sheet::compose_contact_sheet;
use super::pc98_text::render_fixed_cell_lines_with_overrides;
use crate::DialogueTextPatchReport;
use crate::korean_patch::dialogue_punctuation_overrides;

pub(super) fn render_dialogue_text_previews(
    report: &DialogueTextPatchReport,
) -> Result<Vec<Preview>> {
    let punctuation = dialogue_punctuation_overrides()?;
    let mut previews = report
        .entries
        .iter()
        .map(|entry| {
            Ok(Preview {
                source_asset: "MAD.COM + GAIJI.COM".to_owned(),
                output_file: format!("{}.png", entry.id),
                evidence: format!(
                    "read-back {} record at MAD.COM {:#06X}, {} bytes, variant {}, pointer field {:#06X}, SHA-256 {}; diagnostic cells only, visible dialogue-box capacity remains unproven",
                    entry.group_id,
                    entry.file_offset,
                    entry.byte_size,
                    entry.presentation_variant,
                    entry.text_pointer_offset,
                    entry.content_sha256
                ),
                image: render_fixed_cell_lines_with_overrides(&entry.lines, &punctuation)?,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let contact_sheet = compose_contact_sheet(
        previews
            .iter()
            .map(|preview| preview.image.clone())
            .collect(),
        1024,
        8,
    )?;
    previews.push(Preview {
        source_asset: "MAD.COM + GAIJI.COM".to_owned(),
        output_file: "dialogue-text-contact-sheet.png".to_owned(),
        evidence: format!(
            "{} read-back dialogue records in group and record order; this sheet does not prove the runtime box bounds or scene reachability",
            report.entries.len()
        ),
        image: contact_sheet,
    });
    Ok(previews)
}
