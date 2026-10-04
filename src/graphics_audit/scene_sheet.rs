use std::collections::BTreeMap;

use anyhow::Result;

use super::Preview;
use super::decoded::decode_single_asset;
use super::planar::render_strided_brgi;

struct SceneSheetLayout {
    name: &'static str,
    output_file: &'static str,
    width: usize,
    height: usize,
    source_row_bytes: usize,
    plane_stride: usize,
    evidence: &'static str,
}

const SCENE_SHEETS: [SceneSheetLayout; 5] = [
    SceneSheetLayout {
        name: "OP1.DAT",
        output_file: "op1-source-sheet.png",
        width: 640,
        height: 200,
        source_row_bytes: 80,
        plane_stride: 0x3e80,
        evidence: "MAD.COM file 0x99A2 consumer uses 80-byte rows and 0x3E80-byte planes",
    },
    SceneSheetLayout {
        name: "OP2.DAT",
        output_file: "op2-source-sheet.png",
        width: 320,
        height: 400,
        source_row_bytes: 40,
        plane_stride: 0x3e80,
        evidence: "MAD.COM file 0x99E6 consumer uses 40-byte rows and 0x3E80-byte planes",
    },
    SceneSheetLayout {
        name: "OP3.DAT",
        output_file: "op3-source-sheet.png",
        width: 320,
        height: 400,
        source_row_bytes: 40,
        plane_stride: 0x3e80,
        evidence: "MAD.COM file 0x99E6 consumer uses 40-byte rows and 0x3E80-byte planes",
    },
    SceneSheetLayout {
        name: "ED1.DAT",
        output_file: "ed1-source-sheet.png",
        width: 480,
        height: 200,
        source_row_bytes: 60,
        plane_stride: 0x2ee0,
        evidence: "MAD.COM file 0x99C4 consumer uses 60-byte rows and 0x2EE0-byte planes",
    },
    SceneSheetLayout {
        name: "ED2.DAT",
        output_file: "ed2-source-sheet.png",
        width: 320,
        height: 400,
        source_row_bytes: 40,
        plane_stride: 0x3e80,
        evidence: "MAD.COM file 0x99E6 consumer uses 40-byte rows and 0x3E80-byte planes",
    },
];

pub(super) fn render_scene_sheet_previews(
    installer_payload: &BTreeMap<String, Vec<u8>>,
) -> Result<Vec<Preview>> {
    SCENE_SHEETS
        .iter()
        .map(|layout| render_scene_sheet(installer_payload, layout))
        .collect()
}

fn render_scene_sheet(
    installer_payload: &BTreeMap<String, Vec<u8>>,
    layout: &SceneSheetLayout,
) -> Result<Preview> {
    let decoded = decode_single_asset(installer_payload, layout.name)?;
    let image = render_strided_brgi(
        &decoded,
        layout.width,
        layout.height,
        layout.source_row_bytes,
        layout.plane_stride,
    )?;
    Ok(Preview {
        source_asset: layout.name.to_owned(),
        output_file: layout.output_file.to_owned(),
        evidence: layout.evidence.to_owned(),
        image,
    })
}

#[cfg(test)]
#[path = "scene_sheet_tests.rs"]
mod scene_sheet_tests;
