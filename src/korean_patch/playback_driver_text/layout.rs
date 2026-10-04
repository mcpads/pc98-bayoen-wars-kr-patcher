use anyhow::{Context, Result, ensure};

use super::model::{CompiledPlaybackDriverTextEntry, TextStorageWrite};
use crate::external_text::ExternalProgramTextCatalog;

const COM_ORIGIN: usize = 0x100;
const DOS_TERMINATOR: u8 = b'$';

struct Slot {
    id: String,
    offset: usize,
    capacity: usize,
    used: usize,
    expected_original: Vec<u8>,
    replacement: Vec<u8>,
}

pub(super) fn place_records_in_source_storage(
    unpacked: &[u8],
    source: &ExternalProgramTextCatalog,
    entries: &mut [CompiledPlaybackDriverTextEntry],
) -> Result<(Vec<TextStorageWrite>, usize, usize)> {
    let mut source_slots = source
        .entries
        .iter()
        .map(|entry| {
            let capacity = entry.byte_size + 1;
            let expected_original = unpacked
                .get(entry.text_offset..entry.text_offset + capacity)
                .with_context(|| format!("{} source storage lies outside the image", entry.id))?
                .to_vec();
            ensure!(
                expected_original.last() == Some(&DOS_TERMINATOR),
                "{} source storage lost its DOS terminator",
                entry.id
            );
            Ok(Slot {
                id: entry.id.clone(),
                offset: entry.text_offset,
                capacity,
                used: 0,
                expected_original,
                replacement: vec![DOS_TERMINATOR; capacity],
            })
        })
        .collect::<Result<Vec<_>>>()?;
    source_slots.sort_by_key(|slot| slot.offset);
    ensure!(
        source_slots
            .windows(2)
            .all(|pair| pair[0].offset + pair[0].capacity <= pair[1].offset),
        "{} source text slots overlap",
        source.file_name
    );
    let mut slots: Vec<Slot> = Vec::new();
    for slot in source_slots {
        if let Some(previous) = slots.last_mut()
            && previous.offset + previous.capacity == slot.offset
        {
            previous.id.push('+');
            previous.id.push_str(&slot.id);
            previous.capacity += slot.capacity;
            previous
                .expected_original
                .extend_from_slice(&slot.expected_original);
            previous.replacement.extend_from_slice(&slot.replacement);
        } else {
            slots.push(slot);
        }
    }

    let mut placement_order = (0..entries.len()).collect::<Vec<_>>();
    placement_order.sort_by(|left, right| {
        entries[*right]
            .bytes
            .len()
            .cmp(&entries[*left].bytes.len())
            .then_with(|| {
                entries[*left]
                    .source_order
                    .cmp(&entries[*right].source_order)
            })
    });
    for entry_index in placement_order {
        let length = entries[entry_index].bytes.len();
        let slot_index = slots
            .iter()
            .enumerate()
            .filter(|(_, slot)| slot.capacity - slot.used >= length)
            .min_by_key(|(_, slot)| (slot.capacity - slot.used - length, slot.offset))
            .map(|(index, _)| index)
            .with_context(|| {
                format!(
                    "{} cannot place the {}-byte record {} in verified source text slots",
                    source.file_name, length, entries[entry_index].id
                )
            })?;
        let slot = &mut slots[slot_index];
        let offset = slot.offset + slot.used;
        slot.replacement[offset - slot.offset..offset - slot.offset + length]
            .copy_from_slice(&entries[entry_index].bytes);
        slot.used += length;
        entries[entry_index].file_offset = offset;
        entries[entry_index].com_address = u16::try_from(offset + COM_ORIGIN)
            .context("playback-driver text address exceeds 16 bits")?;
    }

    let capacity = slots.iter().map(|slot| slot.capacity).sum();
    let used = entries.iter().map(|entry| entry.bytes.len()).sum();
    let writes = slots
        .into_iter()
        .map(|slot| TextStorageWrite {
            id: slot.id,
            offset: slot.offset,
            expected_original: slot.expected_original,
            replacement: slot.replacement,
        })
        .collect();
    Ok((writes, capacity, used))
}

#[cfg(test)]
#[path = "layout_tests.rs"]
mod layout_tests;
