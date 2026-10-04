use std::collections::BTreeSet;
use std::env;
use std::fs;

use bayoen_wars_builder::{
    ArleCharacterAssetSelection, DevelopmentBuildStatus, IntegratedLocalizationDevelopmentInputs,
    IntegratedLocalizationFileDisposition, MadSystemTextDisposition,
    build_arle_character_asset_development_image, build_dialogue_text_development_image,
    build_fixed_gaiji_text_development_image, build_hangul_visibility_image,
    build_integrated_localization_development_image, build_interface_text_development_image,
    build_mad_scene_narrative_development_image,
    build_mad_scene_narrative_with_title_artwork_development_image,
    build_mad_scene_text_development_image, build_mad_system_interface_text_development_image,
    build_mad_system_text_development_image, build_menu_text_development_image,
    build_monochrome_text_development_image, build_mouse_driver_text_development_image,
    build_narrative_graphics_development_image, build_playback_driver_text_development_image,
    build_sampling_driver_text_development_image, build_shell_text_development_image,
    build_standalone_image, build_system_loader_text_development_image,
    build_title_artwork_development_image, survey_source_path,
    write_fixed_gaiji_text_development_audit, write_interface_text_development_audit,
    write_translation_workspace,
};

#[test]
#[ignore = "requires DS5_DISK1 pointing to a user-owned supported source image"]
fn supported_source_build_is_repeatable_and_does_not_change_the_input() {
    let source = env::var_os("DS5_DISK1").expect("set DS5_DISK1 to the supported Disk 1 HDM");
    let original = fs::read(&source).unwrap();
    let directory = tempfile::tempdir().unwrap();
    let first_path = directory.path().join("first.hdm");
    let second_path = directory.path().join("second.hdm");

    let first_report = build_standalone_image(source.as_ref(), &first_path).unwrap();
    let second_report = build_standalone_image(source.as_ref(), &second_path).unwrap();

    assert_eq!(fs::read(&source).unwrap(), original);
    assert_eq!(
        fs::read(first_path).unwrap(),
        fs::read(second_path).unwrap()
    );
    assert_eq!(first_report, second_report);
    assert_eq!(first_report.file_count, 73);
}

#[test]
#[ignore = "requires DS5_DISK1 pointing to a user-owned supported source image"]
fn supported_source_arle_character_asset_build_is_repeatable_and_read_back() {
    let source = env::var_os("DS5_DISK1").expect("set DS5_DISK1 to the supported Disk 1 HDM");
    let original = fs::read(&source).unwrap();
    let directory = tempfile::tempdir().unwrap();
    let first_path = directory.path().join("first-arle-character.hdm");
    let second_path = directory.path().join("second-arle-character.hdm");
    let project_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let manifest = project_root.join("assets/characters/arle/battle-sprites.json");
    let artwork = project_root.join("assets/characters/arle/battle-sprites.pc98.png");

    let first = build_arle_character_asset_development_image(
        source.as_ref(),
        &manifest,
        &artwork,
        &first_path,
    )
    .unwrap();
    let second = build_arle_character_asset_development_image(
        source.as_ref(),
        &manifest,
        &artwork,
        &second_path,
    )
    .unwrap();

    assert_eq!(fs::read(&source).unwrap(), original);
    assert_eq!(
        fs::read(first_path).unwrap(),
        fs::read(second_path).unwrap()
    );
    assert_eq!(first, second);
    assert_eq!(
        first.patch.build_status,
        DevelopmentBuildStatus::DevelopmentOnly
    );
    assert_eq!(first.patch.approval_status, "needs_human_review");
    assert_eq!(
        first.patch.origin_commit,
        "d5a15d31a9511f48ffc5a1988ac3b4f2e4fa3c09"
    );
    assert_eq!(first.patch.character_slot_index, 6);
    assert_eq!(
        first.patch.palette,
        "mad_com_character_runtime_palette_rgb4"
    );
    assert_eq!(first.patch.palette_table_file_offset, 0xd777);
    assert_eq!(first.patch.runtime_palette_rgb[8], [119, 204, 0]);
    assert_eq!(first.patch.frame_count, 8);
    assert_eq!(
        first
            .patch
            .frames
            .iter()
            .filter_map(|frame| {
                frame
                    .source_coordinate_reference
                    .as_deref()
                    .map(|reference| (frame.role.as_str(), reference))
            })
            .collect::<Vec<_>>(),
        [
            ("charge-maximum", "charge-person-scale"),
            ("charge-initial", "charge-ready"),
        ]
    );
    assert_eq!(
        first
            .patch
            .frames
            .iter()
            .filter(|frame| frame.target_asset == "C08")
            .map(|frame| {
                (
                    frame.subject_source_height,
                    frame.subject_target_height,
                    frame.subject_target_baseline_y,
                )
            })
            .collect::<Vec<_>>(),
        [
            (Some(161), Some(156), Some(178)),
            (Some(156), Some(156), Some(178)),
            (Some(150), Some(156), Some(178)),
        ]
    );
    assert_eq!(first.patch.writes.len(), 2);
    assert!(
        first
            .patch
            .files
            .iter()
            .all(|file| file.replacement_packed_size <= file.original_packed_size)
    );
}

