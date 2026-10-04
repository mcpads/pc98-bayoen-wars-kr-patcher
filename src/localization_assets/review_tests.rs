use super::review_asset;
use crate::localization_assets::AssetReviewStatus;

#[test]
fn proven_program_and_baked_graphics_surfaces_are_targets() {
    for name in [
        "MAD.COM",
        "BPLAY6.COM",
        "DSH.COM",
        "MEGDOS.SYS",
        "TITLE.DAT",
        "SEL1.DAT",
        "SEL3.DAT",
        "OPM.DAT",
        "EDM.DAT",
        "SAMPA",
    ] {
        assert_eq!(
            review_asset(name).status,
            AssetReviewStatus::ConfirmedLocalizationTarget,
            "{name}"
        );
    }
}

#[test]
fn visually_complete_nontext_assets_are_excluded() {
    for name in [
        "OP1.DAT",
        "ED2.DAT",
        "SEL2.DAT",
        "DEFEAT.DAT",
        "DATE_.DAT",
        "C00",
        "C21",
        "BWM1.DAT",
        "BWM10.DAT",
        "MAPB.DAT",
        "MOUSE.DAT",
        "WAKU_P.DAT",
        "WAKU_IMG.DAT",
        "BG__.DAT",
        "KAO",
        "B04",
        "B05",
        "BO.DAT",
        "UN.DAT",
        "CAR.DAT",
        "BW.DAT",
        "ST",
        "SONG.DAT",
        "IO98.SYS",
        "CONFIG.SYS",
        "DATA.DAT",
        "DMADOU.BAT",
    ] {
        assert_eq!(
            review_asset(name).status,
            AssetReviewStatus::EvidenceBasedExclusion,
            "{name}"
        );
    }
}

#[test]
fn unknown_files_remain_unresolved() {
    assert_eq!(
        review_asset("UNKNOWN.DAT").status,
        AssetReviewStatus::Unresolved
    );
}
