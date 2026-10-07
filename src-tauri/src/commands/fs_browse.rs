//! Directory browsing for the in-app file picker.
//!
//! The app used to hand every file/folder choice to the OS dialog
//! (`@tauri-apps/plugin-dialog`). That works, but the native window is a
//! jarring interruption next to a UI this customised, and it cannot be
//! themed, filtered or remembered. So the frontend grew its own browser —
//! and a browser needs to read directories, which nothing here could do.
//!
//! Two commands, deliberately small: [`list_dir`] and [`fs_locations`].
//! Anything more (rename, delete, copy) belongs somewhere else; this module
//! only answers "what is in this folder" and "where might I start".
//!
//! Design notes worth keeping:
//!
//! * **A bad entry must not kill the listing.** `read_dir` yields `Result`
//!   per entry, and a single unreadable file (locked, permission denied) is
//!   common on Windows. Entries that fail are skipped, not propagated.
//! * **The listing is capped.** A folder with 100k files would otherwise
//!   serialise into a multi-megabyte IPC payload and freeze the webview.
//!   When the cap is hit the frontend is told via `truncated` so it can say
//!   so instead of silently lying.
//! * **Hidden is reported, not filtered.** The frontend owns the toggle;
//!   the backend just says what is there.

use serde::Serialize;
use std::path::{Path, PathBuf};

/// Upper bound on entries returned in one listing. Chosen to stay well
/// inside a comfortable IPC payload while covering essentially every
/// real resource-pack folder.
const MAX_ENTRIES: usize = 5000;

#[derive(Serialize, Debug)]
pub struct DirEntry {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    /// Bytes. `0` for directories.
    pub size: u64,
    /// Milliseconds since the Unix epoch. `None` when the platform cannot
    /// report it (or the value predates 1970, which is not worth an error).
    pub modified: Option<u64>,
    pub hidden: bool,
}

#[derive(Serialize, Debug)]
pub struct DirListing {
    /// The directory actually read. May differ from the requested path when
    /// the request was relative or contained `..`.
    pub path: String,
    /// `None` at a drive root.
    pub parent: Option<String>,
    pub entries: Vec<DirEntry>,
    /// True when [`MAX_ENTRIES`] cut the listing short.
    pub truncated: bool,
}

#[derive(Serialize)]
pub struct FsLocation {
    /// Stable id, used by the frontend for icons and ordering.
    pub id: String,
    /// Drives are labelled by their own path; the rest carry an i18n key so
    /// the frontend can translate them.
    pub label: String,
    pub label_key: Option<String>,
    pub path: String,
    /// `"drive" | "home" | "desktop" | "documents" | "downloads"`
    pub kind: String,
}

/// A listing failure the UI can explain.
///
/// A bare `String` forced the frontend to pattern-match on English prose to
/// decide between "access denied" and "this folder is gone". A stable `code`
/// lets it pick the right sentence and keep `detail` for the log and the
/// tooltip, where the raw OS message is actually useful.
#[derive(Serialize, Debug)]
pub struct FsError {
    /// `"denied" | "missing" | "not-dir" | "io"`
    pub code: String,
    /// The raw OS message. Shown on hover, never as the headline.
    pub detail: String,
}

impl FsError {
    fn new(code: &str, detail: impl Into<String>) -> Self {
        Self {
            code: code.to_string(),
            detail: detail.into(),
        }
    }
}

/// Map an `io::Error` to a code the UI has a sentence for.
///
/// `ErrorKind` is used rather than `raw_os_error` so the mapping also works
/// on the Unix builds, even though the Windows codes are the ones that
/// actually show up here.
fn classify(err: &std::io::Error, path: &Path) -> FsError {
    use std::io::ErrorKind;
    let code = match err.kind() {
        ErrorKind::PermissionDenied => "denied",
        ErrorKind::NotFound => "missing",
        _ => "io",
    };
    FsError::new(code, format!("{}: {}", path.display(), err))
}

/// Strip the Windows extended-length prefix for display.
///
/// `fs::canonicalize` returns `\\?\C:\Users\...` (or `\\?\UNC\server\share`),
/// which is an implementation detail of bypassing MAX_PATH. Users typed
/// `D:\`, so that is what they should see. Every filesystem call still goes
/// through `canonicalize`, which re-adds the prefix internally — stripping it
/// here loses no capability, it only stops leaking the trick into the UI.
fn for_display(path: &Path) -> String {
    let raw = path.to_string_lossy();
    if let Some(rest) = raw.strip_prefix(r"\\?\UNC\") {
        return format!(r"\\{}", rest);
    }
    if let Some(rest) = raw.strip_prefix(r"\\?\") {
        return rest.to_string();
    }
    raw.into_owned()
}

