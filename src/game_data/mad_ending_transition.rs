use anyhow::{Context, Result, ensure};
use serde::Serialize;
use v30::{
    CallTarget, Condition, EffectiveAddressBase, EffectiveAddressDisplacement, Instruction,
    JmpTarget, Operand, OperandSize, Prefix, Register16, SegmentRegister, ShiftCount, decode_bytes,
};

use super::{DialogueGroup, MadSceneTransitionCatalog};

const SELECTOR_REFRESH_CALL_FILE_OFFSET: usize = 0x037b;
const SELECTOR_ROUTINE_FILE_OFFSET: usize = 0x044e;
const ENDING_SELECTOR_COMPARE_FILE_OFFSET: usize = 0x0386;
const ENDING_SELECTOR_SKIP_BRANCH_FILE_OFFSET: usize = 0x038c;
const ENDING_BRANCH_FILE_OFFSET: usize = 0x038e;
const ENDING_PRELUDE_FILE_OFFSET: usize = 0x03ba;
const ENDING_STAGE_GROUP_WRITE_FILE_OFFSET: usize = 0x03c6;
const ENDING_DIALOGUE_CALL_FILE_OFFSET: usize = 0x03cc;
const COMPLETION_STAGE_GROUP_COMPARE_FILE_OFFSET: usize = 0x0467;
const COMPLETION_STAGE_GROUP_SKIP_BRANCH_FILE_OFFSET: usize = 0x046d;
const ENDING_SELECTOR_WRITE_FILE_OFFSET: usize = 0x046f;
const SELECTOR_ROUTINE_RETURN_FILE_OFFSET: usize = 0x0475;
const DIALOGUE_GROUP_LOAD_FILE_OFFSET: usize = 0xaf5f;
const DIALOGUE_GROUP_SCALE_FILE_OFFSET: usize = 0xaf62;
const STAGE_GROUP_COM_ADDRESS: u16 = 0xdb8d;
const ENDING_SELECTOR_COM_ADDRESS: u16 = 0xd79e;
const COMPLETION_STAGE_GROUP: u16 = 9;
const ENDING_STAGE_GROUP: u16 = 10;
const ENDING_SELECTOR_VALUE: u8 = 0x80;

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct MadEndingTransitionCatalog {
    pub selector_refresh_call_file_offset: usize,
    pub selector_routine_file_offset: usize,
    pub ending_branch_file_offset: usize,
    pub ending_dialogue_call_file_offset: usize,
    pub stage_group_com_address: u16,
    pub completion_stage_group: u16,
    pub ending_stage_group: u16,
    pub ending_selector_com_address: u16,
    pub ending_selector_value: u8,
    pub ending_dialogue_group_number: usize,
    pub ending_dialogue_group_id: String,
}

pub(super) fn catalog_mad_ending_transition(
    bytes: &[u8],
    scene: &MadSceneTransitionCatalog,
    dialogue_groups: &[DialogueGroup],
) -> Result<MadEndingTransitionCatalog> {
    verify_selector_refresh_call(bytes)?;
    verify_ending_selector_branch(bytes)?;
    verify_completion_group_selection(bytes)?;
    verify_dialogue_group_indexing(bytes)?;

    let ending_call = scene
        .calls
        .iter()
        .find(|call| call.id == "ending-dialogue")
        .context("MAD scene transition has no ending-dialogue caller")?;
    ensure!(
        ending_call.file_offset == ENDING_DIALOGUE_CALL_FILE_OFFSET,
        "MAD ending dialogue caller moved away from the verified transition"
    );

    let ending_dialogue_group_index = usize::from(ENDING_STAGE_GROUP);
    let ending_dialogue_group = dialogue_groups
        .get(ending_dialogue_group_index)
        .context("MAD ending stage group lies outside the dialogue population")?;
    let ending_dialogue_group_number = ending_dialogue_group_index + 1;
    ensure!(
        ending_dialogue_group.id == format!("dialogue-group-{ending_dialogue_group_number:02}"),
        "MAD ending dialogue group identity no longer matches its zero-based selector"
    );

    Ok(MadEndingTransitionCatalog {
        selector_refresh_call_file_offset: SELECTOR_REFRESH_CALL_FILE_OFFSET,
        selector_routine_file_offset: SELECTOR_ROUTINE_FILE_OFFSET,
        ending_branch_file_offset: ENDING_BRANCH_FILE_OFFSET,
        ending_dialogue_call_file_offset: ENDING_DIALOGUE_CALL_FILE_OFFSET,
        stage_group_com_address: STAGE_GROUP_COM_ADDRESS,
        completion_stage_group: COMPLETION_STAGE_GROUP,
        ending_stage_group: ENDING_STAGE_GROUP,
        ending_selector_com_address: ENDING_SELECTOR_COM_ADDRESS,
        ending_selector_value: ENDING_SELECTOR_VALUE,
        ending_dialogue_group_number,
        ending_dialogue_group_id: ending_dialogue_group.id.clone(),
    })
}

