use std::{env, path::PathBuf};

#[derive(Clone)]
pub struct AppPaths {
    pub resources_dir: PathBuf,
    pub index_path: PathBuf,
}

impl AppPaths {
    pub fn from_exe() -> Self {
        let exe_dir = env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|d| d.to_path_buf()));

        let base_dir = exe_dir.unwrap_or_else(|| env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
        let resources_dir = base_dir.join("resources");
        let index_path = resources_dir.join("index.html");

        Self { resources_dir, index_path }
    }
}
