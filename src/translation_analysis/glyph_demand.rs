use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use anyhow::Result;
use encoding_rs::SHIFT_JIS;

use super::load_translation_corpus;
use super::model::{
    TranslationAnalysisStatus, TranslationGlyphAuditReport, TranslationGlyphDemand,
    TranslationGlyphPathReport, TranslationGlyphSegmentReport, TranslationRenderPath,
};
use crate::game_data::{GAIJI_READINESS_GLYPH_COUNT, GRAPHIC_GLYPH_COUNT};
use crate::korean_patch::DIALOGUE_GAIJI_PUNCTUATION;
use crate::translation_drafts::{DevelopmentPolicy, TranslationSurface};

use super::shared_gaiji_sets::audit_shared_gaiji_demand_sets;
use crate::asset_bindings::MONOCHROME_USABLE_GLYPH_COUNT;

const SHARED_GAIJI_CAPACITY: usize = 184;

pub fn audit_translation_glyphs(directory: &Path) -> Result<TranslationGlyphAuditReport> {
    let corpus = load_translation_corpus(directory)?;
    let mut path_demands = BTreeMap::<TranslationRenderPath, DemandAccumulator>::new();
    let mut segment_reports = Vec::new();
    for segment in &corpus.segments {
        let Some(render_path) = render_path_for(segment.surface) else {
            continue;
        };
        let mut demand = DemandAccumulator::default();
        for entry in &segment.entries {
            if entry.development_policy != DevelopmentPolicy::Translate {
                continue;
            }
            demand.add_translation(&entry.korean_text);
        }
        if segment.surface == TranslationSurface::Dialogue {
            demand.force_shared_gaiji(&DIALOGUE_GAIJI_PUNCTUATION);
        }
        path_demands.entry(render_path).or_default().merge(&demand);
        segment_reports.push(TranslationGlyphSegmentReport {
            id: segment.id.clone(),
            render_path,
            entry_count: demand.entry_count,
            text_span_count: demand.text_span_count,
            demand: demand.report(render_path),
        });
    }
    let render_paths = TranslationRenderPath::ALL
        .iter()
        .copied()
        .map(|render_path| {
            let demand = path_demands.remove(&render_path).unwrap_or_default();
            TranslationGlyphPathReport {
                render_path,
                entry_count: demand.entry_count,
                text_span_count: demand.text_span_count,
                demand: demand.report(render_path),
            }
        })
        .collect();
    Ok(TranslationGlyphAuditReport {
        supported_source_sha256: corpus.index.supported_source_sha256,
        status: TranslationAnalysisStatus::NeedsHumanReview,
        translation_unit_count: corpus.index.translation_unit_count,
        translated_text_unit_count: corpus.index.translated_text_unit_count,
        preserved_source_control_unit_count: corpus.index.preserved_source_control_unit_count,
        retained_source_audio_unit_count: corpus.index.retained_source_audio_unit_count,
        render_paths,
        shared_gaiji_demand_sets: audit_shared_gaiji_demand_sets(&corpus.segments),
        segments: segment_reports,
    })
}

fn render_path_for(surface: TranslationSurface) -> Option<TranslationRenderPath> {
    match surface {
        TranslationSurface::DosSystemText
        | TranslationSurface::InterfaceText
        | TranslationSurface::BattleCallout
        | TranslationSurface::FixedGaijiText
        | TranslationSurface::Dialogue
        | TranslationSurface::MenuText
        | TranslationSurface::ExternalProgramText => Some(TranslationRenderPath::SharedPc98Gaiji),
        TranslationSurface::OpeningGlyphIndexPages => {
            Some(TranslationRenderPath::OpeningMonochromeSprites)
        }
        TranslationSurface::EndingGlyphIndexPages => {
            Some(TranslationRenderPath::EndingMonochromeSprites)
        }
        TranslationSurface::BakedGraphicsText => Some(TranslationRenderPath::BakedPlanarGraphics),
        TranslationSurface::MonochromeGlyphAtlas | TranslationSurface::JapaneseVoicePerformance => {
            None
        }
    }
}

impl TranslationRenderPath {
    const ALL: [Self; 4] = [
        Self::SharedPc98Gaiji,
        Self::OpeningMonochromeSprites,
        Self::EndingMonochromeSprites,
        Self::BakedPlanarGraphics,
    ];

    fn capacity(self) -> Option<usize> {
        match self {
            Self::SharedPc98Gaiji => Some(SHARED_GAIJI_CAPACITY),
            Self::OpeningMonochromeSprites | Self::EndingMonochromeSprites => {
                Some(MONOCHROME_USABLE_GLYPH_COUNT)
            }
            Self::BakedPlanarGraphics => None,
        }
    }

