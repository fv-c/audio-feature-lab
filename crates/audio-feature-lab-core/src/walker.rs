use std::collections::BTreeSet;
use std::error::Error;
use std::ffi::OsStr;
use std::fmt;
use std::fs::{self, ReadDir};
use std::io;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Walker {
    config: WalkerConfig,
}

impl Walker {
    pub fn new(config: WalkerConfig) -> Self {
        Self { config }
    }

    pub fn config(&self) -> &WalkerConfig {
        &self.config
    }

    pub fn walk(&self, root: &Path) -> Result<WalkIter, WalkError> {
        let metadata = fs::metadata(root).map_err(|source| WalkError::Io {
            operation: "read root metadata",
            path: root.to_path_buf(),
            source,
        })?;

        if !metadata.is_dir() {
            return Err(WalkError::RootNotDirectory {
                path: root.to_path_buf(),
            });
        }

        Ok(WalkIter {
            root: root.to_path_buf(),
            config: self.config.clone(),
            pending_dirs: vec![root.to_path_buf()],
            current_dir: None,
            current_dir_path: None,
        })
    }

    pub fn scan(&self, root: &Path) -> Result<Vec<WalkedFile>, WalkError> {
        self.walk(root)?.collect()
    }
}

impl Default for Walker {
    fn default() -> Self {
        Self::new(WalkerConfig::default())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WalkerConfig {
    pub extensions: BTreeSet<String>,
    pub hidden: HiddenPolicy,
    pub symlinks: SymlinkPolicy,
}

impl WalkerConfig {
    pub fn with_extensions<I, S>(mut self, extensions: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        self.extensions = extensions
            .into_iter()
            .map(|extension| normalize_extension(extension.as_ref()))
            .collect();
        self
    }

    fn matches_extension(&self, path: &Path) -> bool {
        if self.extensions.is_empty() {
            return true;
        }

        path.extension()
            .and_then(OsStr::to_str)
            .map(normalize_extension)
            .map(|extension| self.extensions.contains(&extension))
            .unwrap_or(false)
    }
}

impl Default for WalkerConfig {
    fn default() -> Self {
        Self {
            extensions: BTreeSet::new(),
            hidden: HiddenPolicy::Exclude,
            symlinks: SymlinkPolicy::SkipAll,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HiddenPolicy {
    Exclude,
    Include,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SymlinkPolicy {
    SkipAll,
    FollowFileTargets,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WalkedFile {
    pub path: PathBuf,
    pub relative_path: PathBuf,
    pub identity: FileIdentity,
}

#[derive(Debug)]
pub struct WalkIter {
    root: PathBuf,
    config: WalkerConfig,
    pending_dirs: Vec<PathBuf>,
    current_dir: Option<ReadDir>,
    current_dir_path: Option<PathBuf>,
}

impl Iterator for WalkIter {
    type Item = Result<WalkedFile, WalkError>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some(current_dir) = self.current_dir.as_mut() {
                match current_dir.next() {
                    Some(Ok(entry)) => match self.handle_entry(entry) {
                        Ok(Some(file)) => return Some(Ok(file)),
                        Ok(None) => continue,
                        Err(error) => return Some(Err(error)),
                    },
                    Some(Err(source)) => {
                        let path = self
                            .current_dir_path
                            .clone()
                            .unwrap_or_else(|| self.root.clone());
                        return Some(Err(WalkError::Io {
                            operation: "read directory entry",
                            path,
                            source,
                        }));
                    }
                    None => {
                        self.current_dir = None;
                        self.current_dir_path = None;
                        continue;
                    }
                }
            }

            let next_dir = self.pending_dirs.pop()?;
            match fs::read_dir(&next_dir) {
                Ok(read_dir) => {
                    self.current_dir = Some(read_dir);
                    self.current_dir_path = Some(next_dir);
                }
                Err(source) => {
                    return Some(Err(WalkError::Io {
                        operation: "read directory",
                        path: next_dir,
                        source,
                    }));
                }
            }
        }
    }
}

impl WalkIter {
    fn handle_entry(&mut self, entry: fs::DirEntry) -> Result<Option<WalkedFile>, WalkError> {
        if self.config.hidden == HiddenPolicy::Exclude && is_hidden_name(&entry.file_name()) {
            return Ok(None);
        }

        let path = entry.path();
        let file_type = entry.file_type().map_err(|source| WalkError::Io {
            operation: "read file type",
            path: path.clone(),
            source,
        })?;

        if file_type.is_symlink() {
            return self.handle_symlink(path);
        }

        if file_type.is_dir() {
            self.pending_dirs.push(path);
            return Ok(None);
        }

        if !file_type.is_file() || !self.config.matches_extension(&path) {
            return Ok(None);
        }

        let metadata = entry.metadata().map_err(|source| WalkError::Io {
            operation: "read file metadata",
            path: path.clone(),
            source,
        })?;

        self.walked_file(path, &metadata)
    }

    fn handle_symlink(&mut self, path: PathBuf) -> Result<Option<WalkedFile>, WalkError> {
        if self.config.symlinks == SymlinkPolicy::SkipAll {
            return Ok(None);
        }

        let metadata = fs::metadata(&path).map_err(|source| WalkError::Io {
            operation: "read symlink target metadata",
            path: path.clone(),
            source,
        })?;

        if !metadata.is_file() || !self.config.matches_extension(&path) {
            return Ok(None);
        }

        self.walked_file(path, &metadata)
    }

    fn walked_file(
        &self,
        path: PathBuf,
        metadata: &fs::Metadata,
    ) -> Result<Option<WalkedFile>, WalkError> {
        let relative_path = path
            .strip_prefix(&self.root)
            .map_err(|_| WalkError::StripPrefix {
                root: self.root.clone(),
                path: path.clone(),
            })?
            .to_path_buf();

        let identity =
            FileIdentity::from_metadata(metadata).map_err(|source| WalkError::Identity {
                path: path.clone(),
                source,
            })?;

        Ok(Some(WalkedFile {
            path,
            relative_path,
            identity,
        }))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileIdentity {
    pub baseline: MetadataIdentity,
    pub content_hash: Option<ContentHash>,
}

impl FileIdentity {
    pub fn from_path(path: &Path) -> Result<Self, WalkError> {
        let metadata = fs::metadata(path).map_err(|source| WalkError::Io {
            operation: "read file metadata",
            path: path.to_path_buf(),
            source,
        })?;

        Self::from_metadata(&metadata).map_err(|source| WalkError::Identity {
            path: path.to_path_buf(),
            source,
        })
    }

    pub fn from_metadata(metadata: &fs::Metadata) -> Result<Self, IdentityError> {
        let modified = metadata
            .modified()
            .map_err(IdentityError::ReadModifiedTime)?;
        let modified_unix_nanos = system_time_to_unix_nanos(modified)?;

        Ok(Self {
            baseline: MetadataIdentity {
                modified_unix_nanos,
                size_bytes: metadata.len(),
            },
            content_hash: None,
        })
    }

    pub fn same_baseline(&self, other: &Self) -> bool {
        self.baseline == other.baseline
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MetadataIdentity {
    pub modified_unix_nanos: i128,
    pub size_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContentHash {
    pub algorithm: String,
    pub value: String,
}

#[derive(Debug)]
pub enum WalkError {
    RootNotDirectory {
        path: PathBuf,
    },
    StripPrefix {
        root: PathBuf,
        path: PathBuf,
    },
    Io {
        operation: &'static str,
        path: PathBuf,
        source: io::Error,
    },
    Identity {
        path: PathBuf,
        source: IdentityError,
    },
}

impl fmt::Display for WalkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RootNotDirectory { path } => {
                write!(f, "scan root is not a directory: {}", path.display())
            }
            Self::StripPrefix { root, path } => write!(
                f,
                "failed to derive path relative to scan root `{}` for `{}`",
                root.display(),
                path.display()
            ),
            Self::Io {
                operation,
                path,
                source,
            } => write!(f, "{operation} for {}: {source}", path.display()),
            Self::Identity { path, source } => {
                write!(
                    f,
                    "failed to build file identity for {}: {source}",
                    path.display()
                )
            }
        }
    }
}

impl Error for WalkError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            Self::Identity { source, .. } => Some(source),
            Self::RootNotDirectory { .. } | Self::StripPrefix { .. } => None,
        }
    }
}

#[derive(Debug)]
pub enum IdentityError {
    ReadModifiedTime(io::Error),
    TimestampOutOfRange,
}

impl fmt::Display for IdentityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ReadModifiedTime(source) => write!(f, "could not read modified time: {source}"),
            Self::TimestampOutOfRange => write!(f, "modified time is out of supported range"),
        }
    }
}

impl Error for IdentityError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::ReadModifiedTime(source) => Some(source),
            Self::TimestampOutOfRange => None,
        }
    }
}

