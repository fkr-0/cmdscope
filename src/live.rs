use std::{fs, io, path::Path, time::SystemTime};

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

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fingerprints_are_stable_for_unchanged_file() {
        let path = std::env::temp_dir().join(format!("cmdscope-live-{}", std::process::id()));
        fs::write(&path, b"history").unwrap();
        let first = FileFingerprint::read(&path).unwrap();
        let second = FileFingerprint::read(&path).unwrap();
        assert_eq!(first, second);
        fs::remove_file(path).unwrap();
    }
}