    fn reserved_source_glyph_count(self) -> usize {
        match self {
            Self::SharedPc98Gaiji => GRAPHIC_GLYPH_COUNT + GAIJI_READINESS_GLYPH_COUNT,
            Self::OpeningMonochromeSprites
            | Self::EndingMonochromeSprites
            | Self::BakedPlanarGraphics => 0,
        }
    }
}

#[derive(Default)]
pub(super) struct DemandAccumulator {
    entry_count: usize,
    text_span_count: usize,
    characters: BTreeSet<char>,
    requires_blank_glyph: bool,
    forced_shared_gaiji: BTreeSet<char>,
}

impl DemandAccumulator {
    pub(super) fn add_translation(&mut self, lines: &[String]) {
        self.entry_count += 1;
        for line in lines {
            self.add_span(line);
        }
    }

    pub(super) fn entry_count(&self) -> usize {
        self.entry_count
    }

    pub(super) fn text_span_count(&self) -> usize {
        self.text_span_count
    }

    pub(super) fn add_span(&mut self, text: &str) {
        self.text_span_count += 1;
        self.requires_blank_glyph |= text.chars().any(char::is_whitespace);
        self.characters.extend(
            text.chars()
                .filter(|character| !character.is_control() && !character.is_whitespace()),
        );
    }

    pub(super) fn force_shared_gaiji(&mut self, characters: &[char]) {
        self.forced_shared_gaiji.extend(
            characters
                .iter()
                .copied()
                .filter(|character| self.characters.contains(character)),
        );
    }

    fn merge(&mut self, other: &Self) {
        self.entry_count += other.entry_count;
        self.text_span_count += other.text_span_count;
        self.characters.extend(other.characters.iter().copied());
        self.requires_blank_glyph |= other.requires_blank_glyph;
        self.forced_shared_gaiji
            .extend(other.forced_shared_gaiji.iter().copied());
    }

    pub(super) fn report(&self, render_path: TranslationRenderPath) -> TranslationGlyphDemand {
        self.report_with_capacity(
            render_path,
            render_path.capacity(),
            render_path.reserved_source_glyph_count(),
        )
    }

    pub(super) fn report_with_capacity(
        &self,
        render_path: TranslationRenderPath,
        physical_capacity: Option<usize>,
        reserved_source_glyph_count: usize,
    ) -> TranslationGlyphDemand {
        let slot_characters = match render_path {
            TranslationRenderPath::SharedPc98Gaiji => self
                .characters
                .iter()
                .copied()
                .filter(|character| {
                    !is_shift_jis_encodable(*character)
                        || self.forced_shared_gaiji.contains(character)
                })
                .collect::<BTreeSet<_>>(),
            TranslationRenderPath::OpeningMonochromeSprites
            | TranslationRenderPath::EndingMonochromeSprites => {
                let mut characters = self.characters.clone();
                if self.requires_blank_glyph {
                    characters.insert(' ');
                }
                characters
            }
            TranslationRenderPath::BakedPlanarGraphics => BTreeSet::new(),
        };
        let slot_glyph_count = physical_capacity.map(|_| slot_characters.len());
        let available_translation_capacity =
            physical_capacity.map(|capacity| capacity - reserved_source_glyph_count);
        let physical_capacity_headroom = physical_capacity
            .zip(slot_glyph_count)
            .map(|(capacity, demand)| capacity as isize - demand as isize);
        let available_capacity_headroom = available_translation_capacity
            .zip(slot_glyph_count)
            .map(|(capacity, demand)| capacity as isize - demand as isize);
        TranslationGlyphDemand {
            unique_rendered_character_count: self.characters.len(),
            unique_hangul_syllable_count: self
                .characters
                .iter()
                .filter(|character| is_hangul_syllable(**character))
                .count(),
            slot_glyph_count,
            verified_physical_capacity: physical_capacity,
            reserved_source_glyph_count,
            available_translation_capacity,
            physical_capacity_headroom,
            physical_capacity_fit: physical_capacity_headroom.map(|headroom| headroom >= 0),
            available_capacity_headroom,
            available_capacity_fit: available_capacity_headroom.map(|headroom| headroom >= 0),
            rendered_characters: strings_for(&self.characters),
            slot_glyph_characters: strings_for(&slot_characters),
        }
    }
}

fn is_shift_jis_encodable(character: char) -> bool {
    let mut encoded = [0_u8; 4];
    let value = character.encode_utf8(&mut encoded);
    let (_, _, had_errors) = SHIFT_JIS.encode(value);
    !had_errors
}

fn is_hangul_syllable(character: char) -> bool {
    matches!(character as u32, 0xAC00..=0xD7A3)
}

fn strings_for(characters: &BTreeSet<char>) -> Vec<String> {
    characters
        .iter()
        .map(|character| character.to_string())
        .collect()
}

#[cfg(test)]
#[path = "glyph_demand_tests.rs"]
mod glyph_demand_tests;
