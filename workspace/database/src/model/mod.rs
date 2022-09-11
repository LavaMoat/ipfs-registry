//! Database model.
mod namespace;
mod package;
mod publisher;

pub use namespace::NamespaceModel;
pub use package::PackageModel;
pub use publisher::PublisherModel;

use std::fmt;

/// Defines parameters for paginating list queries.
#[derive(Debug)]
pub struct Pager {
    pub offset: i64,
    pub limit: i64,
    pub direction: Direction,
}

impl Default for Pager {
    fn default() -> Self {
        Self {
            offset: 0,
            limit: 25,
            direction: Default::default(),
        }
    }
}

/// Represents an order by direction.
#[derive(Debug, Default)]
pub enum Direction {
    #[default]
    ASC,
    DESC,
}

impl Direction {
    /// Get a string for each variant.
    pub fn as_str(&self) -> &str {
        match self {
            Self::ASC => "ASC",
            Self::DESC => "DESC",
        }
    }
}

impl fmt::Display for Direction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}
