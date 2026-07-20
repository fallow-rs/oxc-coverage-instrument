//! Capability-scoped writer for the report tree.
//!
//! Three guarantees hold for every page the reporter emits:
//!
//! - the output root is opened once as a cap-std [`Dir`], and every path is
//!   resolved relative to that capability rather than through the ambient
//!   filesystem;
//! - each directory component is opened `nofollow`, so a symlinked
//!   component fails the write instead of redirecting it outside the root,
//!   including when the symlink is swapped in after the root is opened;
//! - each page is written to a `.oxc-coverage-*.tmp` file and renamed over
//!   its leaf, so a pre-existing or hard-linked leaf is replaced rather
//!   than truncated in place.

use std::collections::hash_map::RandomState;
use std::ffi::{OsStr, OsString};
use std::hash::BuildHasher;
use std::io::{self, Write};
use std::path::{Component, Path};
use std::sync::atomic::{AtomicU64, Ordering};

use cap_fs_ext::{DirExt, FollowSymlinks, OpenOptionsFollowExt, ambient_authority};
use cap_std::fs::{Dir, OpenOptions};

/// A colliding temporary name means another writer won the race for it, so
/// retry; the bound stops a pathological loop if the directory is hostile.
const TEMP_CREATE_ATTEMPTS: usize = 32;

pub(super) struct OutputDir {
    root: Dir,
    random: RandomState,
    sequence: AtomicU64,
}

impl OutputDir {
    /// Create `path` if needed and take a capability on it.
    ///
    /// # Errors
    /// Returns [`io::Error`] if `path` cannot be created or opened.
    pub(super) fn open(path: &Path) -> io::Result<Self> {
        std::fs::create_dir_all(path)?;
        let root = Dir::open_ambient_dir(path, ambient_authority())?;
        Ok(Self { root, random: RandomState::new(), sequence: AtomicU64::new(0) })
    }

    /// Write `contents` to `path`, relative to the output root, replacing
    /// any existing leaf atomically.
    ///
    /// # Errors
    /// Returns [`io::Error`] with kind `InvalidInput` if `path` is empty,
    /// carries a component that is not a plain name, or traverses a
    /// symlinked directory. Other kinds propagate the underlying failure.
    pub(super) fn write(&self, path: &Path, contents: &[u8]) -> io::Result<()> {
        let (parent, leaf) = self.open_parent(path)?;
        let (temporary, mut file) = self.create_temporary(&parent)?;
        let write_result = file.write_all(contents);
        drop(file);

        if let Err(error) = write_result {
            let _ = parent.remove_file_or_symlink(&temporary);
            return Err(error);
        }

        let replace_result = replace_leaf(&parent, &temporary, &leaf);
        if replace_result.is_err() {
            let _ = parent.remove_file_or_symlink(&temporary);
        }
        replace_result
    }

    fn open_parent(&self, path: &Path) -> io::Result<(Dir, OsString)> {
        let mut components = validated_components(path)?;
        let leaf = components.pop().ok_or_else(|| invalid_path(path))?;
        let mut parent = self.root.try_clone()?;

        for component in components {
            parent = open_or_create_dir(&parent, &component, path)?;
        }

        Ok((parent, leaf))
    }

    fn create_temporary(&self, parent: &Dir) -> io::Result<(OsString, cap_std::fs::File)> {
        for _ in 0..TEMP_CREATE_ATTEMPTS {
            let temporary = self.temporary_name();
            let mut options = OpenOptions::new();
            options.write(true).create_new(true).follow(FollowSymlinks::No);
            match parent.open_with(&temporary, &options) {
                Ok(file) => return Ok((temporary, file)),
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
                Err(error) => return Err(error),
            }
        }

        Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            "failed to create a unique temporary HTML report file",
        ))
    }

    fn temporary_name(&self) -> OsString {
        let sequence = self.sequence.fetch_add(1, Ordering::Relaxed);
        OsString::from(format!(".oxc-coverage-{:016x}.tmp", self.random.hash_one(sequence)))
    }
}

fn validated_components(path: &Path) -> io::Result<Vec<OsString>> {
    if path.as_os_str().is_empty() {
        return Err(invalid_path(path));
    }

    path.components()
        .map(|component| match component {
            Component::Normal(component) if is_single_component(component) => {
                Ok(component.to_os_string())
            }
            _ => Err(invalid_path(path)),
        })
        .collect()
}

fn is_single_component(component: &OsStr) -> bool {
    let mut components = Path::new(component).components();
    matches!((components.next(), components.next()), (Some(Component::Normal(_)), None))
}

fn open_or_create_dir(parent: &Dir, component: &OsStr, path: &Path) -> io::Result<Dir> {
    match parent.open_dir_nofollow(component) {
        Ok(directory) => Ok(directory),
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            match parent.create_dir(component) {
                Ok(()) => {}
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
                Err(error) => return Err(error),
            }
            parent
                .open_dir_nofollow(component)
                .map_err(|error| map_component_error(parent, component, path, error))
        }
        Err(error) => Err(map_component_error(parent, component, path, error)),
    }
}

fn map_component_error(
    parent: &Dir,
    component: &OsStr,
    path: &Path,
    error: io::Error,
) -> io::Error {
    match parent.symlink_metadata(component) {
        Ok(metadata) if !metadata.is_dir() => invalid_path(path),
        _ => error,
    }
}

fn replace_leaf(parent: &Dir, temporary: &OsStr, leaf: &OsStr) -> io::Result<()> {
    match parent.rename(temporary, parent, leaf) {
        Ok(()) => Ok(()),
        Err(error) => replace_existing_leaf(parent, temporary, leaf, error),
    }
}

/// Windows `rename` refuses to overwrite an existing file, so the leaf is
/// unlinked first. Unix `rename` replaces atomically and never reaches here.
#[cfg(windows)]
fn replace_existing_leaf(
    parent: &Dir,
    temporary: &OsStr,
    leaf: &OsStr,
    error: io::Error,
) -> io::Result<()> {
    if !matches!(error.kind(), io::ErrorKind::AlreadyExists | io::ErrorKind::PermissionDenied)
        || parent.symlink_metadata(leaf).is_err()
    {
        return Err(error);
    }

    match parent.remove_file_or_symlink(leaf) {
        Ok(()) => {}
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => return Err(error),
    }
    parent.rename(temporary, parent, leaf)
}

#[cfg(not(windows))]
fn replace_existing_leaf(
    _parent: &Dir,
    _temporary: &OsStr,
    _leaf: &OsStr,
    error: io::Error,
) -> io::Result<()> {
    Err(error)
}

fn invalid_path(path: &Path) -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidInput,
        format!("unsafe HTML report output path: {}", path.display()),
    )
}