#[test]
#[ignore = "requires DS5_DISK1 pointing to a user-owned supported source image"]
fn supported_source_structure_matches_the_consumer_linked_population() {
    let source = env::var_os("DS5_DISK1").expect("set DS5_DISK1 to the supported Disk 1 HDM");

    let report = survey_source_path(source.as_ref()).unwrap();
    let gaiji = &report.game_data.gaiji;
    let mad = &report.game_data.mad_com;

    assert_eq!(report.localization_assets.total_files, 73);
    assert_eq!(
        report.localization_assets.confirmed_localization_targets,
        15
    );
    assert_eq!(report.localization_assets.evidence_based_exclusions, 58);
    assert_eq!(report.localization_assets.unresolved, 0);
    assert_eq!(report.compile_lz.consumer_linked_file_count, 58);
    assert_eq!(
        report
            .compile_lz
            .files
            .iter()
            .filter(|file| file.stream_count == 1)
            .count(),
        57
    );
    let samples = report
        .compile_lz
        .files
        .iter()
        .find(|file| file.name == "SAMPA")
        .unwrap();
    assert_eq!(samples.stream_count, 22);
    assert_eq!(samples.total_output_size, 101_872);
    assert_eq!(report.asset_bindings.sample_playback.stream_count, 22);
    assert_eq!(report.asset_bindings.sample_playback.sample_rate_hz, 9_600);
    assert_eq!(
        report
            .asset_bindings
            .sample_language_review
            .japanese_voice_stream_count,
        22
    );
    assert_eq!(gaiji.first_character_code, 0x7621);
    assert_eq!(gaiji.last_character_code, 0x777a);
    assert_eq!(gaiji.first_shift_jis_code, 0xeb9f);
    assert_eq!(gaiji.last_shift_jis_code, 0xec9a);
    assert_eq!(gaiji.uniform_record_size, Some(34));
    assert_eq!(gaiji.glyphs.len(), 184);
    assert_eq!(gaiji.installer.install_loop_offset, 0x12);
    assert_eq!(gaiji.installer.install_loop_byte_size, 0x3f);
    assert_eq!(gaiji.installer.install_loop_instruction_count, 21);
    assert_eq!(gaiji.installer.install_call_offset, 0x49);
    assert_eq!(gaiji.installer.install_routine_offset, 0x6f);
    assert_eq!(gaiji.installer.install_routine_byte_size, 7);
    assert_eq!(gaiji.installer.install_routine_instruction_count, 4);
    assert_eq!(gaiji.installer.bios_interrupt_vector, 0x18);
    assert_eq!(gaiji.installer.bios_function, 0x1a);
    assert_eq!(gaiji.installer.glyph_segment_register, "bx");
    assert_eq!(gaiji.installer.glyph_record_offset_register, "cx");
    assert_eq!(gaiji.installer.character_code_register, "dx");
    assert_eq!(mad.gaiji_readiness.check_offset, 0x3157);
    assert_eq!(mad.gaiji_readiness.expected_bitmap_table_offset, 0x3184);
    assert_eq!(mad.gaiji_readiness.probe_renderer_offset, 0x31a4);
    assert_eq!(mad.gaiji_readiness.probe_text_offset, 0x31b1);
    assert_eq!(mad.gaiji_readiness.probe_shift_jis_code, 0xeba0);
    assert_eq!(mad.gaiji_readiness.reserved_glyph_index, 1);
    assert_eq!(mad.gaiji_readiness.row_count, 16);
    assert_eq!(mad.memory.stack_setup_offset, 0x75);
    assert_eq!(mad.memory.stack_setup_byte_size, 5);
    assert_eq!(mad.memory.stack_setup_instruction_count, 2);
    assert_eq!(mad.memory.initial_stack_pointer, 0xf7fe);
    assert_eq!(mad.memory.program_end_com_address, 0xf7fe);
    assert_eq!(mad.memory.last_nonzero_file_offset, 0xe1f3);
    assert_eq!(mad.memory.zero_tail_start, 0xe1f4);
    assert_eq!(mad.memory.zero_tail_byte_size, 5_386);
    assert_eq!(mad.scene_transition.dialogue_routine_file_offset, 0xaf4b);
    assert_eq!(mad.scene_transition.failure_exit_file_offset, 0x0490);
    assert_eq!(mad.scene_transition.call_site_count, 2);
    assert_eq!(
        mad.scene_transition
            .calls
            .iter()
            .map(|call| (call.id.as_str(), call.file_offset))
            .collect::<Vec<_>>(),
        [("new-game-dialogue", 0x0290), ("ending-dialogue", 0x03cc)]
    );
    assert_eq!(mad.ending_transition.completion_stage_group, 9);
    assert_eq!(mad.ending_transition.ending_stage_group, 10);
    assert_eq!(mad.ending_transition.ending_selector_value, 0x80);
    assert_eq!(mad.ending_transition.ending_dialogue_group_number, 11);
    assert_eq!(
        mad.ending_transition.ending_dialogue_group_id,
        "dialogue-group-11"
    );
    assert_eq!(mad.fixed_gaiji_text.slots.len(), 10);
    assert_eq!(
        mad.fixed_gaiji_text
            .runtime
            .selection_entry_call
            .file_offset,
        0xa042
    );
    assert_eq!(mad.fixed_gaiji_text.runtime.render_call.file_offset, 0xa477);
    assert_eq!(mad.fixed_gaiji_text.runtime.renderer_file_offset, 0x3a48);
    assert_eq!(mad.interface_text.entry_count, 88);
    assert!(mad.system_runtime.reference_population_complete);
    assert_eq!(mad.system_runtime.semantic_reference_count, 23);
    assert_eq!(mad.system_runtime.storage_reference_count, 24);
    assert_eq!(mad.system_runtime.machine_code_reference_count, 10);
    assert_eq!(mad.system_runtime.metadata_reference_count, 14);
    assert_eq!(mad.system_runtime.runtime_insert_reference_count, 1);
    assert!(
        mad.interface_text
            .reference_catalog
            .reference_population_complete
    );
    assert_eq!(mad.interface_text.reference_catalog.reference_count, 139);
    assert_eq!(
        mad.interface_text
            .reference_catalog
            .machine_code_reference_count,
        44
    );
    assert_eq!(
        mad.interface_text
            .reference_catalog
            .metadata_reference_count,
        95
    );
    assert_eq!(
        mad.interface_text.reference_catalog.unreferenced_entry_ids,
        ["interface-text-006"]
    );
    assert_eq!(mad.dialogue.group_count, 11);
    assert_eq!(mad.dialogue.entry_count, 55);
    assert!(mad.dialogue.strings_are_contiguous);
    assert_eq!(mad.dialogue.text_start_offset, 0xb1bd);
    assert_eq!(mad.dialogue.text_end_offset, 0xb7e3);
    assert!(mad.dialogue.reference_catalog.reference_population_complete);
    assert_eq!(mad.dialogue.reference_catalog.reference_count, 67);
    assert_eq!(
        mad.dialogue.reference_catalog.machine_code_reference_count,
        1
    );
    assert_eq!(
        mad.dialogue.reference_catalog.group_pointer_reference_count,
        11
    );
    assert_eq!(
        mad.dialogue.reference_catalog.text_pointer_reference_count,
        55
    );
    assert_eq!(
        mad.dialogue
            .reference_catalog
            .non_reference_occurrence_count,
        6
    );
    let menu = &report.game_data.menu;
    assert_eq!(menu.text.unique_text_count, 47);
    assert_eq!(menu.text.reference_count, 50);
    assert!(menu.runtime.reference_population_complete);
    assert_eq!(menu.runtime.semantic_reference_count, 50);
    assert_eq!(menu.runtime.storage_reference_count, 49);
    assert_eq!(menu.runtime.machine_code_reference_count, 18);
    assert_eq!(menu.runtime.metadata_reference_count, 31);
    assert_eq!(menu.runtime.runtime_insert_reference_count, 2);
    assert_eq!(menu.runtime.total_machine_code_relocation_count, 20);
    assert_eq!(menu.runtime.referenced_entry_count, 47);
    assert_eq!(menu.runtime.non_reference_occurrence_count, 13);
    assert_eq!(menu.runtime.entry.hook_instruction_offset, 0x0009);
    assert_eq!(menu.runtime.entry.original_call_target_com_address, 0x048d);
    assert_eq!(menu.runtime.entry.resume_com_address, 0x010c);
    assert_eq!(menu.runtime.entry.initial_stack_pointer, 0x181e);
    assert_eq!(menu.runtime.entry.program_end_com_address, 0x181e);
    let monochrome = &report.asset_bindings.monochrome_text;
    assert_eq!(monochrome.glyph_renderer_instruction_count, 9);
    assert_eq!(monochrome.glyph_renderer_call_count, 1);
    assert_eq!(monochrome.opening.glyphs.len(), 129);
    assert_eq!(monochrome.opening.glyph_capacity, 254);
    assert_eq!(monochrome.opening.pages.len(), 19);
    assert_eq!(monochrome.ending.glyphs.len(), 69);
    assert_eq!(monochrome.ending.glyph_capacity, 254);
    assert_eq!(monochrome.ending.pages.len(), 9);
    let voice_samples = &report.asset_bindings.voice_samples;
    assert_eq!(voice_samples.streams.len(), 22);
    assert_eq!(
        voice_samples
            .streams
            .iter()
            .map(|stream| stream.sample_count)
            .sum::<usize>(),
        203_624
    );
    assert!(
        voice_samples
            .streams
            .iter()
            .all(|stream| stream.id == format!("voice-sample-{:02}", stream.stream_index + 1))
    );
    let baked_text = &report.asset_bindings.baked_text;
    assert_eq!(baked_text.source_asset_count, 3);
    assert_eq!(baked_text.translation_unit_count, 16);
    assert_eq!(baked_text.consumer_blocks.len(), 5);
    assert!(
        baked_text
            .consumer_blocks
            .iter()
            .all(|block| block.call_count > 0)
    );
    let bplay = report
        .external_text
        .programs
        .iter()
        .find(|program| program.file_name == "BPLAY6.COM")
        .unwrap();
    assert_eq!(bplay.unique_text_count, 20);
    assert_eq!(bplay.reference_count, 23);
    assert!(
        bplay
            .entries
            .iter()
            .any(|entry| entry.id == "external-bplay6-text-020")
    );
    let fplay = report
        .external_text
        .programs
        .iter()
        .find(|program| program.file_name == "FPLAY6.COM")
        .unwrap();
    assert_eq!(fplay.unique_text_count, 21);
    assert_eq!(fplay.reference_count, 26);
    let system_loader = report
        .external_text
        .programs
        .iter()
        .find(|program| program.file_name == "MEGDOS.SYS")
        .unwrap();
    assert_eq!(system_loader.unique_text_count, 7);
    assert_eq!(system_loader.reference_count, 8);
    assert_eq!(report.external_text.unique_text_count, 74);
    assert_eq!(report.external_text.reference_count, 83);
}

