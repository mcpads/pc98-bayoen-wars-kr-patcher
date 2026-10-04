use std::collections::BTreeMap;

use anyhow::{Context, Result, ensure};
use serde::Serialize;

const CONFIG_SYS: &[u8] = b"files=10\r\nbuffers=10\r\nshell=dsh.com\r\n\x1a";
const DMADOU_BAT: &[u8] = b"FPLAY6.COM\r\nBPLAY6.COM\r\nBSAMP.COM\r\nNMOUSE.COM /F1 /CA\r\nGAIJI.COM\r\nMAD.COM\r\nGAIJI.COM\r\nNMOUSE.COM /F1 /CA\r\nBSAMP.COM\r\nBPLAY6.COM\r\nFPLAY6.COM\r\n\x1a";
const DATA_DAT: &[u8] = &[0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x64, 0x26, 0x00];

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct SupportFileBinding {
    pub file_name: String,
    pub byte_size: usize,
    pub role: String,
    pub evidence: String,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct SupportFileCatalog {
    pub file_count: usize,
    pub files: Vec<SupportFileBinding>,
}

pub(super) fn catalog_support_files(
    installer_payload: &BTreeMap<String, Vec<u8>>,
    boot_files: &BTreeMap<String, Vec<u8>>,
) -> Result<SupportFileCatalog> {
    require_exact(boot_files, "CONFIG.SYS", CONFIG_SYS)?;
    require_exact(installer_payload, "DMADOU.BAT", DMADOU_BAT)?;
    require_exact(installer_payload, "DATA.DAT", DATA_DAT)?;

    let files = vec![
        SupportFileBinding {
            file_name: "CONFIG.SYS".to_owned(),
            byte_size: CONFIG_SYS.len(),
            role: "dos_boot_configuration".to_owned(),
            evidence: "complete file contains only DOS files, buffers, and shell directives"
                .to_owned(),
        },
        SupportFileBinding {
            file_name: "DATA.DAT".to_owned(),
            byte_size: DATA_DAT.len(),
            role: "binary_state_record".to_owned(),
            evidence: "complete nine-byte file is binary state with no text storage".to_owned(),
        },
        SupportFileBinding {
            file_name: "DMADOU.BAT".to_owned(),
            byte_size: DMADOU_BAT.len(),
            role: "game_process_orchestration".to_owned(),
            evidence:
                "complete batch file contains only program names, switches, and CRLF separators"
                    .to_owned(),
        },
    ];
    Ok(SupportFileCatalog {
        file_count: files.len(),
        files,
    })
}

fn require_exact(files: &BTreeMap<String, Vec<u8>>, name: &str, expected: &[u8]) -> Result<()> {
    let actual = files
        .get(name)
        .with_context(|| format!("support file population is missing {name}"))?;
    ensure!(
        actual == expected,
        "support file {name} does not match its complete expected content"
    );
    Ok(())
}

#[cfg(test)]
#[path = "support_files_tests.rs"]
mod support_files_tests;
