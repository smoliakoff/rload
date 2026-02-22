pub mod test_server;

use std::path::PathBuf;

pub fn fixture_path(name: &str) -> PathBuf {
    let root = std::env::var("WORKSPACE_ROOT").expect("WORKSPACE_ROOT is not set");
    PathBuf::from(root).join(name)
}