#[test]
#[ignore = "requires DS5_DISK1 pointing to a user-owned supported source image"]
fn supported_source_hangul_visibility_build_is_repeatable() {
    let source = env::var_os("DS5_DISK1").expect("set DS5_DISK1 to the supported Disk 1 HDM");
    let original = fs::read(&source).unwrap();
    let directory = tempfile::tempdir().unwrap();
    let first_path = directory.path().join("first-hangul.hdm");
    let second_path = directory.path().join("second-hangul.hdm");

    let first = build_hangul_visibility_image(source.as_ref(), &first_path).unwrap();
    let second = build_hangul_visibility_image(source.as_ref(), &second_path).unwrap();

    assert_eq!(fs::read(&source).unwrap(), original);
    assert_eq!(
        fs::read(first_path).unwrap(),
        fs::read(second_path).unwrap()
    );
    assert_eq!(first, second);
    assert_eq!(first.build.file_count, 73);
    assert_eq!(first.patch.character, "가");
    assert_eq!(first.patch.shift_jis_code, "0xEC4A");
    assert_eq!(first.patch.audited_consumer_files, 72);
    assert_eq!(first.patch.writes.len(), 3);
    assert!(
        first
            .patch
            .writes
            .iter()
            .all(|write| write.intent == "data")
    );
}

#[test]
#[ignore = "requires DS5_DISK1 pointing to a user-owned supported source image"]
fn supported_source_shell_text_build_is_repeatable_and_read_back() {
    let source = env::var_os("DS5_DISK1").expect("set DS5_DISK1 to the supported Disk 1 HDM");
    let original = fs::read(&source).unwrap();
    let directory = tempfile::tempdir().unwrap();
    let first_path = directory.path().join("first-shell.hdm");
    let second_path = directory.path().join("second-shell.hdm");
    let translations = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/translations");

    let first =
        build_shell_text_development_image(source.as_ref(), &translations, &first_path).unwrap();
    let second =
        build_shell_text_development_image(source.as_ref(), &translations, &second_path).unwrap();

    assert_eq!(fs::read(&source).unwrap(), original);
    assert_eq!(
        fs::read(first_path).unwrap(),
        fs::read(second_path).unwrap()
    );
    assert_eq!(first, second);
    assert_eq!(first.build.file_count, 73);
    assert_eq!(
        first.patch.build_status,
        DevelopmentBuildStatus::DevelopmentOnly
    );
    assert_eq!(first.patch.translation_status, "needs_human_review");
    assert_eq!(first.patch.source_file_size, 981);
    assert_eq!(first.patch.output_file_size, 3_306);
    assert_eq!(first.patch.used_gaiji_slots, 45);
    assert_eq!(first.patch.installer_code_bytes, 551);
    assert_eq!(first.patch.glyph_record_bytes, 1_530);
    assert_eq!(first.patch.packed_text_bytes, 244);
    assert_eq!(first.patch.entries.len(), 8);
    assert_eq!(first.patch.writes.len(), 13);
    assert_eq!(
        first
            .patch
            .writes
            .iter()
            .filter(|write| write.intent == "machine_code")
            .count(),
        11
    );
    assert!(
        first
            .patch
            .glyphs
            .iter()
            .enumerate()
            .all(|(index, glyph)| { glyph.record_offset == 1_532 + index * 34 })
    );
}

#[test]
#[ignore = "requires DS5_DISK1 pointing to a user-owned supported source image"]
fn supported_source_system_loader_text_build_is_repeatable_and_read_back() {
    let source = env::var_os("DS5_DISK1").expect("set DS5_DISK1 to the supported Disk 1 HDM");
    let original = fs::read(&source).unwrap();
    let directory = tempfile::tempdir().unwrap();
    let first_path = directory.path().join("first-system-loader.hdm");
    let second_path = directory.path().join("second-system-loader.hdm");
    let translations = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/translations");

    let first =
        build_system_loader_text_development_image(source.as_ref(), &translations, &first_path)
            .unwrap();
    let second =
        build_system_loader_text_development_image(source.as_ref(), &translations, &second_path)
            .unwrap();

    assert_eq!(fs::read(&source).unwrap(), original);
    assert_eq!(
        fs::read(first_path).unwrap(),
        fs::read(second_path).unwrap()
    );
    assert_eq!(first, second);
    assert_eq!(first.build.file_count, 73);
    assert_eq!(
        first.patch.build_status,
        DevelopmentBuildStatus::DevelopmentOnly
    );
    assert_eq!(first.patch.translation_status, "needs_human_review");
    assert_eq!(first.patch.source_file_size, 17_547);
    assert_eq!(first.patch.output_file_size, 20_203);
    assert_eq!(first.patch.source_resident_byte_count, 0x09a0);
    assert_eq!(first.patch.output_resident_byte_count, 0x1400);
    assert_eq!(first.patch.resident_extension_byte_count, 0x0a60);
    assert_eq!(first.patch.shifted_tail_byte_count, 15_083);
    assert_eq!(
        first.patch.shifted_tail_sha256,
        "1409b495618cc84bbe2748ebc1244904483f34b73339c82ff8a5536067d9e816"
    );
    assert_eq!(first.patch.used_gaiji_slots, 52);
    assert_eq!(first.patch.installer_code_bytes, 635);
    assert_eq!(first.patch.glyph_record_bytes, 1_768);
    assert_eq!(first.patch.packed_text_bytes, 238);
    assert_eq!(first.patch.trampoline_code_bytes, 8);
    assert_eq!(first.patch.reference_count, 8);
    assert_eq!(first.patch.entries.len(), 7);
    assert_eq!(first.patch.resident_writes.len(), 15);
    assert_eq!(
        first
            .patch
            .resident_writes
            .iter()
            .filter(|write| write.intent == "machine_code")
            .count(),
        13
    );
    assert_eq!(first.patch.writes.len(), 1);
    assert_eq!(first.patch.writes[0].intent, "data");
    assert_eq!(
        first
            .patch
            .entries
            .iter()
            .find(|entry| entry.id == "external-megdos-text-007")
            .unwrap()
            .reference_count,
        2
    );
}

#[test]
#[ignore = "requires DS5_DISK1 pointing to a user-owned supported source image"]
fn supported_source_sampling_driver_text_build_is_repeatable_and_read_back() {
    let source = env::var_os("DS5_DISK1").expect("set DS5_DISK1 to the supported Disk 1 HDM");
    let original = fs::read(&source).unwrap();
    let directory = tempfile::tempdir().unwrap();
    let first_path = directory.path().join("first-sampling-driver.hdm");
    let second_path = directory.path().join("second-sampling-driver.hdm");
    let translations = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/translations");

    let first =
        build_sampling_driver_text_development_image(source.as_ref(), &translations, &first_path)
            .unwrap();
    let second =
        build_sampling_driver_text_development_image(source.as_ref(), &translations, &second_path)
            .unwrap();

    assert_eq!(fs::read(&source).unwrap(), original);
    assert_eq!(
        fs::read(first_path).unwrap(),
        fs::read(second_path).unwrap()
    );
    assert_eq!(first, second);
    assert_eq!(first.build.file_count, 73);
    assert_eq!(
        first.patch.build_status,
        DevelopmentBuildStatus::DevelopmentOnly
    );
    assert_eq!(first.patch.translation_status, "needs_human_review");
    assert_eq!(first.patch.source_file_size, 5_921);
    assert_eq!(first.patch.resident_end_com_address, 0x15ec);
    assert_eq!(first.patch.used_gaiji_slots, 20);
    assert_eq!(first.patch.glyph_record_bytes, 680);
    assert_eq!(first.patch.entries.len(), 3);
    assert_eq!(first.patch.writes.len(), 7);
    assert_eq!(
        first
            .patch
            .writes
            .iter()
            .filter(|write| write.intent == "machine_code")
            .count(),
        5
    );
    assert!(
        first
            .patch
            .entries
            .iter()
            .find(|entry| entry.id == "external-bsamp-text-001")
            .unwrap()
            .lines[0]
            .starts_with("～～ ")
    );
}

