use super::file_population::AssetReviewStatus;

pub(super) struct AssetReview {
    pub(super) status: AssetReviewStatus,
    pub(super) evidence: Vec<String>,
}

pub(super) fn review_asset(name: &str) -> AssetReview {
    let (status, evidence) = match name {
        "MAD.COM" => (
            AssetReviewStatus::ConfirmedLocalizationTarget,
            "consumer-linked dialogue, fixed text, system text, and interface text reside in this program",
        ),
        "GAIJI.COM" => (
            AssetReviewStatus::ConfirmedLocalizationTarget,
            "the game installs and consumes this 184-glyph custom character set",
        ),
        "MENU.COM" => (
            AssetReviewStatus::ConfirmedLocalizationTarget,
            "the launcher contains user-visible Japanese DOS and disk error text",
        ),
        "BPLAY6.COM" | "FPLAY6.COM" => (
            AssetReviewStatus::ConfirmedLocalizationTarget,
            "the verified self-expanding COM image contains consumer-referenced Japanese DOS messages and time-of-day text",
        ),
        "BSAMP.COM" => (
            AssetReviewStatus::ConfirmedLocalizationTarget,
            "direct DOS output consumers reference a Japanese banner and resident/unload status text",
        ),
        "NMOUSE.COM" => (
            AssetReviewStatus::ConfirmedLocalizationTarget,
            "direct DOS output consumers and two pointer tables reference Japanese banner, help, mode, and status text",
        ),
        "MEGDOS.SYS" => (
            AssetReviewStatus::ConfirmedLocalizationTarget,
            "the system loader references seven null-terminated Japanese boot and CONFIG.SYS error messages",
        ),
        "IO98.SYS" => (
            AssetReviewStatus::EvidenceBasedExclusion,
            "the complete system image binds its only consumer-linked text surface to fourteen ASCII interrupt diagnostics, eight ASCII flag-state pairs, and four built-in DOS device names; none is Japanese",
        ),
        "DSH.COM" => (
            AssetReviewStatus::ConfirmedLocalizationTarget,
            "eight direct DOS-output consumers reference the shell banner, interrupt error, memory error, and execution-error messages in Japanese",
        ),
        "TITLE.DAT" => (
            AssetReviewStatus::ConfirmedLocalizationTarget,
            "the consumer-proven base-title layout contains baked Japanese title lettering",
        ),
        "SEL1.DAT" => (
            AssetReviewStatus::ConfirmedLocalizationTarget,
            "the consumer-proven tile map contains three baked Japanese difficulty labels",
        ),
        "SEL3.DAT" => (
            AssetReviewStatus::ConfirmedLocalizationTarget,
            "consumer-proven overlays contain ten baked Japanese character names and a stage-selection heading",
        ),
        "OPM.DAT" => (
            AssetReviewStatus::ConfirmedLocalizationTarget,
            "MAD.COM indexes all 129 consumer-bound 32x32 monochrome records as visible Japanese opening-text glyphs",
        ),
        "EDM.DAT" => (
            AssetReviewStatus::ConfirmedLocalizationTarget,
            "MAD.COM indexes all 69 consumer-bound 32x32 monochrome records as visible Japanese ending-text glyphs",
        ),
        "OP1.DAT" | "OP2.DAT" | "OP3.DAT" | "ED1.DAT" | "ED2.DAT" => (
            AssetReviewStatus::EvidenceBasedExclusion,
            "the complete consumer-proven B/R/G/I scene sheet contains illustration only and no baked text",
        ),
        "SEL2.DAT" => (
            AssetReviewStatus::EvidenceBasedExclusion,
            "all ten consumer-proven 128x96 overlays are character portraits without baked text",
        ),
        "TITLE2.DAT" => (
            AssetReviewStatus::EvidenceBasedExclusion,
            "the consumer-proven 64x48 title animation overlay contains artwork only; Japanese lettering resides in TITLE.DAT",
        ),
        "DEFEAT.DAT" => (
            AssetReviewStatus::EvidenceBasedExclusion,
            "all 21 consumer-proven transfers consume the asset as character defeat artwork without baked text",
        ),
        "DATE_.DAT" => (
            AssetReviewStatus::EvidenceBasedExclusion,
            "all decoded bytes partition into seventeen 8x16 numeric or punctuation glyphs without Japanese text",
        ),
        "CONFIG.SYS" => (
            AssetReviewStatus::EvidenceBasedExclusion,
            "the complete file is a DOS boot configuration containing only files, buffers, and shell directives",
        ),
        "DATA.DAT" => (
            AssetReviewStatus::EvidenceBasedExclusion,
            "the complete nine-byte file is a binary state record with no text storage",
        ),
        "DMADOU.BAT" => (
            AssetReviewStatus::EvidenceBasedExclusion,
            "the complete batch file contains only program names, switches, and CRLF separators",
        ),
        "C00" | "C01" | "C02" | "C03" | "C04" | "C05" | "C06" | "C07" | "C08" | "C09" | "C10"
        | "C11" | "C12" | "C13" | "C15" | "C16" | "C17" | "C18" | "C19" | "C20" | "C21" => (
            AssetReviewStatus::EvidenceBasedExclusion,
            "MAD.COM's dynamic slot table and B/R/G/I renderer bind the reachable decoded ranges as character sprites; exact transfer previews contain artwork only, and the two residual ranges are unreachable",
        ),
        "BWM1.DAT" | "BWM2.DAT" | "BWM3.DAT" | "BWM4.DAT" | "BWM5.DAT" | "BWM6.DAT"
        | "BWM7.DAT" | "BWM8.DAT" | "BWM9.DAT" | "BWM10.DAT" => (
            AssetReviewStatus::EvidenceBasedExclusion,
            "MAD.COM binds every decoded byte to battle-map overview pixels or map state; all ten exact 160x40 B/R/G/I overview previews contain terrain and unit symbols without Japanese text",
        ),
        "MAPB.DAT" => (
            AssetReviewStatus::EvidenceBasedExclusion,
            "the complete packed file is byte-identical to consumer-bound BWM10.DAT and is an unused installer-archive duplicate",
        ),
        "MOUSE.DAT" => (
            AssetReviewStatus::EvidenceBasedExclusion,
            "MAD.COM binds the complete asset to 56 mask-plus-B/R/G/I directional markers and one cursor; exact record previews contain symbols only",
        ),
        "WAKU_P.DAT" => (
            AssetReviewStatus::EvidenceBasedExclusion,
            "all 182 consumer-bound 16x16 B/R/G/I records are interface frames, borders, gauges, and symbols without Japanese text",
        ),
        "WAKU_IMG.DAT" => (
            AssetReviewStatus::EvidenceBasedExclusion,
            "the complete file is one pointer followed by the consumer-bound 10x25 WAKU_P.DAT tile map and contains no text storage",
        ),
        "BG__.DAT" => (
            AssetReviewStatus::EvidenceBasedExclusion,
            "all 256 consumer-indexed 16x16 B/R/G/I records are terrain and background tiles without baked text",
        ),
        "KAO" => (
            AssetReviewStatus::EvidenceBasedExclusion,
            "all eighteen consumer-selected 64x64 B/R/G/I records are character portraits without baked text",
        ),
        "B04" | "B05" => (
            AssetReviewStatus::EvidenceBasedExclusion,
            "MAD.COM's loader, selection table, and renderer bind all eleven 128x128 B/R/G/I records as character battle backgrounds without baked text",
        ),
        "BO.DAT" => (
            AssetReviewStatus::EvidenceBasedExclusion,
            "MAD.COM binds 24 mask-plus-B/R/G/I 48x48 battle-object sprites without Japanese text; the final 128 bytes are unreachable",
        ),
        "UN.DAT" => (
            AssetReviewStatus::EvidenceBasedExclusion,
            "all decoded bytes form 24 consumer-bound mask-plus-B/R/G/I 48x48 unit sprites without Japanese text",
        ),
        "CAR.DAT" => (
            AssetReviewStatus::EvidenceBasedExclusion,
            "all decoded bytes form five consumer-bound mask-plus-B/R/G/I 48x48 effect sprites without Japanese text",
        ),
        "BW.DAT" => (
            AssetReviewStatus::EvidenceBasedExclusion,
            "MAD.COM's eleven contiguous B/R/G/I transfers consume the complete file as decorative battle-window artwork without baked text",
        ),
        "ST" => (
            AssetReviewStatus::EvidenceBasedExclusion,
            "the complete decoded file is a 12-word pointer header plus five word-aligned movement-table sections consumed as numeric displacements and thresholds",
        ),
        "SONG.DAT" => (
            AssetReviewStatus::EvidenceBasedExclusion,
            "FPLAY6.COM references the file name exactly once and passes it to resident sound service AH=09/INT 7F; the fixed-size music sequence has an ASCII composer signature and no user-visible text consumer",
        ),
        "SAMPA" => (
            AssetReviewStatus::ConfirmedLocalizationTarget,
            "all 22 consumer-linked streams replay at the BSAMP.COM-proven 9,600 Hz rate and contain Japanese spoken phrases or character vocalizations; exact transcription and Korean dubbing remain independent review work",
        ),
        _ => (
            AssetReviewStatus::Unresolved,
            "awaiting consumer-linked text, graphics, audio, or exclusion evidence",
        ),
    };
    AssetReview {
        status,
        evidence: vec![evidence.to_owned()],
    }
}

#[cfg(test)]
#[path = "review_tests.rs"]
mod review_tests;
