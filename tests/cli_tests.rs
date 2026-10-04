use std::fs;
use std::process::Command;

#[test]
fn verify_source_rejects_an_unrecognized_disk() {
    let directory = tempfile::tempdir().unwrap();
    let source = directory.path().join("wrong.hdm");
    fs::write(&source, [0_u8; 16]).unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_bayoen-wars-builder"))
        .args(["verify-source", "--source"])
        .arg(&source)
        .output()
        .unwrap();

    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("unsupported source size"),
        "unexpected stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn survey_source_uses_the_supported_source_gate() {
    let directory = tempfile::tempdir().unwrap();
    let source = directory.path().join("wrong.hdm");
    fs::write(&source, [0_u8; 16]).unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_bayoen-wars-builder"))
        .args(["survey-source", "--source"])
        .arg(&source)
        .output()
        .unwrap();

    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("unsupported source size"),
        "unexpected stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn translation_workspace_uses_the_supported_source_gate() {
    let directory = tempfile::tempdir().unwrap();
    let source = directory.path().join("wrong.hdm");
    fs::write(&source, [0_u8; 16]).unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_bayoen-wars-builder"))
        .args(["extract-translation-workspace", "--source"])
        .arg(&source)
        .args(["--output-directory"])
        .arg(directory.path().join("translations"))
        .output()
        .unwrap();

    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("unsupported source size"),
        "unexpected stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn build_refuses_to_overwrite_an_existing_output() {
    let directory = tempfile::tempdir().unwrap();
    let output_path = directory.path().join("existing.hdm");
    fs::write(&output_path, b"keep me").unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_bayoen-wars-builder"))
        .args(["build", "--source", "missing.hdm", "--output"])
        .arg(&output_path)
        .output()
        .unwrap();

    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("refusing to overwrite existing output"),
        "unexpected stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(fs::read(output_path).unwrap(), b"keep me");
}

#[test]
fn arle_character_asset_build_refuses_to_overwrite_an_existing_output() {
    let directory = tempfile::tempdir().unwrap();
    let output_path = directory.path().join("existing.hdm");
    fs::write(&output_path, b"keep me").unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_bayoen-wars-builder"))
        .args([
            "build-arle-character-asset-development",
            "--source",
            "missing.hdm",
            "--output",
        ])
        .arg(&output_path)
        .output()
        .unwrap();

    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("refusing to overwrite existing output"),
        "unexpected stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(fs::read(output_path).unwrap(), b"keep me");
}

#[test]
fn playback_driver_text_build_refuses_to_overwrite_an_existing_output() {
    let directory = tempfile::tempdir().unwrap();
    let output_path = directory.path().join("existing.hdm");
    fs::write(&output_path, b"keep me").unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_bayoen-wars-builder"))
        .args([
            "build-playback-driver-text-development",
            "--source",
            "missing.hdm",
            "--output",
        ])
        .arg(&output_path)
        .output()
        .unwrap();

    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("refusing to overwrite existing output"),
        "unexpected stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(fs::read(output_path).unwrap(), b"keep me");
}

#[test]
fn menu_text_build_refuses_to_overwrite_an_existing_output() {
    let directory = tempfile::tempdir().unwrap();
    let output_path = directory.path().join("existing.hdm");
    fs::write(&output_path, b"keep me").unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_bayoen-wars-builder"))
        .args([
            "build-menu-text-development",
            "--source",
            "missing.hdm",
            "--output",
        ])
        .arg(&output_path)
        .output()
        .unwrap();

    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("refusing to overwrite existing output"),
        "unexpected stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(fs::read(output_path).unwrap(), b"keep me");
}

#[test]
fn hangul_visibility_build_refuses_to_overwrite_an_existing_output() {
    let directory = tempfile::tempdir().unwrap();
    let output_path = directory.path().join("existing.hdm");
    fs::write(&output_path, b"keep me").unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_bayoen-wars-builder"))
        .args([
            "build-hangul-visibility",
            "--source",
            "missing.hdm",
            "--output",
        ])
        .arg(&output_path)
        .output()
        .unwrap();

    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("refusing to overwrite existing output"),
        "unexpected stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(fs::read(output_path).unwrap(), b"keep me");
}

#[test]
fn monochrome_text_build_refuses_to_overwrite_an_existing_output() {
    let directory = tempfile::tempdir().unwrap();
    let output_path = directory.path().join("existing.hdm");
    fs::write(&output_path, b"keep me").unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_bayoen-wars-builder"))
        .args([
            "build-monochrome-text-development",
            "--source",
            "missing.hdm",
            "--output",
        ])
        .arg(&output_path)
        .output()
        .unwrap();

    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("refusing to overwrite existing output"),
        "unexpected stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(fs::read(output_path).unwrap(), b"keep me");
}