#[test]
#[ignore = "requires DS5_DISK1 pointing to a user-owned supported source image"]
fn supported_source_playback_driver_text_build_is_repeatable_and_read_back() {
    let source = env::var_os("DS5_DISK1").expect("set DS5_DISK1 to the supported Disk 1 HDM");
    let original = fs::read(&source).unwrap();
    let directory = tempfile::tempdir().unwrap();
    let first_path = directory.path().join("first-playback-drivers.hdm");
    let second_path = directory.path().join("second-playback-drivers.hdm");
    let translations = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/translations");

    let first =
        build_playback_driver_text_development_image(source.as_ref(), &translations, &first_path)
            .unwrap();
    let second =
        build_playback_driver_text_development_image(source.as_ref(), &translations, &second_path)
            .unwrap();

    assert_eq!(fs::read(&source).unwrap(), original);
    assert_eq!(
        fs::read(first_path).unwrap(),
        fs::read(second_path).unwrap()
    );
    assert_eq!(first, second);
    assert_eq!(first.build.file_count, 73);
    assert_eq!(
        first.patch.build_status,
        DevelopmentBuildStatus::DevelopmentOnly
    );
    assert_eq!(first.patch.translation_status, "needs_human_review");
    assert_eq!(first.patch.drivers.len(), 2);
    assert_eq!(first.patch.writes.len(), 2);
    let bplay = first
        .patch
        .drivers
        .iter()
        .find(|driver| driver.file_name == "BPLAY6.COM")
        .unwrap();
    assert_eq!(bplay.entries.len(), 20);
    assert_eq!(bplay.reference_count, 23);
    assert_eq!(bplay.used_gaiji_slots, 133);
    assert_eq!(bplay.first_nonresident_file_offset, 0x7f10);
    let fplay = first
        .patch
        .drivers
        .iter()
        .find(|driver| driver.file_name == "FPLAY6.COM")
        .unwrap();
    assert_eq!(fplay.entries.len(), 21);
    assert_eq!(fplay.reference_count, 26);
    assert_eq!(fplay.used_gaiji_slots, 106);
    assert_eq!(fplay.first_nonresident_file_offset, 0x7f10);
}

#[test]
#[ignore = "requires DS5_DISK1 pointing to a user-owned supported source image"]
fn supported_source_menu_text_build_is_repeatable_and_read_back() {
    let source = env::var_os("DS5_DISK1").expect("set DS5_DISK1 to the supported Disk 1 HDM");
    let original = fs::read(&source).unwrap();
    let directory = tempfile::tempdir().unwrap();
    let first_path = directory.path().join("first-menu.hdm");
    let second_path = directory.path().join("second-menu.hdm");
    let translations = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/translations");

    let first =
        build_menu_text_development_image(source.as_ref(), &translations, &first_path).unwrap();
    let second =
        build_menu_text_development_image(source.as_ref(), &translations, &second_path).unwrap();

    assert_eq!(fs::read(&source).unwrap(), original);
    assert_eq!(
        fs::read(first_path).unwrap(),
        fs::read(second_path).unwrap()
    );
    assert_eq!(first, second);
    assert_eq!(first.build.file_count, 73);
    assert_eq!(
        first.patch.build_status,
        DevelopmentBuildStatus::DevelopmentOnly
    );
    assert_eq!(first.patch.translation_status, "needs_human_review");
    assert_eq!(first.patch.source_file_size, 5_918);
    assert_eq!(first.patch.output_file_size, 13_785);
    assert_eq!(first.patch.used_gaiji_slots, 137);
    assert_eq!(first.patch.installer_code_bytes, 1_655);
    assert_eq!(first.patch.glyph_record_bytes, 4_658);
    assert_eq!(first.patch.trampoline_code_bytes, 6);
    assert_eq!(first.patch.packed_text_bytes, 1_548);
    assert_eq!(first.patch.semantic_reference_count, 50);
    assert_eq!(first.patch.storage_reference_count, 49);
    assert_eq!(first.patch.machine_code_reference_count, 18);
    assert_eq!(first.patch.metadata_reference_count, 31);
    assert_eq!(first.patch.runtime_insert_reference_count, 2);
    assert_eq!(first.patch.entries.len(), 47);
    assert_eq!(first.patch.writes.len(), 56);
    assert_eq!(
        first
            .patch
            .writes
            .iter()
            .filter(|write| write.intent == "machine_code")
            .count(),
        23
    );
    assert_eq!(
        first
            .patch
            .entries
            .iter()
            .map(|entry| entry.runtime_field_count)
            .sum::<usize>(),
        2
    );
}

#[test]
#[ignore = "requires DS5_DISK1 pointing to a user-owned supported source image"]
fn supported_source_mouse_driver_text_build_is_repeatable_and_read_back() {
    let source = env::var_os("DS5_DISK1").expect("set DS5_DISK1 to the supported Disk 1 HDM");
    let original = fs::read(&source).unwrap();
    let directory = tempfile::tempdir().unwrap();
    let first_path = directory.path().join("first-mouse-driver.hdm");
    let second_path = directory.path().join("second-mouse-driver.hdm");
    let translations = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/translations");

    let first =
        build_mouse_driver_text_development_image(source.as_ref(), &translations, &first_path)
            .unwrap();
    let second =
        build_mouse_driver_text_development_image(source.as_ref(), &translations, &second_path)
            .unwrap();

    assert_eq!(fs::read(&source).unwrap(), original);
    assert_eq!(
        fs::read(first_path).unwrap(),
        fs::read(second_path).unwrap()
    );
    assert_eq!(first, second);
    assert_eq!(first.build.file_count, 73);
    assert_eq!(
        first.patch.build_status,
        DevelopmentBuildStatus::DevelopmentOnly
    );
    assert_eq!(first.patch.translation_status, "needs_human_review");
    assert_eq!(first.patch.source_file_size, 3_605);
    assert_eq!(first.patch.output_file_size, 7_697);
    assert_eq!(first.patch.resident_paragraph_count, 0x0096);
    assert_eq!(first.patch.used_gaiji_slots, 71);
    assert_eq!(first.patch.installer_code_bytes, 866);
    assert_eq!(first.patch.glyph_record_bytes, 2_414);
    assert_eq!(first.patch.packed_text_bytes, 812);
    assert_eq!(first.patch.entries.len(), 15);
    assert_eq!(first.patch.writes.len(), 15);
    assert_eq!(
        first
            .patch
            .writes
            .iter()
            .filter(|write| write.intent == "machine_code")
            .count(),
        11
    );
    assert_eq!(
        first
            .patch
            .writes
            .iter()
            .filter(|write| write.intent == "metadata")
            .count(),
        2
    );
    assert!(
        first
            .patch
            .entries
            .iter()
            .find(|entry| entry.id == "external-nmouse-text-001")
            .unwrap()
            .lines[0]
            .starts_with("∮∮  ")
    );
}

#[test]
#[ignore = "requires DS5_DISK1 pointing to a user-owned supported source image"]
fn supported_source_translation_workspace_covers_every_known_text_entry() {
    let source = env::var_os("DS5_DISK1").expect("set DS5_DISK1 to the supported Disk 1 HDM");
    let directory = tempfile::tempdir().unwrap();
    let output = directory.path().join("translations");

    let report = write_translation_workspace(source.as_ref(), &output).unwrap();

    // The protected denominator includes the two producer-evidence atlases and
    // every consumer-backed text segment, including battle callouts.
    assert_eq!(report.segment_count, 31);
    assert_eq!(report.entry_count, 615);
    assert_eq!(report.target_file_count, 15);
    assert_eq!(report.pending_source_file_count, 0);
    let index: serde_json::Value =
        serde_json::from_slice(&fs::read(output.join("index.json")).unwrap()).unwrap();
    assert_eq!(index["target_population_complete"], true);
    assert_eq!(index["translation_segmentation_complete"], true);
}

