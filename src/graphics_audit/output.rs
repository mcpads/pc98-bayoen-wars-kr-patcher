use std::fs;
use std::io::BufWriter;
use std::path::Path;

use anyhow::{Context, Result};

use super::Preview;
use crate::audit_directory::publish_new_directory;
use crate::{GraphicsAuditPreview, GraphicsAuditReport};

pub(super) fn publish_previews(
    output_directory: &Path,
    previews: Vec<Preview>,
) -> Result<GraphicsAuditReport> {
    let records = publish_new_directory(output_directory, |staged| {
        let mut records = Vec::with_capacity(previews.len());
        for preview in previews {
            let path = staged.join(&preview.output_file);
            write_png(&path, &preview.image)?;
            records.push(GraphicsAuditPreview {
                source_asset: preview.source_asset,
                output_file: preview.output_file,
                width: preview.image.width,
                height: preview.image.height,
                evidence: preview.evidence,
            });
        }
        Ok(records)
    })?;

    Ok(GraphicsAuditReport {
        output_directory: output_directory.to_path_buf(),
        preview_count: records.len(),
        previews: records,
    })
}

fn write_png(path: &Path, image: &super::planar::RgbImage) -> Result<()> {
    let file = fs::File::create(path)
        .with_context(|| format!("failed to create graphics preview {}", path.display()))?;
    let mut encoder = png::Encoder::new(
        BufWriter::new(file),
        u32::try_from(image.width)?,
        u32::try_from(image.height)?,
    );
    encoder.set_color(png::ColorType::Rgb);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header()?;
    writer.write_image_data(&image.pixels)?;
    Ok(())
}

#[cfg(test)]
#[path = "output_tests.rs"]
mod output_tests;
