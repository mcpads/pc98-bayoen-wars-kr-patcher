mod entry;
mod model;
mod references;
mod runtime_inserts;

use anyhow::{Result, ensure};

use super::menu::MenuTextEntry;
use entry::catalog_entry;
pub use model::{
    MenuEntryRuntimeCatalog, MenuRuntimeCatalog, MenuRuntimeInsertReference,
    MenuTextNonReferenceOccurrence, MenuTextRuntimeReference, MenuTextRuntimeReferenceKind,
};
use references::catalog_references;
use runtime_inserts::catalog_runtime_insert_references;

pub(super) fn catalog_menu_runtime(
    program: &[u8],
    entries: &[MenuTextEntry],
    semantic_reference_count: usize,
) -> Result<MenuRuntimeCatalog> {
    let entry = catalog_entry(program)?;
    let cataloged = catalog_references(program, entries, semantic_reference_count)?;
    let runtime_insert_references = catalog_runtime_insert_references(program, entries)?;
    ensure!(
        runtime_insert_references.len() == 2,
        "MENU runtime insertion reference population changed"
    );

    Ok(MenuRuntimeCatalog {
        reference_population_complete: true,
        semantic_reference_count,
        storage_reference_count: cataloged.references.len(),
        machine_code_reference_count: cataloged.machine_code_reference_count,
        metadata_reference_count: cataloged.metadata_reference_count,
        runtime_insert_reference_count: runtime_insert_references.len(),
        total_machine_code_relocation_count: cataloged.machine_code_reference_count
            + runtime_insert_references.len(),
        referenced_entry_count: cataloged.referenced_entry_count,
        non_reference_occurrence_count: cataloged.non_reference_occurrences.len(),
        entry,
        references: cataloged.references,
        runtime_insert_references,
        non_reference_occurrences: cataloged.non_reference_occurrences,
    })
}

#[cfg(test)]
#[path = "menu_runtime_tests.rs"]
mod menu_runtime_tests;