#[test]
#[ignore = "requires DS5_DISK1 pointing to a user-owned supported source image"]
fn supported_source_monochrome_text_build_is_repeatable_and_read_back() {
    let source = env::var_os("DS5_DISK1").expect("set DS5_DISK1 to the supported Disk 1 HDM");
    let original = fs::read(&source).unwrap();
    let directory = tempfile::tempdir().unwrap();
    let first_path = directory.path().join("first-monochrome.hdm");
    let second_path = directory.path().join("second-monochrome.hdm");
    let translations = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/translations");

    let first =
        build_monochrome_text_development_image(source.as_ref(), &translations, &first_path)
            .unwrap();
    let second =
        build_monochrome_text_development_image(source.as_ref(), &translations, &second_path)
            .unwrap();

    assert_eq!(fs::read(&source).unwrap(), original);
    assert_eq!(
        fs::read(first_path).unwrap(),
        fs::read(second_path).unwrap()
    );
    assert_eq!(first, second);
    assert_eq!(first.build.file_count, 73);
    assert_eq!(
        first.patch.build_status,
        DevelopmentBuildStatus::DevelopmentOnly
    );
    assert_eq!(first.patch.translation_status, "needs_human_review");
    assert_eq!(first.patch.sequences[0].glyph_capacity, 254);
    assert_eq!(first.patch.sequences[0].used_glyph_count, 147);
    assert_eq!(first.patch.sequences[0].page_count, 19);
    assert_eq!(first.patch.sequences[1].glyph_capacity, 254);
    assert_eq!(first.patch.sequences[1].used_glyph_count, 78);
    assert_eq!(first.patch.sequences[1].page_count, 9);
    assert_eq!(first.patch.writes.len(), 6);
    assert!(
        first
            .patch
            .writes
            .iter()
            .all(|write| write.intent == "data" || write.intent == "metadata")
    );
}

#[test]
#[ignore = "requires DS5_DISK1 pointing to a user-owned supported source image"]
fn supported_source_narrative_graphics_build_is_repeatable_and_read_back() {
    let source = env::var_os("DS5_DISK1").expect("set DS5_DISK1 to the supported Disk 1 HDM");
    let original = fs::read(&source).unwrap();
    let directory = tempfile::tempdir().unwrap();
    let first_path = directory.path().join("first-narrative-graphics.hdm");
    let second_path = directory.path().join("second-narrative-graphics.hdm");
    let translations = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/translations");

    let first =
        build_narrative_graphics_development_image(source.as_ref(), &translations, &first_path)
            .unwrap();
    let second =
        build_narrative_graphics_development_image(source.as_ref(), &translations, &second_path)
            .unwrap();

    assert_eq!(fs::read(&source).unwrap(), original);
    assert_eq!(
        fs::read(first_path).unwrap(),
        fs::read(second_path).unwrap()
    );
    assert_eq!(first, second);
    assert_eq!(first.build.file_count, 73);
    assert_eq!(
        first.patch.monochrome.build_status,
        DevelopmentBuildStatus::DevelopmentOnly
    );
    assert_eq!(first.patch.monochrome.sequences[0].used_glyph_count, 147);
    assert_eq!(first.patch.monochrome.sequences[1].used_glyph_count, 78);
    assert_eq!(first.patch.monochrome.writes.len(), 6);
    assert_eq!(
        first.patch.baked.build_status,
        DevelopmentBuildStatus::DevelopmentOnly
    );
    assert_eq!(first.patch.baked.translation_status, "needs_human_review");
    assert_eq!(first.patch.baked.assets.len(), 3);
    assert_eq!(
        first
            .patch
            .baked
            .assets
            .iter()
            .map(|asset| asset.translation_unit_ids.len())
            .sum::<usize>(),
        16
    );
    assert_eq!(first.patch.baked.writes.len(), 3);
    assert!(
        first
            .patch
            .baked
            .writes
            .iter()
            .all(|write| write.intent == "data")
    );
}

#[test]
#[ignore = "requires DS5_DISK1 pointing to a user-owned supported source image"]
fn supported_source_fixed_gaiji_text_build_is_repeatable_and_read_back() {
    let source = env::var_os("DS5_DISK1").expect("set DS5_DISK1 to the supported Disk 1 HDM");
    let original = fs::read(&source).unwrap();
    let directory = tempfile::tempdir().unwrap();
    let first_path = directory.path().join("first-fixed-text.hdm");
    let second_path = directory.path().join("second-fixed-text.hdm");
    let translations = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/translations");

    let first =
        build_fixed_gaiji_text_development_image(source.as_ref(), &translations, &first_path)
            .unwrap();
    let second =
        build_fixed_gaiji_text_development_image(source.as_ref(), &translations, &second_path)
            .unwrap();

    assert_eq!(fs::read(&source).unwrap(), original);
    assert_eq!(
        fs::read(first_path).unwrap(),
        fs::read(second_path).unwrap()
    );
    assert_eq!(first, second);
    assert_eq!(first.build.file_count, 73);
    assert_eq!(
        first.patch.build_status,
        DevelopmentBuildStatus::DevelopmentOnly
    );
    assert_eq!(first.patch.translation_status, "needs_human_review");
    assert_eq!(first.patch.available_gaiji_slots, 170);
    assert_eq!(first.patch.used_gaiji_slots, 123);
    assert_eq!(first.patch.slots.len(), 10);
    assert_eq!(first.patch.writes.len(), 3);
    assert!(
        first
            .patch
            .writes
            .iter()
            .all(|write| write.intent == "data")
    );
    let audit_directory = directory.path().join("fixed-text-audit");
    let audit =
        write_fixed_gaiji_text_development_audit(source.as_ref(), &translations, &audit_directory)
            .unwrap();
    assert_eq!(audit.preview_count, 10);
}

#[test]
#[ignore = "requires DS5_DISK1 pointing to a user-owned supported source image"]
fn supported_source_interface_text_build_is_repeatable_and_read_back() {
    let source = env::var_os("DS5_DISK1").expect("set DS5_DISK1 to the supported Disk 1 HDM");
    let original = fs::read(&source).unwrap();
    let directory = tempfile::tempdir().unwrap();
    let first_path = directory.path().join("first-interface-text.hdm");
    let second_path = directory.path().join("second-interface-text.hdm");
    let translations = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/translations");

    let first = build_interface_text_development_image(source.as_ref(), &translations, &first_path)
        .unwrap();
    let second =
        build_interface_text_development_image(source.as_ref(), &translations, &second_path)
            .unwrap();

    assert_eq!(fs::read(&source).unwrap(), original);
    assert_eq!(
        fs::read(first_path).unwrap(),
        fs::read(second_path).unwrap()
    );
    assert_eq!(first, second);
    assert_eq!(first.build.file_count, 73);
    assert_eq!(
        first.patch.build_status,
        DevelopmentBuildStatus::DevelopmentOnly
    );
    assert_eq!(first.patch.translation_status, "needs_human_review");
    assert_eq!(first.patch.available_gaiji_slots, 170);
    assert_eq!(first.patch.used_gaiji_slots, 162);
    assert_eq!(first.patch.storage_capacity, 1_166);
    assert_eq!(first.patch.packed_storage_bytes, 1_042);
    assert_eq!(first.patch.storage_headroom, 124);
    assert_eq!(first.patch.entries.len(), 88);
    assert_eq!(first.patch.reference_count, 139);
    assert_eq!(first.patch.machine_code_reference_count, 44);
    assert_eq!(first.patch.metadata_reference_count, 95);
    assert_eq!(
        first
            .patch
            .entries
            .iter()
            .map(|entry| entry.reference_count)
            .sum::<usize>(),
        139
    );
    assert_eq!(first.patch.unreferenced_entry_ids, ["interface-text-006"]);
    assert_eq!(
        (
            first
                .patch
                .shared_alert_window
                .source_background_invalidation_x_pixels,
            first
                .patch
                .shared_alert_window
                .output_background_invalidation_x_pixels,
        ),
        (0x00c0, 0x0090)
    );
    assert_eq!(
        (
            first
                .patch
                .spring_capture_window
                .source_background_invalidation_x_pixels,
            first
                .patch
                .spring_capture_window
                .output_background_invalidation_x_pixels,
        ),
        (0x00a0, 0x0080)
    );
    assert_eq!(
        (
            first
                .patch
                .spring_recovery_window
                .source_background_invalidation_x_pixels,
            first
                .patch
                .spring_recovery_window
                .output_background_invalidation_x_pixels,
        ),
        (0x0090, 0x0070)
    );
    for (id, offset) in [
        ("shared-alert-background-invalidation-x", 0x6bbe),
        ("spring-capture-background-invalidation-x", 0x6bfa),
        ("spring-recovery-background-invalidation-x", 0x6c12),
    ] {
        let write = first
            .patch
            .writes
            .iter()
            .find(|write| write.id == id)
            .unwrap_or_else(|| panic!("missing {id}"));
        assert_eq!(write.file_name, "MAD.COM");
        assert_eq!(write.offset, offset);
        assert_eq!(write.byte_size, 2);
        assert_eq!(write.intent, "metadata");
    }

    let audit_directory = directory.path().join("interface-text-audit");
    let audit =
        write_interface_text_development_audit(source.as_ref(), &translations, &audit_directory)
            .unwrap();
    assert_eq!(audit.preview_count, 89);
    assert!(
        audit_directory
            .join("interface-text-contact-sheet.png")
            .is_file()
    );
}

