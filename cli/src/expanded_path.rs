use std::{
    env::VarError,
    fmt::Display,
    ops::{Deref, DerefMut},
    path::{Path, PathBuf},
    str::FromStr,
};

use shellexpand::LookupError;

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct ExpandedPath(PathBuf);

impl Deref for ExpandedPath {
    type Target = PathBuf;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for ExpandedPath {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl FromStr for ExpandedPath {
    type Err = LookupError<VarError>;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        shellexpand::full(s).map(|expanded| ExpandedPath(PathBuf::from(expanded.as_ref())))
    }
}

impl AsRef<Path> for ExpandedPath {
    fn as_ref(&self) -> &Path {
        &self.0
    }
}

impl Display for ExpandedPath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.display().fmt(f)
    }
}