fn verify_selector_refresh_call(bytes: &[u8]) -> Result<()> {
    let decoded = instruction_at(
        bytes,
        SELECTOR_REFRESH_CALL_FILE_OFFSET,
        "selector refresh call",
    )?;
    let displacement = match decoded.instruction {
        Instruction::Call {
            target: CallTarget::Rel16(displacement),
        } => displacement,
        _ => anyhow::bail!("MAD main loop no longer calls the ending selector routine"),
    };
    ensure!(
        relative_file_target(
            SELECTOR_REFRESH_CALL_FILE_OFFSET,
            decoded.byte_len,
            displacement,
        )? == SELECTOR_ROUTINE_FILE_OFFSET,
        "MAD main loop selector refresh call changed target"
    );
    Ok(())
}

fn verify_ending_selector_branch(bytes: &[u8]) -> Result<()> {
    let compare = instruction_at(
        bytes,
        ENDING_SELECTOR_COMPARE_FILE_OFFSET,
        "ending selector comparison",
    )?;
    ensure!(
        matches!(
            compare.instruction,
            Instruction::Cmp {
                a: Operand::Mem(memory),
                b: Operand::Imm8(ENDING_SELECTOR_VALUE),
            } if is_direct_memory(
                memory,
                ENDING_SELECTOR_COM_ADDRESS,
                OperandSize::Byte,
                Some(SegmentRegister::CS),
            )
        ) && compare.prefixes.as_slice() == [Prefix::Segment(SegmentRegister::CS)],
        "MAD main loop no longer compares the verified ending selector"
    );

    let skip = instruction_at(
        bytes,
        ENDING_SELECTOR_SKIP_BRANCH_FILE_OFFSET,
        "ending selector skip branch",
    )?;
    let skip_displacement = match skip.instruction {
        Instruction::Jcc {
            cond: Condition::Ne,
            target,
        } => i16::from(target),
        _ => anyhow::bail!("MAD main loop no longer skips the ending route on a non-match"),
    };
    ensure!(
        relative_file_target(
            ENDING_SELECTOR_SKIP_BRANCH_FILE_OFFSET,
            skip.byte_len,
            skip_displacement,
        )? == ENDING_BRANCH_FILE_OFFSET + 3,
        "MAD ending selector non-match branch changed target"
    );

    let branch = instruction_at(bytes, ENDING_BRANCH_FILE_OFFSET, "ending route branch")?;
    let displacement = match branch.instruction {
        Instruction::Jmp {
            target: JmpTarget::Rel8(displacement),
        } => i16::from(displacement),
        _ => anyhow::bail!("MAD ending route no longer uses the verified short jump"),
    };
    ensure!(
        relative_file_target(ENDING_BRANCH_FILE_OFFSET, branch.byte_len, displacement)?
            == ENDING_PRELUDE_FILE_OFFSET,
        "MAD ending selector no longer enters the verified transition prelude"
    );
    Ok(())
}

