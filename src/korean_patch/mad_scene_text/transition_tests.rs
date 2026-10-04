use v30::{CallTarget, Instruction, JmpTarget, decode_bytes};

use super::*;
use crate::game_data::FixedGaijiTextRuntimeCall;

fn catalog() -> MadSceneTransitionCatalog {
    MadSceneTransitionCatalog {
        startup_call: MadSceneTransitionCall {
            id: "startup-interface-bank".to_owned(),
            file_offset: 0x000d,
            com_address: 0x010d,
            byte_size: 3,
            target_com_address: 0x2fa6,
        },
        dialogue_routine_file_offset: 0xaf4b,
        dialogue_routine_com_address: 0xb04b,
        failure_exit_file_offset: 0x0490,
        failure_exit_com_address: 0x0590,
        call_site_count: 2,
        calls: vec![
            MadSceneTransitionCall {
                id: "new-game-dialogue".to_owned(),
                file_offset: 0x0290,
                com_address: 0x0390,
                byte_size: 3,
                target_com_address: 0xb04b,
            },
            MadSceneTransitionCall {
                id: "ending-dialogue".to_owned(),
                file_offset: 0x03cc,
                com_address: 0x04cc,
                byte_size: 3,
                target_com_address: 0xb04b,
            },
        ],
    }
}

fn fixed_runtime() -> FixedGaijiTextRuntimeCatalog {
    FixedGaijiTextRuntimeCatalog {
        selection_mode_load_file_offset: 0xa040,
        selection_entry_call: FixedGaijiTextRuntimeCall {
            file_offset: 0xa042,
            com_address: 0xa142,
            byte_size: 3,
            target_com_address: 0x2b74,
        },
        render_call: FixedGaijiTextRuntimeCall {
            file_offset: 0xa477,
            com_address: 0xa577,
            byte_size: 3,
            target_com_address: 0x3b48,
        },
        renderer_file_offset: 0x3a48,
        renderer_com_address: 0x3b48,
    }
}

fn layout(wrapper_capacity: usize) -> SceneTransitionLayout {
    SceneTransitionLayout {
        wrapper_offset: 0xb758,
        wrapper_capacity,
        buffer_offset: 0x2e01,
        buffer_capacity: 122,
        interface_bank_file_offset: 0x01ea,
        dialogue_bank_file_offset: 0x1a5a,
        fixed_bank_file_offset: 0x32ca,
        gaiji_record_count: 184,
        status_record_count: 26,
    }
}

#[test]
fn wrapper_fits_owned_tail_and_routes_selection_and_dialogue_banks() {
    let compiled = compile_scene_transition(&catalog(), &fixed_runtime(), layout(149)).unwrap();

    assert!(compiled.wrapper.bytes().len() <= compiled.wrapper_capacity);
    assert_eq!(compiled.buffer_offset, 0x2e01);
    assert_eq!(
        compiled.file_name_offset,
        compiled.buffer_offset + GAIJI_RECORD_BYTE_SIZE
    );
    assert_eq!(compiled.call_sources.len(), 2);

    let startup = decode_bytes(compiled.startup_entry_source.bytes()).unwrap();
    let Instruction::Call {
        target: CallTarget::Rel16(startup_displacement),
    } = startup.instruction
    else {
        panic!("startup entry redirect is not a near call");
    };
    assert_eq!(
        compiled
            .startup_entry_source
            .origin()
            .off
            .wrapping_add(3)
            .wrapping_add_signed(startup_displacement),
        compiled.startup_entry_com_address
    );
    assert!(compiled.wrapper.placed_instructions().iter().any(|placed| {
        matches!(
            placed.instruction,
            Instruction::Call {
                target: CallTarget::Rel16(displacement)
            } if placed.location.off.wrapping_add(3).wrapping_add_signed(displacement) == 0x2fa6
        )
    }));
    assert_eq!(
        compiled
            .wrapper
            .placed_instructions()
            .iter()
            .filter(|placed| matches!(placed.instruction, Instruction::Int { vector: 0x18 }))
            .count(),
        1
    );
    for (call, source) in catalog().calls.iter().zip(compiled.call_sources.values()) {
        let decoded = decode_bytes(source.bytes()).unwrap();
        let Instruction::Call {
            target: CallTarget::Rel16(displacement),
        } = decoded.instruction
        else {
            panic!("{} redirect is not a near call", call.id);
        };
        assert_eq!(
            source
                .origin()
                .off
                .wrapping_add(3)
                .wrapping_add_signed(displacement),
            0xb858
        );
    }
    let selection = decode_bytes(compiled.selection_entry_source.bytes()).unwrap();
    let Instruction::Call {
        target: CallTarget::Rel16(selection_displacement),
    } = selection.instruction
    else {
        panic!("selection entry redirect is not a near call");
    };
    assert_eq!(
        compiled
            .selection_entry_source
            .origin()
            .off
            .wrapping_add(3)
            .wrapping_add_signed(selection_displacement),
        compiled.fixed_entry_com_address
    );
    assert!(compiled.wrapper.placed_instructions().iter().any(|placed| {
        matches!(
            placed.instruction,
            Instruction::Jmp {
                target: JmpTarget::Rel16(displacement)
            } if placed.location.off.wrapping_add(3).wrapping_add_signed(displacement) == 0x2b74
        )
    }));
}

#[test]
fn record_reads_preserve_the_file_name_for_later_bank_switches() {
    let compiled = compile_scene_transition(&catalog(), &fixed_runtime(), layout(149)).unwrap();
    let mut mad_com = vec![0_u8; 0x1_0000];
    let file_name_end = compiled.file_name_offset + compiled.file_name.len();
    mad_com[compiled.file_name_offset..file_name_end].copy_from_slice(&compiled.file_name);

    mad_com[compiled.buffer_offset..compiled.buffer_offset + GAIJI_RECORD_BYTE_SIZE]
        .copy_from_slice(&[0xa5; GAIJI_RECORD_BYTE_SIZE]);

    assert_eq!(
        &mad_com[compiled.file_name_offset..file_name_end],
        b"GAIJI.COM\0"
    );
}

#[test]
fn wrapper_rejects_insufficient_owned_storage() {
    let compiled = compile_scene_transition(&catalog(), &fixed_runtime(), layout(149)).unwrap();
    let required = compiled.wrapper.bytes().len();

    assert!(compile_scene_transition(&catalog(), &fixed_runtime(), layout(required - 1)).is_err());
}

#[test]
fn status_bank_entry_preserves_registers_and_uses_its_prefix_count() {
    let compiled = compile_scene_transition(&catalog(), &fixed_runtime(), layout(149)).unwrap();
    let address = compiled.switch_status_bank_com_address;
    let entries = compiled.wrapper.placed_instructions();
    let start = entries
        .iter()
        .position(|p| p.location.off == address)
        .unwrap();
    assert!(matches!(entries[start].instruction, Instruction::Pushf));
    assert!(matches!(entries[start + 2].instruction, Instruction::Pusha));
    assert!(matches!(
        entries[start + 6].instruction,
        Instruction::Mov {
            dest: Operand::Reg16(Register16::CX),
            src: Operand::Imm16(26)
        }
    ));
    assert!(matches!(
        entries[start + 7].instruction,
        Instruction::Push {
            src: Operand::Reg16(Register16::CX)
        }
    ));
    assert!(entries.iter().any(|p| matches!(
        p.instruction,
        Instruction::Pop {
            dest: Operand::Reg16(Register16::SI)
        }
    )));
}
