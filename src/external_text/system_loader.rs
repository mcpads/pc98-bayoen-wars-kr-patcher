use anyhow::{Result, ensure};

use super::catalog::{ExternalProgramTextCatalog, ExternalTextStorage};
use super::program_text::ProgramTextBuilder;
use super::system_loader_runtime::catalog_system_loader_runtime;

const SYSTEM_MESSAGES: [(usize, usize, &str); 7] = [
    (0x082a, 0x02b1, "shell_launch_failure"),
    (0x0854, 0x0330, "config_file_too_large"),
    (0x0891, 0x037e, "invalid_config_command"),
    (0x08c4, 0x05ae, "file_not_found_or_invalid"),
    (0x08ed, 0x05b3, "too_many_disk_drives"),
    (0x0910, 0x05b8, "sector_size_too_large"),
    (0x0937, 0x06ae, "insufficient_memory"),
];

pub(crate) fn catalog_system_loader(bytes: &[u8]) -> Result<ExternalProgramTextCatalog> {
    ensure!(
        bytes.get(0x02a9..0x02b9)
            == Some(
                &[
                    0x83, 0xf8, 0x08, 0xba, 0x37, 0x09, 0x74, 0x03, 0xba, 0x2a, 0x08, 0xb0, 0x00,
                    0xe8, 0x36, 0x04,
                ][..]
            ),
        "MEGDOS.SYS shell-error consumer does not match"
    );
    ensure!(
        bytes.get(0x05ae..0x05c2)
            == Some(
                &[
                    0xba, 0xc4, 0x08, 0xeb, 0x03, 0xba, 0xed, 0x08, 0xeb, 0x03, 0xba, 0x10, 0x09,
                    0x0e, 0x1f, 0xb0, 0x00, 0xe8, 0x2d, 0x01,
                ][..]
            ),
        "MEGDOS.SYS execution-error consumer does not match"
    );

    let mut builder = ProgramTextBuilder::new(
        "MEGDOS.SYS",
        ExternalTextStorage::OriginalFile,
        bytes.len(),
        bytes,
    );
    for (text_offset, consumer_offset, role) in SYSTEM_MESSAGES {
        builder.add_null_reference(text_offset, consumer_offset, role)?;
    }
    builder.add_null_reference(0x0937, 0x02ac, "shell_launch_insufficient_memory")?;
    let catalog = builder.finish()?;
    catalog_system_loader_runtime(bytes, &catalog)?;
    Ok(catalog)
}

#[cfg(test)]
#[path = "system_loader_tests.rs"]
mod system_loader_tests;
