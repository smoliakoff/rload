pub mod test_server;

use std::path::PathBuf;

pub fn fixture_path(name: &str) -> PathBuf {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    root.join("../../").join(name)
}