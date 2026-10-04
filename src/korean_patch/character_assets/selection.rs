use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, ensure};

use crate::source_disk::sha256_hex;

pub const ARLE_CHARACTER_ASSET_SET_FILENAMES: [&str; 3] = [
    "battle-sprites.json",
    "battle-sprites.pc98.png",
    "battle-sprites-authoring.png",
];

const MANIFEST_FILENAME: &str = ARLE_CHARACTER_ASSET_SET_FILENAMES[0];
const ARTWORK_FILENAME: &str = ARLE_CHARACTER_ASSET_SET_FILENAMES[1];
const TRACKED_ASSET_HASHES: [(&str, &str); 3] = [
    (
        "battle-sprites.json",
        "fab9ed912ca10ddcea1fbc1d7b283202e46e2d41b4dee9390142b2ef37d3b3e8",
    ),
    (
        "battle-sprites.pc98.png",
        "abc6f393708efa54511b7a8cb1404979d576fce03c31f8e813475293be347cc7",
    ),
    (
        "battle-sprites-authoring.png",
        "41e743d387e9d8f6a9da94c529c61d1dddc40fa96ab25ce09981bf56e1cd26df",
    ),
];

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArleCharacterAssetSet {
    source: ArleCharacterAssetSetSource,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArleCharacterAssetSelection {
    replacement: Option<ArleCharacterAssetSet>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum ArleCharacterAssetSetSource {
    Tracked(PathBuf),
    Directory(PathBuf),
}

impl ArleCharacterAssetSet {
    pub fn tracked() -> Self {
        Self {
            source: ArleCharacterAssetSetSource::Tracked(
                Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/characters/arle"),
            ),
        }
    }

    pub fn from_directory(directory: impl Into<PathBuf>) -> Self {
        Self {
            source: ArleCharacterAssetSetSource::Directory(directory.into()),
        }
    }

    pub fn directory(&self) -> &Path {
        match &self.source {
            ArleCharacterAssetSetSource::Tracked(directory)
            | ArleCharacterAssetSetSource::Directory(directory) => directory,
        }
    }

    pub(crate) fn resolve(&self) -> Result<ResolvedArleCharacterAssetSet> {
        let directory = self.directory();
        ensure!(
            directory.is_dir(),
            "Arle character asset set path is not a directory: {}",
            directory.display()
        );
        for file_name in ARLE_CHARACTER_ASSET_SET_FILENAMES {
            let path = directory.join(file_name);
            ensure!(
                path.is_file(),
                "Arle character asset set is missing required file {}",
                path.display()
            );
        }
        if matches!(&self.source, ArleCharacterAssetSetSource::Tracked(_)) {
            verify_tracked_asset_hashes(directory)?;
        }
        Ok(ResolvedArleCharacterAssetSet {
            manifest: directory.join(MANIFEST_FILENAME),
            artwork: directory.join(ARTWORK_FILENAME),
        })
    }
}

impl ArleCharacterAssetSelection {
    pub fn preserve_original() -> Self {
        Self { replacement: None }
    }

    pub fn tracked() -> Self {
        Self::replace(ArleCharacterAssetSet::tracked())
    }

    pub fn from_directory(directory: impl Into<PathBuf>) -> Self {
        Self::replace(ArleCharacterAssetSet::from_directory(directory))
    }

    pub fn replace(asset_set: ArleCharacterAssetSet) -> Self {
        Self {
            replacement: Some(asset_set),
        }
    }

    pub fn replacement(&self) -> Option<&ArleCharacterAssetSet> {
        self.replacement.as_ref()
    }

    pub(crate) fn resolve(&self) -> Result<ResolvedArleCharacterAssetSelection> {
        self.replacement
            .as_ref()
            .map(ArleCharacterAssetSet::resolve)
            .transpose()
            .map(|replacement| match replacement {
                Some(asset_set) => {
                    ResolvedArleCharacterAssetSelection::Replace(Box::new(asset_set))
                }
                None => ResolvedArleCharacterAssetSelection::PreserveOriginal,
            })
    }
}

impl Default for ArleCharacterAssetSet {
    fn default() -> Self {
        Self::tracked()
    }
}

impl fmt::Display for ArleCharacterAssetSelection {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.replacement {
            Some(asset_set) => write!(formatter, "{}", asset_set.directory().display()),
            None => formatter.write_str("original game Arle assets (preserved)"),
        }
    }
}

fn verify_tracked_asset_hashes(directory: &Path) -> Result<()> {
    for (file_name, expected_sha256) in TRACKED_ASSET_HASHES {
        let path = directory.join(file_name);
        let bytes = fs::read(&path)
            .with_context(|| format!("failed to read tracked Arle asset {}", path.display()))?;
        let actual_sha256 = sha256_hex(&bytes);
        ensure!(
            actual_sha256 == expected_sha256,
            "tracked Arle asset {file_name} hash changed: expected {expected_sha256}, got {actual_sha256}"
        );
    }
    Ok(())
}

#[derive(Debug, Eq, PartialEq)]
pub(crate) enum ResolvedArleCharacterAssetSelection {
    PreserveOriginal,
    Replace(Box<ResolvedArleCharacterAssetSet>),
}

#[derive(Debug, Eq, PartialEq)]
pub(crate) struct ResolvedArleCharacterAssetSet {
    pub manifest: PathBuf,
    pub artwork: PathBuf,
}

#[cfg(test)]
#[path = "selection_tests.rs"]
mod selection_tests;
