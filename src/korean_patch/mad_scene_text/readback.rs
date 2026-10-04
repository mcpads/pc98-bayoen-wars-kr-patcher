use std::collections::{BTreeMap, BTreeSet};

use anyhow::{Context, Result, ensure};
use v30::{CallTarget, Instruction, decode_bytes};

use super::super::dialogue_text::verify_dialogue_text_records_and_references;
use super::super::fixed_gaiji_text::verify_fixed_gaiji_text_component;
use super::super::gaiji_record::{GAIJI_RECORD_SIZE, GaijiRecord};
use super::super::interface_text::verify_interface_text_records_and_references;
use super::super::mad_system_text::verify_mad_system_text_records_and_references;
use super::super::shared_text::verify_gaiji_bank_prefix_readback;
use super::compile::CompiledMadSceneText;
use crate::game_data::GameDataCatalog;

pub(super) fn verify_mad_scene_text_payload(
    files: &std::collections::BTreeMap<String, Vec<u8>>,
    source: &GameDataCatalog,
    compiled: &CompiledMadSceneText,
) -> Result<()> {
    let gaiji_com = files
        .get("GAIJI.COM")
        .context("MAD scene payload is missing GAIJI.COM")?;
    verify_gaiji_bank_prefix_readback(gaiji_com, &source.gaiji, &compiled.shared.bank)?;
    let startup_gaiji = crate::game_data::parse_gaiji_program(
        gaiji_com
            .get(..compiled.shared.bank.patched_program.len())
            .context("MAD scene startup GAIJI bank lies outside GAIJI.COM")?,
    )?;
    let mad_com = files
        .get("MAD.COM")
        .context("MAD scene payload is missing MAD.COM")?;
    verify_mad_system_text_records_and_references(
        mad_com,
        &source.mad_com.system_runtime,
        &compiled.shared.system,
    )?;
    verify_interface_text_records_and_references(
        mad_com,
        &startup_gaiji,
        &source.mad_com.interface_text.reference_catalog,
        &compiled.shared.interface,
    )?;
    verify_dialogue_text_records_and_references(
        mad_com,
        &source.mad_com.dialogue.reference_catalog,
        &compiled.dialogue,
    )?;
    verify_bank_file(files, compiled)?;
    verify_fixed_gaiji_text_component(
        mad_com,
        &compiled.fixed_bank_file.records,
        &source.gaiji,
        &compiled.fixed,
    )?;
    verify_battle_callouts(mad_com, &source.mad_com.battle_callouts, compiled)?;
    super::super::unit_list_status::verify_unit_list_status(mad_com, &compiled.unit_list_status)?;
    verify_transition(mad_com, &source.mad_com.scene_transition, compiled)
}

fn verify_bank_file(
    files: &std::collections::BTreeMap<String, Vec<u8>>,
    compiled: &CompiledMadSceneText,
) -> Result<()> {
    let gaiji_com = files
        .get("GAIJI.COM")
        .context("MAD scene payload is missing GAIJI.COM")?;
    ensure!(
        gaiji_com.len() == compiled.battle_bank_file.output_file_size,
        "MAD scene GAIJI.COM output size failed readback"
    );
    ensure!(
        gaiji_com.get(
            compiled.dialogue_bank_file.file_offset - compiled.interface_extension_records.len()
                ..compiled.dialogue_bank_file.file_offset
        ) == Some(compiled.interface_extension_records.as_slice()),
        "MAD scene interface GAIJI extension failed readback"
    );
    ensure!(
        gaiji_com.get(
            compiled.dialogue_bank_file.file_offset..compiled.dialogue_bank_file.output_file_size
        ) == Some(compiled.dialogue_bank_file.records.as_slice()),
        "MAD scene dialogue GAIJI bank failed readback"
    );
    ensure!(
        compiled
            .dialogue_bank_file
            .records
            .as_chunks::<GAIJI_RECORD_SIZE>()
            .0
            .iter()
            .all(|bytes| GaijiRecord::parse(bytes).is_ok()),
        "MAD scene dialogue GAIJI bank contains an invalid record"
    );
    ensure!(
        gaiji_com
            .get(compiled.fixed_bank_file.file_offset..compiled.fixed_bank_file.output_file_size)
            == Some(compiled.fixed_bank_file.records.as_slice()),
        "MAD scene fixed-text GAIJI bank failed readback"
    );
    ensure!(
        compiled
            .fixed_bank_file
            .records
            .as_chunks::<GAIJI_RECORD_SIZE>()
            .0
            .iter()
            .all(|bytes| GaijiRecord::parse(bytes).is_ok()),
        "MAD scene fixed-text GAIJI bank contains an invalid record"
    );
    ensure!(
        gaiji_com
            .get(compiled.battle_bank_file.file_offset..compiled.battle_bank_file.output_file_size)
            == Some(compiled.battle_bank_file.records.as_slice()),
        "MAD scene battle-callout GAIJI bank failed readback"
    );
    ensure!(
        compiled
            .battle_bank_file
            .records
            .as_chunks::<GAIJI_RECORD_SIZE>()
            .0
            .iter()
            .all(|bytes| GaijiRecord::parse(bytes).is_ok()),
        "MAD scene battle-callout GAIJI bank contains an invalid record"
    );
    Ok(())
}