/// Reject names that would escape the directory being browsed.
///
/// Only relevant to the save-as flow, where a typed name is appended to the
/// current folder. Without this a name of `..\..\Windows\System32\x` would
/// resolve somewhere the user never navigated to.
pub(crate) fn safe_leaf(name: &str) -> Result<&str, FsError> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err(FsError::new("bad-name", "empty name"));
    }
    // Separators and the parent reference are the whole attack: either one
    // lets the caller climb out of the current directory.
    if trimmed.contains(['/', '\\']) || trimmed == ".." || trimmed == "." {
        return Err(FsError::new(
            "bad-name",
            format!("name must not contain a path separator: {trimmed}"),
        ));
    }
    // Windows also forbids these outright; catching them here gives a much
    // better message than the OS error would.
    if trimmed.contains([':', '*', '?', '"', '<', '>', '|']) {
        return Err(FsError::new(
            "bad-name",
            format!("name contains a reserved character: {trimmed}"),
        ));
    }
    Ok(trimmed)
}

/// Case-insensitive natural ordering: compares digit runs as numbers so
/// `1.20` sorts after `1.9` rather than before it. Resource-pack versions
/// are full of this shape, and plain lexicographic order reads as broken.
///
/// The result is a **total** order: two distinct names never compare
/// [`Ordering::Equal`]. That matters because `sort_by` is stable — if a
/// distinct pair compared Equal, the two would keep whatever order the
/// directory read produced, which varies between refreshes and makes the
/// list visibly jump.
fn natural_cmp(a: &str, b: &str) -> std::cmp::Ordering {
    let primary = natural_cmp_ci(a, b);
    if primary != std::cmp::Ordering::Equal {
        return primary;
    }
    // Equal under case folding. On Windows two such names cannot coexist in
    // one folder, but on a case-sensitive volume they can — so fall back to
    // the raw bytes rather than leaving the pair unordered.
    a.cmp(b)
}

/// The case-insensitive primary comparison behind [`natural_cmp`].
fn natural_cmp_ci(a: &str, b: &str) -> std::cmp::Ordering {
    use std::cmp::Ordering;

    let (ab, bb) = (a.as_bytes(), b.as_bytes());
    let (mut i, mut j) = (0usize, 0usize);

    while i < ab.len() && j < bb.len() {
        let (ca, cb) = (ab[i], bb[j]);

        if ca.is_ascii_digit() && cb.is_ascii_digit() {
            // Walk both digit runs, then compare by value first and by the
            // raw run second.
            //
            // The raw-run tie-break is not cosmetic: without it "01" and "1"
            // compare Equal, and `sort_by` is stable — so two distinct
            // filenames would keep whatever order the directory happened to
            // hand back, which changes between refreshes and makes the list
            // jump. The comparator has to be a *total* order.
            let si = i;
            while i < ab.len() && ab[i].is_ascii_digit() {
                i += 1;
            }
            let sj = j;
            while j < bb.len() && bb[j].is_ascii_digit() {
                j += 1;
            }
            let na = &a[si..i];
            let nb = &b[sj..j];
            let ta = na.trim_start_matches('0');
            let tb = nb.trim_start_matches('0');
            let ord = ta
                .len()
                .cmp(&tb.len())
                .then_with(|| ta.cmp(tb))
                .then_with(|| na.cmp(nb));
            if ord != Ordering::Equal {
                return ord;
            }
            continue;
        }

        let la = ca.to_ascii_lowercase();
        let lb = cb.to_ascii_lowercase();
        if la != lb {
            return la.cmp(&lb);
        }
        i += 1;
        j += 1;
    }

    // One is a prefix of the other; the shorter sorts first.
    (ab.len() - i).cmp(&(bb.len() - j))
}

#[cfg(windows)]
fn is_hidden(name: &str, meta: &std::fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;
    const FILE_ATTRIBUTE_HIDDEN: u32 = 0x2;
    name.starts_with('.') || (meta.file_attributes() & FILE_ATTRIBUTE_HIDDEN) != 0
}

#[cfg(not(windows))]
fn is_hidden(name: &str, _meta: &std::fs::Metadata) -> bool {
    name.starts_with('.')
}

fn modified_ms(meta: &std::fs::Metadata) -> Option<u64> {
    meta.modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_millis() as u64)
}