#[test]
#[ignore = "requires DS5_DISK1 pointing to a user-owned supported source image"]
fn supported_source_mad_system_text_build_is_repeatable_and_read_back() {
    let source = env::var_os("DS5_DISK1").expect("set DS5_DISK1 to the supported Disk 1 HDM");
    let original = fs::read(&source).unwrap();
    let directory = tempfile::tempdir().unwrap();
    let first_path = directory.path().join("first-mad-system-text.hdm");
    let second_path = directory.path().join("second-mad-system-text.hdm");
    let translations = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/translations");

    let first =
        build_mad_system_text_development_image(source.as_ref(), &translations, &first_path)
            .unwrap();
    let second =
        build_mad_system_text_development_image(source.as_ref(), &translations, &second_path)
            .unwrap();

    assert_eq!(fs::read(&source).unwrap(), original);
    assert_eq!(
        fs::read(first_path).unwrap(),
        fs::read(second_path).unwrap()
    );
    assert_eq!(first, second);
    assert_eq!(first.build.file_count, 73);
    assert_eq!(
        first.patch.build_status,
        DevelopmentBuildStatus::DevelopmentOnly
    );
    assert_eq!(first.patch.translation_status, "needs_human_review");
    assert_eq!(first.patch.available_gaiji_slots, 170);
    assert_eq!(first.patch.used_gaiji_slots, 32);
    assert_eq!(first.patch.storage_capacity, 407);
    assert_eq!(first.patch.packed_storage_bytes, 285);
    assert_eq!(first.patch.storage_headroom, 122);
    assert_eq!(first.patch.semantic_reference_count, 23);
    assert_eq!(first.patch.storage_reference_count, 24);
    assert_eq!(first.patch.machine_code_reference_count, 10);
    assert_eq!(first.patch.metadata_reference_count, 14);
    assert_eq!(first.patch.runtime_insert_reference_count, 1);
    assert_eq!(first.patch.entries.len(), 10);
    assert_eq!(first.patch.writes.len(), 14);
    assert_eq!(
        first
            .patch
            .writes
            .iter()
            .filter(|write| write.intent == "machine_code")
            .count(),
        10
    );
    let dynamic = first
        .patch
        .entries
        .iter()
        .find(|entry| entry.id == "mad-system-text-005")
        .unwrap();
    assert_eq!(dynamic.runtime_insert_byte_offset, Some(11));
    assert_eq!(dynamic.runtime_insert_byte_capacity, Some(8));
}

#[test]
#[ignore = "requires DS5_DISK1 pointing to a user-owned supported source image"]
fn supported_source_mad_system_interface_text_build_is_repeatable_and_read_back() {
    let source = env::var_os("DS5_DISK1").expect("set DS5_DISK1 to the supported Disk 1 HDM");
    let original = fs::read(&source).unwrap();
    let directory = tempfile::tempdir().unwrap();
    let first_path = directory.path().join("first-mad-system-interface-text.hdm");
    let second_path = directory
        .path()
        .join("second-mad-system-interface-text.hdm");
    let translations = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/translations");

    let first = build_mad_system_interface_text_development_image(
        source.as_ref(),
        &translations,
        &first_path,
    )
    .unwrap();
    let second = build_mad_system_interface_text_development_image(
        source.as_ref(),
        &translations,
        &second_path,
    )
    .unwrap();

    assert_eq!(fs::read(&source).unwrap(), original);
    assert_eq!(
        fs::read(first_path).unwrap(),
        fs::read(second_path).unwrap()
    );
    assert_eq!(first, second);
    assert_eq!(first.build.file_count, 73);
    assert_eq!(
        first.patch.build_status,
        DevelopmentBuildStatus::DevelopmentOnly
    );
    assert_eq!(first.patch.translation_status, "needs_human_review");
    assert_eq!(first.patch.available_gaiji_slots, 170);
    assert!(first.patch.used_gaiji_slots <= first.patch.available_gaiji_slots);
    assert_eq!(
        first.patch.gaiji_headroom,
        first.patch.available_gaiji_slots - first.patch.used_gaiji_slots
    );
    assert_eq!(first.patch.system.storage_capacity, 407);
    assert_eq!(first.patch.system.packed_storage_bytes, 285);
    assert_eq!(first.patch.system.storage_headroom, 122);
    assert_eq!(first.patch.system.semantic_reference_count, 23);
    assert_eq!(first.patch.system.storage_reference_count, 24);
    assert_eq!(first.patch.system.entries.len(), 10);
    assert_eq!(first.patch.interface.storage_capacity, 1_166);
    assert_eq!(first.patch.interface.packed_storage_bytes, 1_042);
    assert_eq!(first.patch.interface.storage_headroom, 124);
    assert_eq!(first.patch.interface.reference_count, 139);
    assert_eq!(first.patch.interface.entries.len(), 88);
    // The combined plan owns both text regions and every shared conditional
    // window write needed to display the current interface corpus.
    assert!(
        first
            .patch
            .writes
            .iter()
            .any(|w| w.id == "stage-result-text-origin-state-12")
    );
    for state in ["12", "13"] {
        assert!(first.patch.writes.iter().any(|write| write.id
            == format!("stage-result-text-origin-state-{state}")
            && write.intent == "machine_code"));
    }
}

#[test]
#[ignore = "requires DS5_DISK1 pointing to a user-owned supported source image"]
fn supported_source_isolated_dialogue_builder_rejects_uninstalled_codes() {
    let source = env::var_os("DS5_DISK1").expect("set DS5_DISK1 to the supported Disk 1 HDM");
    let original = fs::read(&source).unwrap();
    let directory = tempfile::tempdir().unwrap();
    let first_path = directory.path().join("first-dialogue-text.hdm");
    let second_path = directory.path().join("second-dialogue-text.hdm");
    let translations = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/translations");

    let first = build_dialogue_text_development_image(source.as_ref(), &translations, &first_path)
        .unwrap_err();
    let second =
        build_dialogue_text_development_image(source.as_ref(), &translations, &second_path)
            .unwrap_err();

    assert_eq!(fs::read(&source).unwrap(), original);
    assert!(!first_path.exists());
    assert!(!second_path.exists());
    assert_eq!(format!("{first:#}"), format!("{second:#}"));
    assert!(
        format!("{first:#}")
            .contains("needs 172 GAIJI glyphs but only 170 verified character slots are available")
    );
}

#[test]
#[ignore = "requires DS5_DISK1 pointing to a user-owned supported source image"]
fn supported_source_mad_scene_text_build_does_not_publish_when_the_image_is_full() {
    let source = env::var_os("DS5_DISK1").expect("set DS5_DISK1 to the supported Disk 1 HDM");
    let original = fs::read(&source).unwrap();
    let directory = tempfile::tempdir().unwrap();
    let first_path = directory.path().join("first-mad-scene-text.hdm");
    let second_path = directory.path().join("second-mad-scene-text.hdm");
    let translations = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/translations");

    let first = build_mad_scene_text_development_image(source.as_ref(), &translations, &first_path)
        .unwrap_err();
    let second =
        build_mad_scene_text_development_image(source.as_ref(), &translations, &second_path)
            .unwrap_err();

    assert_eq!(fs::read(&source).unwrap(), original);
    assert!(!first_path.exists());
    assert!(!second_path.exists());
    assert_eq!(format!("{first:#}"), format!("{second:#}"));
    assert!(format!("{first:#}").contains("No space left on device"));
}

