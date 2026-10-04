use anyhow::{Result, ensure};

use super::catalog::{ExternalProgramTextCatalog, ExternalTextStorage};
use super::program_text::ProgramTextBuilder;

const DOS_OUTPUTS: [(usize, usize, &str); 8] = [
    (0x03a0, 0x020c, "startup_banner"),
    (0x03f0, 0x0151, "interrupt_error"),
    (0x040f, 0x024a, "memory_resize_failure"),
    (0x0428, 0x0269, "exec_error_invalid_function"),
    (0x0443, 0x0270, "exec_error_file_not_found"),
    (0x045b, 0x0277, "exec_error_insufficient_memory"),
    (0x0472, 0x027e, "exec_error_invalid_environment"),
    (0x0483, 0x0285, "exec_error_invalid_format"),
];

pub(crate) fn catalog_shell(bytes: &[u8]) -> Result<ExternalProgramTextCatalog> {
    ensure!(
        bytes.get(0x0151..0x0158) == Some(&[0xba, 0xf0, 0x03, 0xb4, 0x09, 0xcd, 0x21][..]),
        "DSH.COM interrupt-error output consumer does not match"
    );
    ensure!(
        bytes.get(0x020c..0x0213) == Some(&[0xba, 0xa0, 0x03, 0xb4, 0x09, 0xcd, 0x21][..]),
        "DSH.COM startup output consumer does not match"
    );
    ensure!(
        bytes.get(0x0269..0x028c)
            == Some(
                &[
                    0xba, 0x28, 0x04, 0x3c, 0x01, 0x74, 0x18, 0xba, 0x43, 0x04, 0x3c, 0x02, 0x74,
                    0x11, 0xba, 0x5b, 0x04, 0x3c, 0x08, 0x74, 0x0a, 0xba, 0x72, 0x04, 0x3c, 0x0a,
                    0x74, 0x03, 0xba, 0x83, 0x04, 0xb4, 0x09, 0xcd, 0x21,
                ][..]
            ),
        "DSH.COM execution-error output switch does not match"
    );

    let mut builder = ProgramTextBuilder::new(
        "DSH.COM",
        ExternalTextStorage::OriginalFile,
        bytes.len(),
        bytes,
    );
    for (runtime_address, consumer_offset, role) in DOS_OUTPUTS {
        builder.add_com_dos_reference(runtime_address, consumer_offset, role, None)?;
    }
    builder.finish()
}

#[cfg(test)]
#[path = "shell_tests.rs"]
mod shell_tests;
