use crate::HistoryStore;
use std::{
    fs, io,
    path::{Path, PathBuf},
    time::SystemTime,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileFingerprint {
    pub len: u64,
    pub modified: Option<SystemTime>,
}
impl FileFingerprint {
    pub fn read(path: &Path) -> io::Result<Self> {
        let metadata = fs::metadata(path)?;
        Ok(Self {
            len: metadata.len(),
            modified: metadata.modified().ok(),
        })
    }
}

#[derive(Debug)]
pub struct LiveHistory {
    path: PathBuf,
    fingerprint: FileFingerprint,
}
impl LiveHistory {
    pub fn new(path: impl Into<PathBuf>) -> io::Result<Self> {
        let path = path.into();
        let fingerprint = FileFingerprint::read(&path)?;
        Ok(Self { path, fingerprint })
    }
    pub fn load_if_changed(&mut self) -> io::Result<Option<HistoryStore>> {
        let next = FileFingerprint::read(&self.path)?;
        if next == self.fingerprint {
            return Ok(None);
        }
        let store = HistoryStore::load_sqlite(&self.path).map_err(io::Error::other)?;
        self.fingerprint = next;
        Ok(Some(store))
    }
    pub fn path(&self) -> &Path {
        &self.path
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fingerprints_are_stable_for_unchanged_file() {
        let path = std::env::temp_dir().join(format!("cmdscope-live-{}", std::process::id()));
        fs::write(&path, b"history").unwrap();
        assert_eq!(
            FileFingerprint::read(&path).unwrap(),
            FileFingerprint::read(&path).unwrap()
        );
        fs::remove_file(path).unwrap();
    }
}