#[test]
#[ignore = "requires DS5_DISK1 pointing to a user-owned supported source image"]
fn supported_source_mad_scene_narrative_build_is_repeatable_and_read_back() {
    let source = env::var_os("DS5_DISK1").expect("set DS5_DISK1 to the supported Disk 1 HDM");
    let original = fs::read(&source).unwrap();
    let directory = tempfile::tempdir().unwrap();
    let first_path = directory.path().join("first-mad-scene-narrative.hdm");
    let second_path = directory.path().join("second-mad-scene-narrative.hdm");
    let translations = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/translations");

    let first =
        build_mad_scene_narrative_development_image(source.as_ref(), &translations, &first_path)
            .unwrap();
    let second =
        build_mad_scene_narrative_development_image(source.as_ref(), &translations, &second_path)
            .unwrap();

    assert_eq!(fs::read(&source).unwrap(), original);
    assert_eq!(
        fs::read(first_path).unwrap(),
        fs::read(second_path).unwrap()
    );
    assert_eq!(first, second);
    assert_eq!(first.build.file_count, 73);
    assert_eq!(first.patch.scene_text.unit_list_status.entry_count, 10);
    let window = &first.patch.scene_text.interface.stage_result_window;
    assert_eq!(window.output_frame_inner_width_words, 16);
    assert_eq!(window.output_text_position, 0x370e);
    assert_eq!(window.output_background_invalidation_x_pixels, 0x60);
    assert_eq!(
        window.text_position_instruction_file_offsets,
        [0x57db, 0x57fb]
    );
    assert_eq!(first.patch.monochrome.writes.len(), 6);
    assert_eq!(first.patch.baked.writes.len(), 3);
    assert!(
        first
            .patch
            .writes
            .iter()
            .any(|w| w.id == "unit-list-status-bank-wrapper")
    );
    assert_eq!(first.patch.monochrome.sequences[0].used_glyph_count, 147);
    assert_eq!(first.patch.monochrome.sequences[1].used_glyph_count, 78);
    assert_eq!(first.patch.baked.assets.len(), 3);
    assert_eq!(first.patch.scene_text.transition.call_site_count, 2);
    assert_eq!(first.patch.scene_text.fixed.used_gaiji_slots, 123);
    assert_eq!(
        first.patch.scene_text.translation_status,
        "needs_human_review"
    );
}

#[test]
#[ignore = "requires DS5_DISK1 and BAYOEN_TITLE_SOURCE_PREVIEW"]
fn supported_source_in_game_narrative_with_title_is_repeatable_and_read_back() {
    let source = env::var_os("DS5_DISK1").expect("set DS5_DISK1 to the supported Disk 1 HDM");
    let source_preview = env::var_os("BAYOEN_TITLE_SOURCE_PREVIEW")
        .expect("set BAYOEN_TITLE_SOURCE_PREVIEW to the source-extracted title PNG");
    let original = fs::read(&source).unwrap();
    let directory = tempfile::tempdir().unwrap();
    let first_path = directory.path().join("first-in-game-narrative.hdm");
    let second_path = directory.path().join("second-in-game-narrative.hdm");
    let project_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let translations = project_root.join("assets/translations");
    let manifest = project_root.join("assets/title_art/development-title-art.json");
    let artwork = project_root.join("assets/title_art/title-authored-frame.png");

    let first = build_mad_scene_narrative_with_title_artwork_development_image(
        source.as_ref(),
        &translations,
        &manifest,
        source_preview.as_ref(),
        &artwork,
        &first_path,
    )
    .unwrap();
    let second = build_mad_scene_narrative_with_title_artwork_development_image(
        source.as_ref(),
        &translations,
        &manifest,
        source_preview.as_ref(),
        &artwork,
        &second_path,
    )
    .unwrap();

    assert_eq!(fs::read(&source).unwrap(), original);
    assert_eq!(
        fs::read(&first_path).unwrap(),
        fs::read(&second_path).unwrap()
    );
    assert_eq!(first, second);
    assert_eq!(first.build.file_count, 73);
    assert_eq!(
        first.patch.scene_text.system_text_disposition,
        MadSystemTextDisposition::PreservedSource
    );
    assert!(
        first
            .patch
            .scene_text
            .system
            .entries
            .iter()
            .all(|entry| entry.file_offset == entry.original_file_offset)
    );
    assert_eq!(
        first
            .patch
            .writes
            .iter()
            .map(|write| write.file_name.as_str())
            .collect::<BTreeSet<_>>(),
        BTreeSet::from([
            "EDM.DAT",
            "GAIJI.COM",
            "MAD.COM",
            "OPM.DAT",
            "SEL1.DAT",
            "SEL3.DAT",
            "TITLE.DAT",
        ])
    );
    assert_eq!(first.patch.scene_text.dialogue.used_gaiji_slots, 172);
    assert_eq!(first.patch.scene_text.dialogue.available_gaiji_slots, 174);
    assert_eq!(first.patch.monochrome.sequences[0].used_glyph_count, 147);
    assert_eq!(first.patch.monochrome.sequences[1].used_glyph_count, 78);
    assert!(first.patch.baked.title_artwork.is_some());
}

#[test]
#[ignore = "requires DS5_DISK1 and BAYOEN_TITLE_SOURCE_PREVIEW"]
fn supported_source_title_artwork_build_is_repeatable_and_read_back() {
    let source = env::var_os("DS5_DISK1").expect("set DS5_DISK1 to the supported Disk 1 HDM");
    let source_preview = env::var_os("BAYOEN_TITLE_SOURCE_PREVIEW")
        .expect("set BAYOEN_TITLE_SOURCE_PREVIEW to the source-extracted title PNG");
    let original = fs::read(&source).unwrap();
    let directory = tempfile::tempdir().unwrap();
    let first_path = directory.path().join("first-title-artwork.hdm");
    let second_path = directory.path().join("second-title-artwork.hdm");
    let project_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let artwork = project_root.join("assets/title_art/title-authored-frame.png");
    let translations = project_root.join("assets/translations");
    let manifest = project_root.join("assets/title_art/development-title-art.json");

    let first = build_title_artwork_development_image(
        source.as_ref(),
        &translations,
        &manifest,
        source_preview.as_ref(),
        &artwork,
        &first_path,
    )
    .unwrap();
    let second = build_title_artwork_development_image(
        source.as_ref(),
        &translations,
        &manifest,
        source_preview.as_ref(),
        &artwork,
        &second_path,
    )
    .unwrap();

    assert_eq!(fs::read(&source).unwrap(), original);
    assert_eq!(
        fs::read(first_path).unwrap(),
        fs::read(second_path).unwrap()
    );
    assert_eq!(first, second);
    assert_eq!(first.build.file_count, 73);
    assert_eq!(first.patch.baked.writes.len(), 5);

    let title = first
        .patch
        .baked
        .title_artwork
        .expect("the dedicated build must report its title artwork");
    assert_eq!(title.input_kind, "source_extracted_active_frame");
    assert_eq!(title.artwork_kind, "project_authored_consumer_frame");
    assert_eq!(
        title.conversion_method,
        "pixel_exact_rgb_crop_without_background_separation"
    );
    assert_eq!(title.artwork_width, 1280);
    assert_eq!(title.artwork_height, 800);
    assert_eq!(title.crop_x, 0);
    assert_eq!(title.crop_y, 0);
    assert_eq!(title.preserved_component_region_count, 0);
    assert_eq!(
        title.conversion_output_rgb_sha256,
        "e9da7939ece4a801557060785926ade8e4ea20e853790c6f1d3bddad1182f89f"
    );
    assert_eq!(title.output_width, 640);
    assert_eq!(title.output_height, 400);
    assert_eq!(title.content_transfer_count, 4);
    assert_eq!(title.title_animation_source_offset, 0x0c00);
    assert_eq!(title.background_palette_indices, [0, 1]);
    assert_eq!(
        title.background_policy,
        "restore_source_background_texture_by_edge_connectivity"
    );
    assert_eq!(
        title.outside_transfer_policy,
        "restore_source_for_neutral_backing_only"
    );
    assert_eq!(
        title.outside_transfer_trimmable_palette_indices,
        [2, 3, 4, 5, 6]
    );
    assert_eq!(title.generation_supporting_reference_count, 1);
    assert_eq!(
        (
            title.source_visible_x,
            title.source_visible_y,
            title.source_visible_width,
            title.source_visible_height,
        ),
        (70, 13, 500, 307)
    );
    assert_eq!(
        (
            title.replacement_visible_x,
            title.replacement_visible_y,
            title.replacement_visible_width,
            title.replacement_visible_height,
        ),
        (66, 16, 494, 304)
    );
    assert_eq!(title.edge_connected_background_pixels_preserved, 25_569);
    assert_eq!(
        title.edge_connected_background_pixel_indices_restored,
        11_806
    );
    assert_eq!(title.artwork_owned_background_pixels_retained, 10_887);
    assert_eq!(title.source_logo_pixels_replaced_with_background, 21_036);
    assert_eq!(title.source_pixels_restored_outside_transfers, 600);
    assert_eq!(
        &title.runtime_palette_rgb4[..3],
        &[[0, 0, 0], [6, 0, 0], [15, 15, 15]]
    );
    assert!(!title.title2_animation_enabled);
    assert!(title.title2_animation_end_fallback_disabled);
    assert!(title.replacement_packed_size <= title.maximum_packed_size);
    assert!(title.replacement_decode_command_count <= title.maximum_decode_command_count);
    assert_eq!(title.approval_status, "needs_human_review");
}

