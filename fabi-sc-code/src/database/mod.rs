mod indexed_db;
mod taskbar_db;

pub use indexed_db::IndexedDb;
pub use taskbar_db::{
    AppMetadata, AvailableApp, PinnedApp, TaskbarDb, WindowConfig, fetch_app_metadata,
};
