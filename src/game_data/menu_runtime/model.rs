use serde::Serialize;

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct MenuRuntimeCatalog {
    pub reference_population_complete: bool,
    pub semantic_reference_count: usize,
    pub storage_reference_count: usize,
    pub machine_code_reference_count: usize,
    pub metadata_reference_count: usize,
    pub runtime_insert_reference_count: usize,
    pub total_machine_code_relocation_count: usize,
    pub referenced_entry_count: usize,
    pub non_reference_occurrence_count: usize,
    pub entry: MenuEntryRuntimeCatalog,
    pub references: Vec<MenuTextRuntimeReference>,
    pub runtime_insert_references: Vec<MenuRuntimeInsertReference>,
    pub non_reference_occurrences: Vec<MenuTextNonReferenceOccurrence>,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct MenuEntryRuntimeCatalog {
    pub hook_instruction_offset: usize,
    pub hook_com_address: u16,
    pub original_call_target_com_address: u16,
    pub resume_com_address: u16,
    pub initial_stack_pointer: u16,
    pub program_end_com_address: u16,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct MenuTextRuntimeReference {
    pub id: String,
    pub storage_kind: MenuTextRuntimeReferenceKind,
    pub storage_offset: usize,
    pub instruction_offset: Option<usize>,
    pub target_com_address: u16,
    pub target_entry_id: String,
    pub consumer_roles: Vec<String>,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MenuTextRuntimeReferenceKind {
    MachineCodeImmediate,
    MetadataTableEntry,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct MenuTextNonReferenceOccurrence {
    pub storage_offset: usize,
    pub word: u16,
    pub matched_target_id: String,
    pub reason: String,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct MenuRuntimeInsertReference {
    pub id: String,
    pub role: String,
    pub target_entry_id: String,
    pub entry_byte_offset: usize,
    pub byte_capacity: usize,
    pub instruction_offset: usize,
    pub storage_offset: usize,
    pub target_com_address: u16,
}