fn verify_battle_callouts(
    mad_com: &[u8],
    source: &crate::game_data::BattleCalloutCatalog,
    compiled: &CompiledMadSceneText,
) -> Result<()> {
    let battle = &compiled.battle;
    let pointer_table_end = battle.pointer_table_offset + battle.pointer_table_replacement.len();
    ensure!(
        mad_com.get(battle.pointer_table_offset..pointer_table_end)
            == Some(battle.pointer_table_replacement.as_slice()),
        "battle callout pointer table failed readback"
    );
    let packed_end = battle.text_region_start + battle.packed_storage_bytes;
    ensure!(
        mad_com.get(battle.text_region_start..packed_end)
            == Some(&battle.text_region_replacement[..battle.packed_storage_bytes]),
        "packed battle callout text failed readback"
    );
    for entry in &battle.entries {
        ensure!(
            mad_com.get(entry.file_offset..entry.file_offset + entry.bytes.len())
                == Some(entry.bytes.as_slice()),
            "{} battle callout bytes failed readback",
            entry.id
        );
        ensure!(
            entry.bytes.ends_with(b"$$")
                && entry
                    .bytes
                    .windows(2)
                    .filter(|bytes| *bytes == b"$0")
                    .count()
                    + 1
                    == entry.lines.len(),
            "{} battle callout controls changed",
            entry.id
        );
    }
    let targets = battle
        .entries
        .iter()
        .map(|entry| (entry.id.as_str(), entry.com_address))
        .collect::<BTreeMap<_, _>>();
    let mut verified_pointer_offsets = BTreeSet::new();
    for pointer in &source.pointers {
        let bytes: [u8; 2] = mad_com
            .get(pointer.storage_offset..pointer.storage_offset + 2)
            .with_context(|| format!("{} lies outside MAD.COM", pointer.id))?
            .try_into()
            .expect("two bytes convert to a word");
        let actual = u16::from_le_bytes(bytes);
        let expected = targets
            .get(pointer.target_entry_id.as_str())
            .copied()
            .with_context(|| format!("missing compiled {}", pointer.target_entry_id))?;
        ensure!(
            actual == expected,
            "{} targets {actual:#06x}, expected {expected:#06x}",
            pointer.id
        );
        ensure!(
            verified_pointer_offsets.insert(pointer.storage_offset),
            "battle callout pointer storage was verified twice"
        );
    }
    ensure!(
        verified_pointer_offsets.len() == source.pointer_count,
        "battle callout pointer readback population changed"
    );

    let hook = &compiled.battle_hook;
    ensure!(
        mad_com.get(hook.wrapper_offset..hook.wrapper_offset + hook.wrapper.bytes().len())
            == Some(hook.wrapper.bytes()),
        "battle callout bank wrapper failed readback"
    );
    let tail_start = hook.wrapper_offset + hook.wrapper.bytes().len();
    let tail_end = hook.wrapper_offset + hook.wrapper_capacity;
    ensure!(
        mad_com
            .get(tail_start..tail_end)
            .context("battle callout storage tail lies outside MAD.COM")?
            .iter()
            .all(|byte| *byte == 0),
        "battle callout storage tail padding failed readback"
    );
    let wrapper_address = u16::try_from(hook.wrapper_offset + 0x100)?;
    let mut verified_calls = 0;
    for call in &source.calls {
        let decoded = decode_bytes(
            mad_com
                .get(call.file_offset..)
                .with_context(|| format!("{} lies outside MAD.COM", call.id))?,
        )?;
        let target = match decoded.instruction {
            Instruction::Call {
                target: CallTarget::Rel16(displacement),
            } if decoded.byte_len == call.byte_size && decoded.prefixes.is_empty() => call
                .com_address
                .wrapping_add(u16::try_from(decoded.byte_len)?)
                .wrapping_add_signed(displacement),
            _ => anyhow::bail!("{} failed typed V30 readback", call.id),
        };
        ensure!(
            target == wrapper_address,
            "{} targets {target:#06x}, expected {wrapper_address:#06x}",
            call.id
        );
        verified_calls += 1;
    }
    ensure!(
        verified_calls == source.call_site_count,
        "battle callout hook readback population changed"
    );
    Ok(())
}

