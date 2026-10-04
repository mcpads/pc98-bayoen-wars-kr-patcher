use super::*;

fn project_authored_artwork() -> ArtworkInput {
    ArtworkInput {
        kind: ARTWORK_KIND.to_owned(),
        sha256: "23".repeat(32),
        width: 1280,
        height: 800,
        color_mode: MASTER_COLOR_MODE.to_owned(),
        conversion: ArtworkConversionInput {
            source_kind: CONVERSION_SOURCE_KIND.to_owned(),
            source_sha256: "59".repeat(32),
            source_width: 1612,
            source_height: 976,
            source_region: CropInput {
                x: 143,
                y: 12,
                width: 1280,
                height: 800,
            },
            output_rgb_sha256: "e9".repeat(32),
            method: CONVERSION_METHOD.to_owned(),
        },
        generation_supporting_references: vec![GenerationSupportingReferenceInput {
            role: "character_identity_anatomy_only_not_edit_target".to_owned(),
            page_url: "https://example.com/character".to_owned(),
            asset_url: "https://example.com/character.png".to_owned(),
            sha256: "4f".repeat(32),
        }],
        crop: CropInput {
            x: 0,
            y: 0,
            width: 1280,
            height: 800,
        },
        preserved_component_regions: Vec::new(),
        translation_draft_sha256: "4b".repeat(32),
    }
}

#[test]
fn project_authored_frame_preserves_the_full_generation_master_crop_lineage() {
    validate_project_authored_artwork(&project_authored_artwork()).unwrap();

    let mut partial_frame = project_authored_artwork();
    partial_frame.crop.x = 1;
    partial_frame.crop.width -= 1;
    let error = validate_project_authored_artwork(&partial_frame).unwrap_err();

    assert!(error.to_string().contains("complete consumer frame"));
}

#[test]
fn artwork_crop_accepts_an_asymmetric_logo_framing_without_stretching_it() {
    let crop = CropInput {
        x: 143,
        y: 12,
        width: 1280,
        height: 800,
    };

    validate_artwork_crop(&crop, 1612, 976).unwrap();

    let square = CropInput {
        x: 0,
        y: 0,
        width: 400,
        height: 400,
    };
    let error = validate_artwork_crop(&square, 400, 400).unwrap_err();

    assert!(error.to_string().contains("differs from the 8:5"));
}

#[test]
fn sampling_placement_preserves_the_complete_frame_aspect_ratio() {
    let placement = SamplingPlacementInput {
        x: 32,
        y: 0,
        width: 256,
        height: 160,
    };

    validate_sampling_placement(&placement, 320, 200).unwrap();

    let distorted = SamplingPlacementInput {
        x: 32,
        y: 0,
        width: 256,
        height: 161,
    };
    let error = validate_sampling_placement(&distorted, 320, 200).unwrap_err();

    assert!(error.to_string().contains("preserve the 8:5"));
}

#[test]
fn manifest_parser_rejects_unrecognized_fields() {
    let error = serde_json::from_slice::<TitleArtworkManifest>(
        br#"{
            "id":"title",
            "build_status":"development_only",
            "unexpected":true,
            "input":{},
            "artwork":{},
            "consumer":{}
        }"#,
    )
    .err()
    .unwrap();

    assert!(error.to_string().contains("unknown field `unexpected`"));
}