fn verify_completion_group_selection(bytes: &[u8]) -> Result<()> {
    let compare = instruction_at(
        bytes,
        COMPLETION_STAGE_GROUP_COMPARE_FILE_OFFSET,
        "completion stage comparison",
    )?;
    ensure!(
        matches!(
            compare.instruction,
            Instruction::Cmp {
                a: Operand::Mem(memory),
                b: Operand::Imm16(COMPLETION_STAGE_GROUP),
            } if is_direct_memory(
                memory,
                STAGE_GROUP_COM_ADDRESS,
                OperandSize::Word,
                Some(SegmentRegister::CS),
            )
        ) && compare.prefixes.as_slice() == [Prefix::Segment(SegmentRegister::CS)],
        "MAD selector routine no longer checks completion stage group 9"
    );

    let skip = instruction_at(
        bytes,
        COMPLETION_STAGE_GROUP_SKIP_BRANCH_FILE_OFFSET,
        "completion group skip branch",
    )?;
    let skip_displacement = match skip.instruction {
        Instruction::Jcc {
            cond: Condition::Ne,
            target,
        } => i16::from(target),
        _ => anyhow::bail!("MAD selector routine no longer guards the ending selector write"),
    };
    ensure!(
        relative_file_target(
            COMPLETION_STAGE_GROUP_SKIP_BRANCH_FILE_OFFSET,
            skip.byte_len,
            skip_displacement,
        )? == SELECTOR_ROUTINE_RETURN_FILE_OFFSET,
        "MAD completion-group non-match branch no longer returns without selecting the ending"
    );

    let selector_write = instruction_at(
        bytes,
        ENDING_SELECTOR_WRITE_FILE_OFFSET,
        "ending selector write",
    )?;
    ensure!(
        matches!(
            selector_write.instruction,
            Instruction::Mov {
                dest: Operand::Mem(memory),
                src: Operand::Imm8(ENDING_SELECTOR_VALUE),
            } if is_direct_memory(
                memory,
                ENDING_SELECTOR_COM_ADDRESS,
                OperandSize::Byte,
                Some(SegmentRegister::CS),
            )
        ) && selector_write.prefixes.as_slice() == [Prefix::Segment(SegmentRegister::CS)],
        "MAD selector routine no longer writes the verified ending selector"
    );

    let ending_group_write = instruction_at(
        bytes,
        ENDING_STAGE_GROUP_WRITE_FILE_OFFSET,
        "ending stage group write",
    )?;
    ensure!(
        matches!(
            ending_group_write.instruction,
            Instruction::Mov {
                dest: Operand::Mem(memory),
                src: Operand::Imm16(ENDING_STAGE_GROUP),
            } if is_direct_memory(
                memory,
                STAGE_GROUP_COM_ADDRESS,
                OperandSize::Word,
                None,
            )
        ) && ending_group_write.prefixes.is_empty(),
        "MAD ending transition no longer advances the dialogue stage group to 10"
    );
    ensure!(
        ENDING_STAGE_GROUP_WRITE_FILE_OFFSET + ending_group_write.byte_len
            == ENDING_DIALOGUE_CALL_FILE_OFFSET,
        "MAD ending stage group write no longer immediately precedes its dialogue caller"
    );
    Ok(())
}

fn verify_dialogue_group_indexing(bytes: &[u8]) -> Result<()> {
    let load = instruction_at(
        bytes,
        DIALOGUE_GROUP_LOAD_FILE_OFFSET,
        "dialogue group load",
    )?;
    ensure!(
        matches!(
            load.instruction,
            Instruction::Mov {
                dest: Operand::Reg16(Register16::AX),
                src: Operand::Mem(memory),
            } if is_direct_memory(
                memory,
                STAGE_GROUP_COM_ADDRESS,
                OperandSize::Word,
                None,
            )
        ) && load.prefixes.is_empty(),
        "MAD dialogue consumer no longer loads the zero-based stage group"
    );

    let scale = instruction_at(
        bytes,
        DIALOGUE_GROUP_SCALE_FILE_OFFSET,
        "dialogue group scale",
    )?;
    ensure!(
        matches!(
            scale.instruction,
            Instruction::Shl {
                dest: Operand::Reg16(Register16::AX),
                count: ShiftCount::One,
            }
        ) && scale.prefixes.is_empty(),
        "MAD dialogue consumer no longer scales the group index for its word pointer table"
    );
    Ok(())
}

fn instruction_at(bytes: &[u8], offset: usize, role: &str) -> Result<v30::DecodedInstruction> {
    decode_bytes(
        bytes
            .get(offset..)
            .with_context(|| format!("MAD {role} lies outside MAD.COM"))?,
    )
    .with_context(|| format!("MAD {role} is not typed V30 code at file offset {offset:#x}"))
}

fn is_direct_memory(
    memory: v30::EffectiveAddress,
    address: u16,
    size: OperandSize,
    segment: Option<SegmentRegister>,
) -> bool {
    memory.base() == EffectiveAddressBase::Direct
        && memory.displacement() == EffectiveAddressDisplacement::Absolute(address)
        && memory.size() == size
        && memory.segment() == segment
}

fn relative_file_target(offset: usize, byte_len: usize, displacement: i16) -> Result<usize> {
    (offset + byte_len)
        .checked_add_signed(isize::from(displacement))
        .context("MAD ending transition target lies outside MAD.COM")
}

#[cfg(test)]
#[path = "mad_ending_transition_tests.rs"]
mod mad_ending_transition_tests;