fn verify_transition(
    mad_com: &[u8],
    scene: &crate::game_data::MadSceneTransitionCatalog,
    compiled: &CompiledMadSceneText,
) -> Result<()> {
    let transition = &compiled.transition;
    ensure!(
        mad_com.get(
            transition.wrapper_offset..transition.wrapper_offset + transition.wrapper.bytes().len()
        ) == Some(transition.wrapper.bytes()),
        "MAD scene bank wrapper failed readback"
    );
    ensure!(
        mad_com.get(
            transition.file_name_offset..transition.file_name_offset + transition.file_name.len()
        ) == Some(transition.file_name.as_slice()),
        "MAD scene bank file name failed readback"
    );
    let tail_start = transition.wrapper_offset + transition.wrapper.bytes().len();
    let tail_end = transition.wrapper_offset + transition.wrapper_capacity;
    ensure!(
        mad_com
            .get(tail_start..tail_end)
            .context("MAD scene dialogue tail lies outside MAD.COM")?
            .iter()
            .all(|byte| *byte == 0),
        "MAD scene dialogue tail padding failed readback"
    );
    let buffer_tail_end = transition.buffer_offset + GAIJI_RECORD_SIZE;
    ensure!(
        mad_com
            .get(transition.buffer_offset..buffer_tail_end)
            .context("MAD scene bank buffer lies outside MAD.COM")?
            .iter()
            .all(|byte| *byte == 0),
        "MAD scene bank buffer is not zero-initialized"
    );
    let startup = decode_bytes(
        mad_com
            .get(scene.startup_call.file_offset..)
            .context("MAD startup interface-bank call lies outside MAD.COM")?,
    )?;
    let startup_target = match startup.instruction {
        Instruction::Call {
            target: CallTarget::Rel16(displacement),
        } if startup.byte_len == scene.startup_call.byte_size && startup.prefixes.is_empty() => {
            scene
                .startup_call
                .com_address
                .wrapping_add(u16::try_from(startup.byte_len)?)
                .wrapping_add_signed(displacement)
        }
        _ => anyhow::bail!("MAD startup interface-bank call failed typed V30 readback"),
    };
    ensure!(
        startup_target == transition.startup_entry_com_address,
        "MAD startup call targets {startup_target:#06x}, expected {:#06x}",
        transition.startup_entry_com_address
    );
    let wrapper_address = u16::try_from(transition.wrapper_offset + 0x100)?;
    for call in &scene.calls {
        let decoded = decode_bytes(
            mad_com
                .get(call.file_offset..)
                .with_context(|| format!("MAD scene call {} lies outside MAD.COM", call.id))?,
        )?;
        let target = match decoded.instruction {
            Instruction::Call {
                target: CallTarget::Rel16(displacement),
            } if decoded.byte_len == call.byte_size && decoded.prefixes.is_empty() => call
                .com_address
                .wrapping_add(u16::try_from(decoded.byte_len)?)
                .wrapping_add_signed(displacement),
            _ => anyhow::bail!("MAD scene call {} failed typed V30 readback", call.id),
        };
        ensure!(
            target == wrapper_address,
            "MAD scene call {} targets {target:#06x}, expected {wrapper_address:#06x}",
            call.id
        );
    }
    let fixed_entry = &compiled.transition.selection_entry_source;
    let decoded = decode_bytes(
        mad_com
            .get(source_selection_call_offset(compiled)..)
            .context("MAD selection entry call lies outside MAD.COM")?,
    )?;
    let target = match decoded.instruction {
        Instruction::Call {
            target: CallTarget::Rel16(displacement),
        } if decoded.byte_len == fixed_entry.bytes().len() && decoded.prefixes.is_empty() => {
            u16::try_from(source_selection_call_offset(compiled) + 0x100 + decoded.byte_len)?
                .wrapping_add_signed(displacement)
        }
        _ => anyhow::bail!("MAD selection entry call failed typed V30 readback"),
    };
    ensure!(
        target == transition.fixed_entry_com_address,
        "MAD selection entry call targets {target:#06x}, expected {:#06x}",
        transition.fixed_entry_com_address
    );
    Ok(())
}

fn source_selection_call_offset(compiled: &CompiledMadSceneText) -> usize {
    compiled.transition.selection_entry_source.origin().off as usize - 0x100
}
