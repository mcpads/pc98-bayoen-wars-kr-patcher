use super::*;

fn runtime() -> SystemLoaderRuntimeCatalog {
    SystemLoaderRuntimeCatalog {
        source_file_size: 0x448b,
        entry_jump_offset: 0,
        initial_entry_address: 0x100,
        resident_size_load_offset: 0x100,
        resident_copy_count_load_offset: 0x120,
        resident_byte_count: 0x09a0,
        relocated_entry_address: 0x012c,
        post_stack_hook_offset: 0x0150,
        post_stack_hook_byte_count: 5,
        post_stack_resume_address: 0x0155,
        tail_offset: 0x09a0,
        tail_byte_count: 0x3aeb,
        tail_declared_copy_byte_count: 0x3a2d,
        tail_entry_target: 0x413b,
        references: Vec::new(),
    }
}

#[test]
fn post_stack_hook_fills_the_displaced_lds_width_with_typed_nops() {
    let assembled = assemble_post_stack_hook(&runtime(), 0x09a0).unwrap();
    let spans = assembled.instruction_spans();

    assert_eq!(assembled.bytes().len(), 5);
    assert!(matches!(
        spans[0].instruction,
        Instruction::Jmp {
            target: JmpTarget::Rel16(_)
        }
    ));
    assert_eq!(spans[1].instruction, Instruction::Nop);
    assert_eq!(spans[2].instruction, Instruction::Nop);
}

#[test]
fn trampoline_replays_the_exact_ss_lds_then_resumes() {
    let mut source = vec![0; 0x200];
    source[0x150..0x155].copy_from_slice(&[0x36, 0xc5, 0x36, 0x05, 0x00]);

    let assembled = assemble_post_stack_trampoline(&source, &runtime(), 0x1200).unwrap();

    assert_eq!(&assembled.bytes()[..5], &source[0x150..0x155]);
    assert!(matches!(
        assembled.instruction_spans()[1].instruction,
        Instruction::Jmp {
            target: JmpTarget::Rel16(_)
        }
    ));
}
