use anyhow::{Context, Result, ensure};
use serde::Serialize;
use v30::{CallTarget, Instruction, Operand, Register16, decode_bytes};

const COM_ORIGIN: usize = 0x100;
const STARTUP_CALL_FILE_OFFSET: usize = 0x000d;
const STARTUP_CALL_TARGET_COM_ADDRESS: u16 = 0x2fa6;
const DIALOGUE_ROUTINE_FILE_OFFSET: usize = 0xaf4b;
const FAILURE_EXIT_FILE_OFFSET: usize = 0x0490;
const DOS_TERMINATE_FILE_OFFSET: usize = 0x04e0;
const CALL_SITES: [(usize, &str); 2] = [(0x0290, "new-game-dialogue"), (0x03cc, "ending-dialogue")];

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct MadSceneTransitionCatalog {
    pub startup_call: MadSceneTransitionCall,
    pub dialogue_routine_file_offset: usize,
    pub dialogue_routine_com_address: u16,
    pub failure_exit_file_offset: usize,
    pub failure_exit_com_address: u16,
    pub call_site_count: usize,
    pub calls: Vec<MadSceneTransitionCall>,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct MadSceneTransitionCall {
    pub id: String,
    pub file_offset: usize,
    pub com_address: u16,
    pub byte_size: usize,
    pub target_com_address: u16,
}

pub(super) fn catalog_mad_scene_transition(bytes: &[u8]) -> Result<MadSceneTransitionCatalog> {
    let startup_call = catalog_call(
        bytes,
        STARTUP_CALL_FILE_OFFSET,
        "startup-interface-bank",
        STARTUP_CALL_TARGET_COM_ADDRESS,
    )?;
    let dialogue_routine_com_address = com_address(DIALOGUE_ROUTINE_FILE_OFFSET)?;
    let failure_exit_com_address = com_address(FAILURE_EXIT_FILE_OFFSET)?;
    verify_failure_exit(bytes)?;
    let target_occurrences = bytes
        .windows(3)
        .enumerate()
        .filter(|(_, bytes)| bytes[0] == 0xe8)
        .map(|(offset, bytes)| {
            relative_call_target(offset, i16::from_le_bytes([bytes[1], bytes[2]]))
        })
        .collect::<Result<Vec<_>>>()?
        .into_iter()
        .filter_map(|(offset, target)| (target == dialogue_routine_com_address).then_some(offset))
        .collect::<Vec<_>>();
    ensure!(
        target_occurrences
            == CALL_SITES
                .iter()
                .map(|(offset, _)| *offset)
                .collect::<Vec<_>>(),
        "MAD dialogue entry-call population changed"
    );

    let calls = CALL_SITES
        .iter()
        .map(|&(file_offset, id)| {
            catalog_call(bytes, file_offset, id, dialogue_routine_com_address)
        })
        .collect::<Result<Vec<_>>>()?;

    Ok(MadSceneTransitionCatalog {
        startup_call,
        dialogue_routine_file_offset: DIALOGUE_ROUTINE_FILE_OFFSET,
        dialogue_routine_com_address,
        failure_exit_file_offset: FAILURE_EXIT_FILE_OFFSET,
        failure_exit_com_address,
        call_site_count: calls.len(),
        calls,
    })
}

fn catalog_call(
    bytes: &[u8],
    file_offset: usize,
    id: &str,
    expected_target: u16,
) -> Result<MadSceneTransitionCall> {
    let decoded = decode_bytes(
        bytes
            .get(file_offset..)
            .with_context(|| format!("MAD scene transition {id} lies outside MAD.COM"))?,
    )?;
    let target = match decoded.instruction {
        Instruction::Call {
            target: CallTarget::Rel16(displacement),
        } if decoded.byte_len == 3 && decoded.prefixes.is_empty() => {
            relative_call_target(file_offset, displacement)?.1
        }
        _ => anyhow::bail!("MAD scene transition {id} is not the verified typed V30 near CALL"),
    };
    ensure!(
        target == expected_target,
        "MAD scene transition {id} targets {target:#06x}, expected {expected_target:#06x}"
    );
    Ok(MadSceneTransitionCall {
        id: id.to_owned(),
        file_offset,
        com_address: com_address(file_offset)?,
        byte_size: decoded.byte_len,
        target_com_address: target,
    })
}

fn verify_failure_exit(bytes: &[u8]) -> Result<()> {
    let load = decode_bytes(
        bytes
            .get(DOS_TERMINATE_FILE_OFFSET..)
            .context("MAD DOS termination load lies outside MAD.COM")?,
    )?;
    ensure!(
        matches!(
            load.instruction,
            Instruction::Mov {
                dest: Operand::Reg16(Register16::AX),
                src: Operand::Imm16(0x4c00),
            }
        ) && load.byte_len == 3
            && load.prefixes.is_empty(),
        "MAD failure exit no longer loads the verified DOS termination status"
    );
    let terminate = decode_bytes(
        bytes
            .get(DOS_TERMINATE_FILE_OFFSET + load.byte_len..)
            .context("MAD DOS termination interrupt lies outside MAD.COM")?,
    )?;
    ensure!(
        matches!(terminate.instruction, Instruction::Int { vector: 0x21 })
            && terminate.byte_len == 2
            && terminate.prefixes.is_empty(),
        "MAD failure exit no longer ends in the verified DOS interrupt"
    );
    Ok(())
}

fn relative_call_target(file_offset: usize, displacement: i16) -> Result<(usize, u16)> {
    let next = com_address(
        file_offset
            .checked_add(3)
            .context("MAD scene transition address overflow")?,
    )?;
    Ok((file_offset, next.wrapping_add_signed(displacement)))
}

fn com_address(file_offset: usize) -> Result<u16> {
    Ok(u16::try_from(file_offset + COM_ORIGIN)?)
}

#[cfg(test)]
#[path = "mad_scene_transition_tests.rs"]
mod mad_scene_transition_tests;