#[test]
#[ignore = "requires DS5_DISK1 and BAYOEN_TITLE_SOURCE_PREVIEW"]
fn supported_source_integrated_localization_build_covers_every_target_file() {
    let source = env::var_os("DS5_DISK1").expect("set DS5_DISK1 to the supported Disk 1 HDM");
    let source_preview = env::var_os("BAYOEN_TITLE_SOURCE_PREVIEW")
        .expect("set BAYOEN_TITLE_SOURCE_PREVIEW to the source-extracted title PNG");
    let original = fs::read(&source).unwrap();
    let directory = tempfile::tempdir().unwrap();
    let first_path = directory.path().join("first-integrated-localization.hdm");
    let second_path = directory.path().join("second-integrated-localization.hdm");
    let directory_assets_path = directory.path().join("directory-assets-localization.hdm");
    let original_arle_path = directory.path().join("original-arle-localization.hdm");
    let project_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let artwork = project_root.join("assets/title_art/title-authored-frame.png");
    let translations = project_root.join("assets/translations");
    let manifest = project_root.join("assets/title_art/development-title-art.json");
    let tracked_arle = ArleCharacterAssetSelection::tracked();
    let directory_arle =
        ArleCharacterAssetSelection::from_directory(project_root.join("assets/characters/arle"));
    let original_arle = ArleCharacterAssetSelection::preserve_original();

    let first = build_integrated_localization_development_image(
        source.as_ref(),
        IntegratedLocalizationDevelopmentInputs {
            translations: &translations,
            title_artwork_manifest: &manifest,
            title_source_preview: source_preview.as_ref(),
            title_artwork: &artwork,
            arle_character_assets: &tracked_arle,
        },
        &first_path,
    )
    .unwrap();
    let second = build_integrated_localization_development_image(
        source.as_ref(),
        IntegratedLocalizationDevelopmentInputs {
            translations: &translations,
            title_artwork_manifest: &manifest,
            title_source_preview: source_preview.as_ref(),
            title_artwork: &artwork,
            arle_character_assets: &tracked_arle,
        },
        &second_path,
    )
    .unwrap();
    let directory_assets = build_integrated_localization_development_image(
        source.as_ref(),
        IntegratedLocalizationDevelopmentInputs {
            translations: &translations,
            title_artwork_manifest: &manifest,
            title_source_preview: source_preview.as_ref(),
            title_artwork: &artwork,
            arle_character_assets: &directory_arle,
        },
        &directory_assets_path,
    )
    .unwrap();
    let original_arle_build = build_integrated_localization_development_image(
        source.as_ref(),
        IntegratedLocalizationDevelopmentInputs {
            translations: &translations,
            title_artwork_manifest: &manifest,
            title_source_preview: source_preview.as_ref(),
            title_artwork: &artwork,
            arle_character_assets: &original_arle,
        },
        &original_arle_path,
    )
    .unwrap();

    assert_eq!(fs::read(&source).unwrap(), original);
    assert_eq!(
        fs::read(&first_path).unwrap(),
        fs::read(&second_path).unwrap()
    );
    assert_eq!(first, second);
    assert_eq!(
        fs::read(&first_path).unwrap(),
        fs::read(directory_assets_path).unwrap()
    );
    assert_eq!(first, directory_assets);
    assert_ne!(
        fs::read(&first_path).unwrap(),
        fs::read(original_arle_path).unwrap()
    );
    assert_eq!(first.build.file_count, 73);
    assert_eq!(
        first.patch.build_status,
        DevelopmentBuildStatus::DevelopmentOnly
    );
    assert_eq!(first.patch.translation_status, "needs_human_review");
    assert_eq!(first.patch.translation_unit_count, 417);
    assert_eq!(first.patch.translated_text_unit_count, 392);
    assert_eq!(first.patch.preserved_source_control_unit_count, 3);
    assert_eq!(first.patch.retained_source_audio_unit_count, 22);
    assert_eq!(first.patch.target_file_count, 17);
    assert_eq!(first.patch.modified_file_count, 16);
    assert_eq!(first.patch.retained_source_audio_file_count, 1);
    assert!(first.patch.title_artwork.is_some());
    let arle_replacement = first.patch.arle_character_asset.replacement().unwrap();
    assert_eq!(arle_replacement.frame_count, 8);
    assert_eq!(arle_replacement.character_slot_index, 6);
    assert_eq!(first.patch.components.len(), 8);
    assert_eq!(first.patch.composition_writes.len(), 16);
    assert_eq!(
        first
            .patch
            .font_profiles
            .iter()
            .map(|font| font.consumer_target.as_str())
            .collect::<Vec<_>>(),
        ["body16", "narrative32", "baked-display16", "difficulty32",]
    );

    let target_files = first
        .patch
        .files
        .iter()
        .map(|file| file.file_name.as_str())
        .collect::<BTreeSet<_>>();
    assert_eq!(
        target_files,
        BTreeSet::from([
            "BPLAY6.COM",
            "BSAMP.COM",
            "C07",
            "C08",
            "DSH.COM",
            "EDM.DAT",
            "FPLAY6.COM",
            "GAIJI.COM",
            "MAD.COM",
            "MEGDOS.SYS",
            "MENU.COM",
            "NMOUSE.COM",
            "OPM.DAT",
            "SAMPA",
            "SEL1.DAT",
            "SEL3.DAT",
            "TITLE.DAT",
        ])
    );
    let retained = first
        .patch
        .files
        .iter()
        .filter(|file| {
            file.disposition == IntegratedLocalizationFileDisposition::RetainedSourceAudio
        })
        .collect::<Vec<_>>();
    assert_eq!(retained.len(), 1);
    assert_eq!(retained[0].file_name, "SAMPA");
    let composition_files = first
        .patch
        .composition_writes
        .iter()
        .map(|write| write.file_name.as_str())
        .collect::<BTreeSet<_>>();
    let modified_files = first
        .patch
        .files
        .iter()
        .filter(|file| {
            file.disposition == IntegratedLocalizationFileDisposition::PatchedDevelopment
        })
        .map(|file| file.file_name.as_str())
        .collect::<BTreeSet<_>>();
    assert_eq!(composition_files, modified_files);

    assert_eq!(original_arle_build.patch.target_file_count, 17);
    assert_eq!(original_arle_build.patch.modified_file_count, 14);
    assert_eq!(original_arle_build.patch.composition_writes.len(), 14);
    assert!(
        original_arle_build
            .patch
            .arle_character_asset
            .original_is_preserved()
    );
    let preserved_arle_files = original_arle_build
        .patch
        .files
        .iter()
        .filter(|file| {
            file.disposition == IntegratedLocalizationFileDisposition::PreservedOriginalAsset
        })
        .map(|file| file.file_name.as_str())
        .collect::<BTreeSet<_>>();
    assert_eq!(preserved_arle_files, BTreeSet::from(["C07", "C08"]));
}