/// Read one directory.
///
/// Returns an error only when the directory itself cannot be read. Problems
/// with individual entries are skipped — see the module docs.
///
/// Note the two distinct paths in play: `dir` is the canonicalised form used
/// for every filesystem call (it carries the `\\?\` prefix on Windows), while
/// everything handed back to the frontend goes through [`for_display`] so the
/// UI shows `D:\` rather than `\\?\D:\`.
#[tauri::command]
pub fn list_dir(path: String) -> Result<DirListing, FsError> {
    use std::fs;

    let requested = PathBuf::from(&path);
    let dir = fs::canonicalize(&requested)
        .map_err(|e| classify(&e, &requested))?;

    if !dir.is_dir() {
        return Err(FsError::new(
            "not-dir",
            format!("{} is not a directory", for_display(&dir)),
        ));
    }

    let read = fs::read_dir(&dir).map_err(|e| classify(&e, &dir))?;

    let mut entries: Vec<DirEntry> = Vec::new();
    let mut truncated = false;

    for item in read {
        // A failing entry is normal on Windows (locked file, access denied
        // on a system folder). Skipping keeps the rest of the listing usable.
        let Ok(item) = item else { continue };
        let name = item.file_name().to_string_lossy().into_owned();
        let Ok(meta) = item.metadata() else { continue };

        let is_dir = meta.is_dir();
        entries.push(DirEntry {
            path: for_display(&item.path()),
            hidden: is_hidden(&name, &meta),
            size: if is_dir { 0 } else { meta.len() },
            modified: modified_ms(&meta),
            is_dir,
            name,
        });

        if entries.len() >= MAX_ENTRIES {
            truncated = true;
            break;
        }
    }

    // Folders first, then natural order within each group.
    entries.sort_by(|a, b| {
        b.is_dir
            .cmp(&a.is_dir)
            .then_with(|| natural_cmp(&a.name, &b.name))
    });

    Ok(DirListing {
        path: for_display(&dir),
        parent: dir.parent().map(for_display),
        entries,
        truncated,
    })
}

