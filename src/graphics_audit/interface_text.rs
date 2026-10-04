use anyhow::Result;

use super::Preview;
use super::contact_sheet::compose_contact_sheet;
use super::pc98_text::render_fixed_cell_lines;
use crate::InterfaceTextPatchReport;

pub(super) fn render_interface_text_previews(
    report: &InterfaceTextPatchReport,
) -> Result<Vec<Preview>> {
    let mut previews = report
        .entries
        .iter()
        .map(|entry| {
            let consumer_evidence = if entry.reference_count == 0 {
                "no source consumer reference; retained as an unreferenced source record".to_owned()
            } else {
                format!(
                    "{} consumer references: {} typed V30 instructions and {} metadata entries",
                    entry.reference_count,
                    entry.machine_code_reference_count,
                    entry.metadata_reference_count
                )
            };
            Ok(Preview {
                source_asset: "MAD.COM + GAIJI.COM".to_owned(),
                output_file: format!("{}.png", entry.id),
                evidence: format!(
                    "read-back record at MAD.COM {:#06X}, {} bytes, SHA-256 {}; {consumer_evidence}",
                    entry.file_offset, entry.byte_size, entry.content_sha256
                ),
                image: render_fixed_cell_lines(&entry.lines)?,
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
        output_file: "interface-text-contact-sheet.png".to_owned(),
        evidence: format!(
            "{} read-back interface records in stable ID order; individual previews retain per-record consumer evidence",
            report.entries.len()
        ),
        image: contact_sheet,
    });
    Ok(previews)
}