fn normalize_extension(extension: &str) -> String {
    extension.trim_start_matches('.').to_ascii_lowercase()
}

fn is_hidden_name(name: &OsStr) -> bool {
    name.to_string_lossy().starts_with('.')
}

fn system_time_to_unix_nanos(time: std::time::SystemTime) -> Result<i128, IdentityError> {
    match time.duration_since(UNIX_EPOCH) {
        Ok(duration) => {
            i128::try_from(duration.as_nanos()).map_err(|_| IdentityError::TimestampOutOfRange)
        }
        Err(error) => {
            let nanos = i128::try_from(error.duration().as_nanos())
                .map_err(|_| IdentityError::TimestampOutOfRange)?;
            Ok(-nanos)
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::process;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::{FileIdentity, HiddenPolicy, SymlinkPolicy, Walker, WalkerConfig};

    static NEXT_DIR_ID: AtomicU64 = AtomicU64::new(0);

    #[test]
    fn scans_recursively_with_case_insensitive_extension_filtering() {
        let temp_dir = TestDir::new();
        write_file(&temp_dir.path().join("root.wav"), b"wav");
        write_file(&temp_dir.path().join("nested/track.FLAC"), b"flac");
        write_file(&temp_dir.path().join("nested/notes.txt"), b"text");

        let walker = Walker::new(WalkerConfig::default().with_extensions(["wav", "flac"]));
        let mut files = walker.scan(temp_dir.path()).expect("scan should succeed");
        files.sort_by(|left, right| left.relative_path.cmp(&right.relative_path));

        let relative_paths = files
            .iter()
            .map(|entry| entry.relative_path.clone())
            .collect::<Vec<_>>();
        assert_eq!(
            relative_paths,
            vec![
                PathBuf::from("nested").join("track.FLAC"),
                PathBuf::from("root.wav"),
            ]
        );
    }

    #[test]
    fn excludes_hidden_files_and_directories_by_default() {
        let temp_dir = TestDir::new();
        write_file(&temp_dir.path().join("visible.wav"), b"visible");
        write_file(&temp_dir.path().join(".hidden.wav"), b"hidden");
        write_file(&temp_dir.path().join(".private/inside.flac"), b"inside");

        let walker = Walker::new(WalkerConfig::default().with_extensions(["wav", "flac"]));
        let files = walker.scan(temp_dir.path()).expect("scan should succeed");

        assert_eq!(files.len(), 1);
        assert_eq!(files[0].relative_path, PathBuf::from("visible.wav"));
    }

    #[test]
    fn can_include_hidden_files_when_requested() {
        let temp_dir = TestDir::new();
        write_file(&temp_dir.path().join("visible.wav"), b"visible");
        write_file(&temp_dir.path().join(".hidden.wav"), b"hidden");
        write_file(&temp_dir.path().join(".private/inside.flac"), b"inside");

        let walker = Walker::new(WalkerConfig {
            hidden: HiddenPolicy::Include,
            ..WalkerConfig::default().with_extensions(["wav", "flac"])
        });
        let mut files = walker.scan(temp_dir.path()).expect("scan should succeed");
        files.sort_by(|left, right| left.relative_path.cmp(&right.relative_path));

        let relative_paths = files
            .iter()
            .map(|entry| entry.relative_path.clone())
            .collect::<Vec<_>>();
        assert_eq!(
            relative_paths,
            vec![
                PathBuf::from(".hidden.wav"),
                PathBuf::from(".private").join("inside.flac"),
                PathBuf::from("visible.wav"),
            ]
        );
    }

    #[test]
    fn preserves_relative_paths_without_os_specific_assumptions() {
        let temp_dir = TestDir::new();
        let relative_path = PathBuf::from("disc 01").join("track one.wav");
        let full_path = temp_dir.path().join(&relative_path);
        write_file(&full_path, b"audio");

        let walker = Walker::new(WalkerConfig::default().with_extensions(["wav"]));
        let files = walker.scan(temp_dir.path()).expect("scan should succeed");

        assert_eq!(files.len(), 1);
        assert_eq!(files[0].relative_path, relative_path);
        assert_eq!(files[0].path, full_path);
    }

    #[test]
    fn file_identity_uses_mtime_and_size_baseline() {
        let temp_dir = TestDir::new();
        let path = temp_dir.path().join("sample.wav");
        write_file(&path, b"1234");

        let identity_before = FileIdentity::from_path(&path).expect("identity should build");
        let identity_repeat = FileIdentity::from_path(&path).expect("identity should build");
        assert!(identity_before.same_baseline(&identity_repeat));
        assert_eq!(identity_before.content_hash, None);

        write_file(&path, b"12345678");
        let identity_after = FileIdentity::from_path(&path).expect("identity should build");

        assert!(!identity_before.same_baseline(&identity_after));
        assert_eq!(identity_after.baseline.size_bytes, 8);
    }

    #[test]
    fn follows_file_symlinks_only_when_requested() {
        let temp_dir = TestDir::new();
        let target = temp_dir.path().join("target.wav");
        write_file(&target, b"target");

        let scan_root = temp_dir.path().join("scan-root");
        fs::create_dir_all(&scan_root).expect("root should exist");
        let link = scan_root.join("linked.wav");

        if let Err(error) = create_file_symlink(&target, &link) {
            if error.kind() == std::io::ErrorKind::PermissionDenied {
                return;
            }

            panic!("failed to create test symlink: {error}");
        }

        let skip_walker = Walker::new(WalkerConfig::default().with_extensions(["wav"]));
        let skipped = skip_walker.scan(&scan_root).expect("scan should succeed");
        assert!(skipped.is_empty());

        let follow_walker = Walker::new(WalkerConfig {
            symlinks: SymlinkPolicy::FollowFileTargets,
            ..WalkerConfig::default().with_extensions(["wav"])
        });
        let followed = follow_walker.scan(&scan_root).expect("scan should succeed");

        assert_eq!(followed.len(), 1);
        assert_eq!(followed[0].relative_path, PathBuf::from("linked.wav"));
        assert_eq!(followed[0].path, link);
    }

    struct TestDir {
        path: PathBuf,
    }

    impl TestDir {
        fn new() -> Self {
            let mut path = std::env::temp_dir();
            let unique = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("time should be after epoch")
                .as_nanos();
            let id = NEXT_DIR_ID.fetch_add(1, Ordering::Relaxed);
            path.push(format!(
                "audio-feature-lab-walker-{}-{}-{}",
                process::id(),
                unique,
                id
            ));
            fs::create_dir_all(&path).expect("temp directory should be created");
            Self { path }
        }

        fn path(&self) -> &Path {
            &self.path
        }
    }

    impl Drop for TestDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    fn write_file(path: &Path, contents: &[u8]) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("parent directories should exist");
        }

        fs::write(path, contents).expect("test file should be written");
    }

    #[cfg(unix)]
    fn create_file_symlink(target: &Path, link: &Path) -> std::io::Result<()> {
        std::os::unix::fs::symlink(target, link)
    }

    #[cfg(windows)]
    fn create_file_symlink(target: &Path, link: &Path) -> std::io::Result<()> {
        std::os::windows::fs::symlink_file(target, link)
    }
}