#[test]
fn narrative_graphics_build_refuses_to_overwrite_an_existing_output() {
    let directory = tempfile::tempdir().unwrap();
    let output_path = directory.path().join("existing.hdm");
    fs::write(&output_path, b"keep me").unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_bayoen-wars-builder"))
        .args([
            "build-narrative-graphics-development",
            "--source",
            "missing.hdm",
            "--output",
        ])
        .arg(&output_path)
        .output()
        .unwrap();

    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("refusing to overwrite existing output"),
        "unexpected stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(fs::read(output_path).unwrap(), b"keep me");
}

#[test]
fn title_artwork_build_refuses_to_overwrite_an_existing_output() {
    let directory = tempfile::tempdir().unwrap();
    let output_path = directory.path().join("existing.hdm");
    fs::write(&output_path, b"keep me").unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_bayoen-wars-builder"))
        .args([
            "build-title-artwork-development",
            "--source",
            "missing.hdm",
            "--source-preview",
            "missing-source.png",
            "--artwork",
            "missing-artwork.png",
            "--output",
        ])
        .arg(&output_path)
        .output()
        .unwrap();

    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("refusing to overwrite existing output"),
        "unexpected stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(fs::read(output_path).unwrap(), b"keep me");
}

#[test]
fn mad_scene_narrative_build_refuses_to_overwrite_an_existing_output() {
    let directory = tempfile::tempdir().unwrap();
    let output_path = directory.path().join("existing.hdm");
    fs::write(&output_path, b"keep me").unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_bayoen-wars-builder"))
        .args([
            "build-mad-scene-narrative-development",
            "--source",
            "missing.hdm",
            "--output",
        ])
        .arg(&output_path)
        .output()
        .unwrap();

    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("refusing to overwrite existing output"),
        "unexpected stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(fs::read(output_path).unwrap(), b"keep me");
}

#[test]
fn fixed_gaiji_text_build_refuses_to_overwrite_an_existing_output() {
    let directory = tempfile::tempdir().unwrap();
    let output_path = directory.path().join("existing.hdm");
    fs::write(&output_path, b"keep me").unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_bayoen-wars-builder"))
        .args([
            "build-fixed-gaiji-text-development",
            "--source",
            "missing.hdm",
            "--output",
        ])
        .arg(&output_path)
        .output()
        .unwrap();

    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("refusing to overwrite existing output"),
        "unexpected stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(fs::read(output_path).unwrap(), b"keep me");
}

#[test]
fn interface_text_build_refuses_to_overwrite_an_existing_output() {
    let directory = tempfile::tempdir().unwrap();
    let output_path = directory.path().join("existing.hdm");
    fs::write(&output_path, b"keep me").unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_bayoen-wars-builder"))
        .args([
            "build-interface-text-development",
            "--source",
            "missing.hdm",
            "--output",
        ])
        .arg(&output_path)
        .output()
        .unwrap();

    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("refusing to overwrite existing output"),
        "unexpected stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(fs::read(output_path).unwrap(), b"keep me");
}

#[test]
fn mad_system_text_build_refuses_to_overwrite_an_existing_output() {
    let directory = tempfile::tempdir().unwrap();
    let output_path = directory.path().join("existing.hdm");
    fs::write(&output_path, b"keep me").unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_bayoen-wars-builder"))
        .args([
            "build-mad-system-text-development",
            "--source",
            "missing.hdm",
            "--output",
        ])
        .arg(&output_path)
        .output()
        .unwrap();

    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("refusing to overwrite existing output"),
        "unexpected stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(fs::read(output_path).unwrap(), b"keep me");
}

#[test]
fn mad_system_interface_text_build_refuses_to_overwrite_an_existing_output() {
    let directory = tempfile::tempdir().unwrap();
    let output_path = directory.path().join("existing.hdm");
    fs::write(&output_path, b"keep me").unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_bayoen-wars-builder"))
        .args([
            "build-mad-system-interface-text-development",
            "--source",
            "missing.hdm",
            "--output",
        ])
        .arg(&output_path)
        .output()
        .unwrap();

    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("refusing to overwrite existing output"),
        "unexpected stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(fs::read(output_path).unwrap(), b"keep me");
}

