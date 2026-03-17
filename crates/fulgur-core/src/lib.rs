pub mod config;
pub mod error;
pub mod pageable;
pub mod paginate;

pub use config::{Config, ConfigBuilder, Margin, PageSize};
pub use error::{Error, Result};
