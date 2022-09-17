//! Database model.
mod namespace;
mod package;
mod publisher;

pub use namespace::NamespaceModel;
pub use package::PackageModel;
pub use publisher::PublisherModel;

use serde::Deserialize;
use std::{fmt, str::FromStr};

use crate::Error;

/// Default limit for pagination.
pub fn default_limit() -> i64 {
    25
}

/// Determines how versions should be fetched when listing packages.
#[derive(Default, Debug, Deserialize, Copy, Clone)]
#[serde(rename_all = "lowercase")]
pub enum VersionIncludes {
    #[default]
    None,
    Latest,
}

impl fmt::Display for VersionIncludes {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::None => "none",
                Self::Latest => "latest",
            }
        )
    }
}

impl FromStr for VersionIncludes {
    type Err = Error;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        if s.to_lowercase() == "none" {
            Ok(Self::None)
        } else if s.to_lowercase() == "latest" {
            Ok(Self::Latest)
        } else {
            Err(Error::InvalidVersionIncludes(s.to_owned()))
        }
    }
}

/// Defines parameters for paginating list queries.
#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct Pager {
    pub offset: i64,
    pub limit: i64,
    pub sort: SortOrder,
}

impl Default for Pager {
    fn default() -> Self {
        Self {
            offset: 0,
            limit: default_limit(),
            sort: Default::default(),
        }
    }
}

/// Represents an order by direction.
#[derive(Debug, Default, Deserialize, Copy, Clone)]
#[serde(rename_all = "lowercase")]
pub enum SortOrder {
    #[default]
    Asc,
    Desc,
}

impl SortOrder {
    /// Get a string for each variant.
    pub fn as_str(&self) -> &str {
        match self {
            Self::Asc => "ASC",
            Self::Desc => "DESC",
        }
    }
}

impl fmt::Display for SortOrder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str().to_lowercase())
    }
}

impl FromStr for SortOrder {
    type Err = Error;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        if s.to_lowercase() == "asc" {
            Ok(Self::Asc)
        } else if s.to_lowercase() == "desc" {
            Ok(Self::Desc)
        } else {
            Err(Error::InvalidSortOrder(s.to_owned()))
        }
    }
}
