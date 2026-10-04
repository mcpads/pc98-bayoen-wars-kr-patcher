use anyhow::{Context, Result, ensure};
use v30::{
    CallTarget, Condition, DecodedInstruction, EffectiveAddress, EffectiveAddressBase,
    EffectiveAddressDisplacement, Instruction, Operand, OperandSize, Register8, Register16,
    decode_bytes,
};

const COM_LOAD_ADDRESS: usize = 0x0100;

pub(super) fn instruction_at(
    bytes: &[u8],
    com_address: usize,
    role: &str,
) -> Result<DecodedInstruction> {
    let file_offset = com_address
        .checked_sub(COM_LOAD_ADDRESS)
        .context("NMOUSE.COM address lies before the COM load address")?;
    decode_bytes(
        bytes
            .get(file_offset..)
            .with_context(|| format!("NMOUSE.COM {role} lies outside the file"))?,
    )
    .with_context(|| format!("NMOUSE.COM {role} is not typed V30 code"))
}

pub(super) fn ensure_call(bytes: &[u8], address: usize, target: usize, role: &str) -> Result<()> {
    let call = instruction_at(bytes, address, role)?;
    let displacement = match call.instruction {
        Instruction::Call {
            target: CallTarget::Rel16(displacement),
        } => displacement,
        _ => anyhow::bail!("NMOUSE.COM {role} is no longer a near call"),
    };
    ensure!(
        relative_target(address, call.byte_len, displacement)? == target,
        "NMOUSE.COM {role} changed target"
    );
    Ok(())
}

pub(super) fn ensure_branch(
    bytes: &[u8],
    address: usize,
    condition: Condition,
    target: usize,
    role: &str,
) -> Result<()> {
    let branch = instruction_at(bytes, address, role)?;
    let displacement = match branch.instruction {
        Instruction::Jcc {
            cond,
            target: displacement,
        } if cond == condition => i16::from(displacement),
        _ => anyhow::bail!("NMOUSE.COM {role} changed condition"),
    };
    ensure!(
        relative_target(address, branch.byte_len, displacement)? == target,
        "NMOUSE.COM {role} changed target"
    );
    Ok(())
}

pub(super) fn ensure_cmp_al_immediate(
    bytes: &[u8],
    address: usize,
    value: u8,
    role: &str,
) -> Result<()> {
    let comparison = instruction_at(bytes, address, role)?;
    ensure!(
        comparison.instruction
            == (Instruction::Cmp {
                a: Operand::Reg8(Register8::AL),
                b: Operand::Imm8(value),
            }),
        "NMOUSE.COM {role} comparison changed"
    );
    Ok(())
}

pub(super) fn ensure_direct_byte_write(
    bytes: &[u8],
    address: usize,
    target: u16,
    value: u8,
    role: &str,
) -> Result<()> {
    let write = instruction_at(bytes, address, role)?;
    ensure!(
        matches!(
            write.instruction,
            Instruction::Mov {
                dest: Operand::Mem(memory),
                src: Operand::Imm8(actual),
            } if actual == value && is_direct_memory(memory, target, OperandSize::Byte)
        ),
        "NMOUSE.COM {role} no longer records the expected parse result"
    );
    Ok(())
}

pub(super) fn ensure_register_immediate(
    bytes: &[u8],
    address: usize,
    register: Register16,
    value: u16,
    role: &str,
) -> Result<()> {
    let load = instruction_at(bytes, address, role)?;
    ensure!(
        load.instruction
            == (Instruction::Mov {
                dest: Operand::Reg16(register),
                src: Operand::Imm16(value),
            }),
        "NMOUSE.COM {role} changed"
    );
    Ok(())
}

pub(super) fn ensure_dos_text_output(bytes: &[u8], address: usize, role: &str) -> Result<()> {
    let text = instruction_at(bytes, address, role)?;
    ensure!(
        matches!(
            text.instruction,
            Instruction::Mov {
                dest: Operand::Reg16(Register16::DX),
                src: Operand::Imm16(_),
            }
        ),
        "NMOUSE.COM {role} no longer loads a DOS string"
    );
    let function = instruction_at(bytes, address + text.byte_len, role)?;
    ensure!(
        function.instruction
            == (Instruction::Mov {
                dest: Operand::Reg8(Register8::AH),
                src: Operand::Imm8(9),
            }),
        "NMOUSE.COM {role} no longer selects DOS string output"
    );
    let interrupt = instruction_at(bytes, address + text.byte_len + function.byte_len, role)?;
    ensure!(
        interrupt.instruction == Instruction::Int { vector: 0x21 },
        "NMOUSE.COM {role} no longer calls DOS"
    );
    Ok(())
}

pub(super) fn ensure_process_exit(
    bytes: &[u8],
    address: usize,
    code: u8,
    role: &str,
) -> Result<()> {
    let exit = instruction_at(bytes, address, role)?;
    ensure!(
        exit.instruction
            == (Instruction::Mov {
                dest: Operand::Reg16(Register16::AX),
                src: Operand::Imm16(0x4c00 | u16::from(code)),
            }),
        "NMOUSE.COM {role} changed status"
    );
    let interrupt = instruction_at(bytes, address + exit.byte_len, role)?;
    ensure!(
        interrupt.instruction == Instruction::Int { vector: 0x21 },
        "NMOUSE.COM {role} no longer returns through DOS"
    );
    Ok(())
}

pub(super) fn is_direct_memory(memory: EffectiveAddress, address: u16, size: OperandSize) -> bool {
    memory.segment().is_none()
        && memory.base() == EffectiveAddressBase::Direct
        && memory.displacement() == EffectiveAddressDisplacement::Absolute(address)
        && memory.size() == size
}

fn relative_target(address: usize, byte_len: usize, displacement: i16) -> Result<usize> {
    (address + byte_len)
        .checked_add_signed(isize::from(displacement))
        .context("NMOUSE.COM relative target lies outside the program")
}
