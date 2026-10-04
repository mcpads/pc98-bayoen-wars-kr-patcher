use anyhow::{Result, bail};
use serde::Serialize;

const CHARACTER_PREFIX: &str = concat!(
    "。あいうえおかきくけこがぎぐげご",
    "さしすせそざじずぜぞたちつてとだ",
    "ぢづでどなにぬねのはひふへほばび",
    "ぶべぼぱぴぷぺぽまみむめもやゆよ",
    "らりるれろわをんっゃゅょぁぃぅぇ",
    "ぉー～！？、「…・アイウオカキク",
    "ケコガグゴサシスセジゾタチツトダ",
    "ドナニネハフバビブパピプマメモヤ",
    "ラリルレンァィェッャュ",
);
const FIRST_GRAPHIC_INDEX: usize = 139;
const CHARACTER_SUFFIX_START: usize = 141;
const CHARACTER_SUFFIX: &str = concat!(
    "全開絶",
    "好調然来大丈夫目泉体回復危打撃怒",
    "激計女王乱舞元気死魔導力Ｉ",
);
const EFFECT_START: usize = 173;
const GLYPH_COUNT: usize = 184;
pub(crate) const GRAPHIC_GLYPH_COUNT: usize = 2 + (GLYPH_COUNT - EFFECT_START);

#[derive(Debug, Clone, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum GaijiGlyphMeaning {
    Character { character: char },
    Graphic { role: String },
}

impl GaijiGlyphMeaning {
    pub(super) fn source_text(&self) -> String {
        match self {
            Self::Character { character } => character.to_string(),
            Self::Graphic { role } => format!("<GAIJI-GRAPHIC:{role}>"),
        }
    }
}

pub(super) fn meaning_for_index(index: usize) -> Result<GaijiGlyphMeaning> {
    if let Some(character) = CHARACTER_PREFIX.chars().nth(index) {
        return Ok(GaijiGlyphMeaning::Character { character });
    }
    if index == FIRST_GRAPHIC_INDEX {
        return Ok(graphic("halftone-fill"));
    }
    if index == FIRST_GRAPHIC_INDEX + 1 {
        return Ok(graphic("solid-fill"));
    }
    if let Some(character) = index
        .checked_sub(CHARACTER_SUFFIX_START)
        .and_then(|suffix_index| CHARACTER_SUFFIX.chars().nth(suffix_index))
    {
        return Ok(GaijiGlyphMeaning::Character { character });
    }
    if (EFFECT_START..GLYPH_COUNT).contains(&index) {
        return Ok(graphic(&format!(
            "particle-frame-{:02}",
            index - EFFECT_START + 1
        )));
    }
    bail!("GAIJI glyph index {index} lies outside the verified meaning table")
}

fn graphic(role: &str) -> GaijiGlyphMeaning {
    GaijiGlyphMeaning::Graphic {
        role: role.to_owned(),
    }
}

#[cfg(test)]
#[path = "gaiji_meaning_tests.rs"]
mod gaiji_meaning_tests;