#[test]
fn dialogue_text_build_refuses_to_overwrite_an_existing_output() {
    let directory = tempfile::tempdir().unwrap();
    let output_path = directory.path().join("existing.hdm");
    fs::write(&output_path, b"keep me").unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_bayoen-wars-builder"))
        .args([
            "build-dialogue-text-development",
            "--source",
            "missing.hdm",
            "--output",
        ])
        .arg(&output_path)
        .output()
        .unwrap();

    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("refusing to overwrite existing output"),
        "unexpected stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(fs::read(output_path).unwrap(), b"keep me");
}

#[test]
fn mad_scene_text_build_refuses_to_overwrite_an_existing_output() {
    let directory = tempfile::tempdir().unwrap();
    let output_path = directory.path().join("existing.hdm");
    fs::write(&output_path, b"keep me").unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_bayoen-wars-builder"))
        .args([
            "build-mad-scene-text-development",
            "--source",
            "missing.hdm",
            "--output",
        ])
        .arg(&output_path)
        .output()
        .unwrap();

    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("refusing to overwrite existing output"),
        "unexpected stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(fs::read(output_path).unwrap(), b"keep me");
}

#[test]
fn shell_text_build_refuses_to_overwrite_an_existing_output() {
    let directory = tempfile::tempdir().unwrap();
    let output_path = directory.path().join("existing.hdm");
    fs::write(&output_path, b"keep me").unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_bayoen-wars-builder"))
        .args([
            "build-shell-text-development",
            "--source",
            "missing.hdm",
            "--output",
        ])
        .arg(&output_path)
        .output()
        .unwrap();

    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("refusing to overwrite existing output"),
        "unexpected stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(fs::read(output_path).unwrap(), b"keep me");
}

#[test]
fn system_loader_text_build_refuses_to_overwrite_an_existing_output() {
    let directory = tempfile::tempdir().unwrap();
    let output_path = directory.path().join("existing.hdm");
    fs::write(&output_path, b"keep me").unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_bayoen-wars-builder"))
        .args([
            "build-system-loader-text-development",
            "--source",
            "missing.hdm",
            "--output",
        ])
        .arg(&output_path)
        .output()
        .unwrap();

    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("refusing to overwrite existing output"),
        "unexpected stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(fs::read(output_path).unwrap(), b"keep me");
}

#[test]
fn sampling_driver_text_build_refuses_to_overwrite_an_existing_output() {
    let directory = tempfile::tempdir().unwrap();
    let output_path = directory.path().join("existing.hdm");
    fs::write(&output_path, b"keep me").unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_bayoen-wars-builder"))
        .args([
            "build-sampling-driver-text-development",
            "--source",
            "missing.hdm",
            "--output",
        ])
        .arg(&output_path)
        .output()
        .unwrap();

    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("refusing to overwrite existing output"),
        "unexpected stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(fs::read(output_path).unwrap(), b"keep me");
}

#[test]
fn mouse_driver_text_build_refuses_to_overwrite_an_existing_output() {
    let directory = tempfile::tempdir().unwrap();
    let output_path = directory.path().join("existing.hdm");
    fs::write(&output_path, b"keep me").unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_bayoen-wars-builder"))
        .args([
            "build-mouse-driver-text-development",
            "--source",
            "missing.hdm",
            "--output",
        ])
        .arg(&output_path)
        .output()
        .unwrap();

    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("refusing to overwrite existing output"),
        "unexpected stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(fs::read(output_path).unwrap(), b"keep me");
}

#[test]
fn integrated_localization_build_refuses_to_overwrite_an_existing_output() {
    let directory = tempfile::tempdir().unwrap();
    let output_path = directory.path().join("existing.hdm");
    fs::write(&output_path, b"keep me").unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_bayoen-wars-builder"))
        .args([
            "build-integrated-localization-development",
            "--source",
            "missing.hdm",
            "--title-source-preview",
            "missing-source-title.png",
            "--title-artwork",
            "missing-generated-title.png",
            "--output",
        ])
        .arg(&output_path)
        .output()
        .unwrap();

    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("refusing to overwrite existing output"),
        "unexpected stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(fs::read(output_path).unwrap(), b"keep me");
}

#[test]
fn integrated_localization_rejects_custom_and_original_arle_together() {
    let directory = tempfile::tempdir().unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_bayoen-wars-builder"))
        .args([
            "build-integrated-localization-development",
            "--source",
            "missing.hdm",
            "--title-source-preview",
            "missing-source-title.png",
            "--arle-assets",
            "missing-assets",
            "--preserve-original-arle",
            "--output",
        ])
        .arg(directory.path().join("output.hdm"))
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("cannot be used with")
            && stderr.contains("--arle-assets")
            && stderr.contains("--preserve-original-arle"),
        "unexpected stderr: {stderr}"
    );
}