/// Places the picker can offer as shortcuts, in the order they should appear.
///
/// Only entries that actually exist are returned — a machine without a
/// Downloads folder should not show a dead shortcut.
#[tauri::command]
pub fn fs_locations() -> Vec<FsLocation> {
    let mut out: Vec<FsLocation> = Vec::new();

    #[cfg(windows)]
    {
        // Probe A..Z. `exists()` on a drive root is cheap and does not spin
        // up removable media the way opening a handle would.
        for letter in b'A'..=b'Z' {
            let root = format!("{}:\\", letter as char);
            if Path::new(&root).exists() {
                out.push(FsLocation {
                    id: format!("drive-{}", letter as char),
                    label: root.clone(),
                    label_key: None,
                    path: root,
                    kind: "drive".into(),
                });
            }
        }
    }

    #[cfg(not(windows))]
    {
        out.push(FsLocation {
            id: "drive-root".into(),
            label: "/".into(),
            label_key: None,
            path: "/".into(),
            kind: "drive".into(),
        });
    }

    let home = std::env::var("USERPROFILE")
        .ok()
        .or_else(|| std::env::var("HOME").ok())
        .filter(|p| !p.is_empty());

    if let Some(home) = home {
        out.push(FsLocation {
            id: "home".into(),
            label: String::new(),
            label_key: Some("fs.home".into()),
            path: home.clone(),
            kind: "home".into(),
        });

        for (sub, key, kind) in [
            ("Desktop", "fs.desktop", "desktop"),
            ("Documents", "fs.documents", "documents"),
            ("Downloads", "fs.downloads", "downloads"),
        ] {
            let p = Path::new(&home).join(sub);
            if p.is_dir() {
                out.push(FsLocation {
                    id: kind.into(),
                    label: String::new(),
                    label_key: Some(key.into()),
                    path: p.to_string_lossy().into_owned(),
                    kind: kind.into(),
                });
            }
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cmp::Ordering;

    #[test]
    fn natural_order_puts_1_20_after_1_9() {
        assert_eq!(natural_cmp("1.9", "1.20"), Ordering::Less);
        assert_eq!(natural_cmp("1.20", "1.9"), Ordering::Greater);
    }

    #[test]
    fn natural_order_is_case_insensitive_for_ordering() {
        // Case folding decides the *order*: "apple" sorts before "Banana"
        // even though 'B' < 'a' in raw bytes.
        assert_eq!(natural_cmp("apple", "Banana"), Ordering::Less);
        assert_eq!(natural_cmp("Banana", "apple"), Ordering::Greater);
        // Names differing only by case are still ordered, not left equal.
        assert_eq!(natural_cmp("Apple", "apple"), Ordering::Less);
    }

    #[test]
    fn natural_order_handles_leading_zeros_and_prefixes() {
        // "01" and "1" are numerically equal, so the raw run decides. What
        // matters is that the result is *not* Equal — otherwise two distinct
        // filenames would sort arbitrarily and the list would jump between
        // refreshes.
        assert_ne!(natural_cmp("01", "1"), Ordering::Equal);
        assert_eq!(natural_cmp("01", "1"), Ordering::Less);
        assert_eq!(natural_cmp("pack", "pack_old"), Ordering::Less);
    }

    #[test]
    fn natural_order_is_total_and_antisymmetric() {
        // Every pair of distinct names must get a definite order, and
        // reversing the arguments must reverse the result. A comparator that
        // violates this makes `sort_by` produce a different list each time.
        let names = ["1", "01", "1.20", "1.9", "Apple", "apple", "pack", "pack_old", "2x"];
        for a in names {
            for b in names {
                let ab = natural_cmp(a, b);
                let ba = natural_cmp(b, a);
                assert_eq!(ab, ba.reverse(), "not antisymmetric: {a} vs {b}");
                if a != b {
                    assert_ne!(ab, Ordering::Equal, "distinct names compared equal: {a} vs {b}");
                }
            }
        }
    }

    /// `list_dir` against a real directory.
    ///
    /// The comparator tests above never touch the filesystem, and the command
    /// is the part that actually ships — without this, nothing proved it reads
    /// a disk, sorts, or reports a parent correctly.
    #[test]
    fn list_dir_reads_a_real_directory() {
        let dir = std::env::temp_dir().join("2pyr_list_dir_test");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("beta")).unwrap();
        std::fs::create_dir_all(dir.join("Alpha")).unwrap();
        std::fs::write(dir.join("1.20.zip"), b"xx").unwrap();
        std::fs::write(dir.join("1.9.zip"), b"x").unwrap();

        let listing = list_dir(dir.to_string_lossy().into_owned()).unwrap();

        let names: Vec<&str> = listing.entries.iter().map(|e| e.name.as_str()).collect();
        // Folders first, then natural order — not lexicographic, which would
        // put "1.20" ahead of "1.9".
        assert_eq!(names, vec!["Alpha", "beta", "1.9.zip", "1.20.zip"]);
        assert!(listing.parent.is_some(), "a temp dir always has a parent");
        assert!(!listing.truncated);

        let sized = listing.entries.iter().find(|e| e.name == "1.9.zip").unwrap();
        assert_eq!(sized.size, 1);
        assert!(!sized.is_dir);
        assert!(sized.modified.is_some());
        // Directories report zero bytes rather than the directory's own size,
        // which is meaningless and platform-dependent.
        assert_eq!(listing.entries.iter().find(|e| e.name == "beta").unwrap().size, 0);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn list_dir_rejects_a_missing_path() {
        let missing = std::env::temp_dir().join("2pyr_definitely_not_here_9f3a");
        let _ = std::fs::remove_dir_all(&missing);
        let err = list_dir(missing.to_string_lossy().into_owned()).unwrap_err();
        assert_eq!(err.code, "missing");
    }

    /// The UI must never see the `\\?\` long-path prefix.
    ///
    /// `canonicalize` adds it on Windows to bypass MAX_PATH; leaking it makes
    /// the breadcrumb read `\\?\D:\` and confuses anyone who then types that
    /// path somewhere else. Driven through `list_dir` rather than by calling
    /// `for_display` directly, because the prefix has to be absent from the
    /// *output*, whatever produces it.
    #[test]
    fn list_dir_never_returns_the_long_path_prefix() {
        let dir = std::env::temp_dir().join("2pyr_prefix_test");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        let listing = list_dir(dir.to_string_lossy().into_owned()).unwrap();
        assert!(!listing.path.starts_with(r"\\?\"), "path leaked: {}", listing.path);
        if let Some(parent) = &listing.parent {
            assert!(!parent.starts_with(r"\\?\"), "parent leaked: {parent}");
        }
        for e in &listing.entries {
            assert!(!e.path.starts_with(r"\\?\"), "entry leaked: {}", e.path);
        }
        assert_eq!(for_display(&dir), listing.path);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn display_strips_both_prefix_forms() {
        assert_eq!(for_display(Path::new(r"\\?\D:\Games")), r"D:\Games");
        // UNC shares use a different spelling; the result must stay a UNC path.
        assert_eq!(
            for_display(Path::new(r"\\?\UNC\server\share\dir")),
            r"\\server\share\dir"
        );
        // Anything without the prefix is passed through untouched.
        assert_eq!(for_display(Path::new(r"D:\plain")), r"D:\plain");
    }

    /// The save-as flow appends a typed name to the current folder, so a name
    /// containing a separator would silently resolve outside it.
    #[test]
    fn safe_leaf_refuses_to_escape_the_directory() {
        assert!(safe_leaf("pack.zip").is_ok());
        assert!(safe_leaf("  spaced.zip  ").is_ok());

        for bad in [
            r"..\..\Windows\System32\x.dll",
            "../../etc/passwd",
            "..",
            ".",
            "",
            "   ",
            r"sub\dir",
            "sub/dir",
            "C:evil",
            "bad:name",
            "star*",
            "question?",
        ] {
            assert!(safe_leaf(bad).is_err(), "should have been refused: {bad:?}");
        }
    }
}
