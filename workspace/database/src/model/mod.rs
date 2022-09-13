//! Database model.
mod namespace;
mod package;
mod publisher;

pub use namespace::NamespaceModel;
pub use package::PackageModel;
pub use publisher::PublisherModel;

use serde::Deserialize;
use std::fmt;

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
        write!(f, "{}", self.as_str())
    }
}
