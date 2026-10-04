use anyhow::{Result, ensure};
use v30::{Condition, Instruction, Operand, OperandSize, Register16};

#[path = "command_routes/typed_decode.rs"]
mod typed_decode;

use typed_decode::{
    ensure_branch, ensure_call, ensure_cmp_al_immediate, ensure_direct_byte_write,
    ensure_dos_text_output, ensure_process_exit, ensure_register_immediate, instruction_at,
    is_direct_memory,
};

const COMMAND_TAIL_LENGTH_ADDRESS: u16 = 0x0080;
const INVALID_PARAMETER_STATE_ADDRESS: u16 = 0x0f14;

pub(super) fn verify_mouse_driver_command_routes(bytes: &[u8]) -> Result<()> {
    verify_help_without_arguments(bytes)?;
    verify_parameter_ranges(bytes)?;
    verify_invalid_parameter_exit(bytes)?;
    verify_resident_reinvocation_unloads(bytes)?;
    Ok(())
}

fn verify_help_without_arguments(bytes: &[u8]) -> Result<()> {
    let empty_tail = instruction_at(bytes, 0x095e, "empty command-tail comparison")?;
    ensure!(
        matches!(
            empty_tail.instruction,
            Instruction::Cmp {
                a: Operand::Mem(memory),
                b: Operand::Imm8(0),
            } if is_direct_memory(memory, COMMAND_TAIL_LENGTH_ADDRESS, OperandSize::Byte)
        ),
        "NMOUSE.COM no longer distinguishes an empty command tail"
    );

    ensure_branch(
        bytes,
        0x0963,
        Condition::Ne,
        0x096d,
        "non-empty command tail",
    )?;
    ensure_call(bytes, 0x0965, 0x0a74, "help/status route")?;
    ensure_dos_text_output(bytes, 0x0a74, "help/status text")?;
    Ok(())
}

fn verify_parameter_ranges(bytes: &[u8]) -> Result<()> {
    ensure_cmp_al_immediate(bytes, 0x0b2a, b'A', "ASCII command mode")?;
    ensure_branch(bytes, 0x0b2c, Condition::E, 0x0b3b, "ASCII command mode")?;
    ensure_cmp_al_immediate(bytes, 0x0b30, b'N', "NEC command mode")?;
    ensure_branch(bytes, 0x0b32, Condition::E, 0x0b3b, "NEC command mode")?;
    ensure_direct_byte_write(
        bytes,
        0x0b34,
        INVALID_PARAMETER_STATE_ADDRESS,
        1,
        "invalid command mode",
    )?;

    ensure_cmp_al_immediate(bytes, 0x0b44, b'0', "minimum frequency")?;
    ensure_branch(bytes, 0x0b46, Condition::B, 0x0b4c, "frequency below zero")?;
    ensure_cmp_al_immediate(bytes, 0x0b48, b'3', "maximum frequency")?;
    ensure_branch(
        bytes,
        0x0b4a,
        Condition::Be,
        0x0b53,
        "frequency within zero through three",
    )?;
    ensure_direct_byte_write(
        bytes,
        0x0b4c,
        INVALID_PARAMETER_STATE_ADDRESS,
        2,
        "invalid frequency",
    )?;
    Ok(())
}

fn verify_invalid_parameter_exit(bytes: &[u8]) -> Result<()> {
    ensure_call(bytes, 0x09f9, 0x0afc, "parameter parser")?;

    let parsed = instruction_at(bytes, 0x09fc, "parameter parse result comparison")?;
    ensure!(
        matches!(
            parsed.instruction,
            Instruction::Cmp {
                a: Operand::Mem(memory),
                b: Operand::Imm8(0),
            } if is_direct_memory(memory, INVALID_PARAMETER_STATE_ADDRESS, OperandSize::Byte)
        ),
        "NMOUSE.COM no longer checks the parsed parameter result"
    );
    ensure_branch(
        bytes,
        0x0a01,
        Condition::E,
        0x0a0f,
        "valid parameter continuation",
    )?;
    ensure_dos_text_output(bytes, 0x0a03, "invalid parameter text")?;
    ensure_process_exit(bytes, 0x0a0a, 2, "invalid parameter exit")?;
    Ok(())
}

fn verify_resident_reinvocation_unloads(bytes: &[u8]) -> Result<()> {
    ensure_register_immediate(bytes, 0x099a, Register16::SI, 0x0106, "resident signature")?;
    ensure_register_immediate(bytes, 0x099d, Register16::DI, 0x0106, "resident handler")?;
    ensure_register_immediate(bytes, 0x09a4, Register16::CX, 10, "signature length")?;

    let comparison = instruction_at(bytes, 0x09a8, "resident signature comparison")?;
    ensure!(
        comparison.instruction == Instruction::Rep(Box::new(Instruction::Cmpsb)),
        "NMOUSE.COM no longer compares the resident signature"
    );
    ensure_branch(
        bytes,
        0x09aa,
        Condition::Ne,
        0x09e7,
        "not-resident installation route",
    )?;
    ensure_dos_text_output(bytes, 0x09db, "resident unload text")?;
    ensure_process_exit(bytes, 0x09e2, 0, "resident unload exit")?;
    Ok(())
}

#[cfg(test)]
#[path = "command_routes_tests.rs"]
mod command_routes_tests;
