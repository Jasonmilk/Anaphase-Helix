//! Shared temp-dir helper for this module's test blocks.

use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

static SEQ: AtomicUsize = AtomicUsize::new(0);

/// Scratch directory for tests.
///
/// A fixed name is a race: `cargo test` runs tests in parallel, and a test that
/// removes its directory at the end will remove a *sibling's* files mid-run if
/// both picked the same name. The name is therefore derived from the process id
/// and a per-process sequence number, so every call owns its own directory and
/// no two tests can disturb each other.
pub fn tmp_dir(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!(
        "se-test-{}-{}-{}",
        std::process::id(),
        SEQ.fetch_add(1, Ordering::Relaxed),
        tag
    ));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}
