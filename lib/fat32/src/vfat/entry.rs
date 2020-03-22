use crate::traits;
use crate::vfat::{Dir, File, Metadata, VFatHandle};
use core::fmt;

// You can change this definition if you want
#[derive(Debug)]
pub enum Entry<HANDLE: VFatHandle> {
    File(File<HANDLE>),
    Dir(Dir<HANDLE>),
}

// TODO: Implement any useful helper methods on `Entry`.

impl<HANDLE: VFatHandle> traits::Entry for Entry<HANDLE> {
    // FIXME: Implement `traits::Entry` for `Entry`.
    type File = File<HANDLE>;
    type Dir = Dir<HANDLE>;
    type Metadata = Metadata;

    /// The name of the file or directory corresponding to this entry.
    fn name(&self) -> &str {
        match &self {
            Entry::Dir(dir) => &dir.name(),
            Entry::File(file) => &file.name(),
        }
    }

    /// The metadata associated with the entry.
    fn metadata(&self) -> &Self::Metadata {
        match &self {
            Entry::Dir(dir) => &dir.metadata,
            Entry::File(file) => &file.metadata,
        }
    }

    /// If `self` is a file, returns `Some` of a reference to the file.
    /// Otherwise returns `None`.
    fn as_file(&self) -> Option<&<Self as traits::Entry>::File> {
        match &self {
            Entry::File(file) => Some(file),
            _ => None,
        }
    }

    /// If `self` is a directory, returns `Some` of a reference to the
    /// directory. Otherwise returns `None`.
    fn as_dir(&self) -> Option<&<Self as traits::Entry>::Dir> {
        match &self {
            Entry::Dir(dir) => Some(dir),
            _ => None,
        }
    }

    /// If `self` is a file, returns `Some` of the file. Otherwise returns
    /// `None`.
    fn into_file(self) -> Option<<Self as traits::Entry>::File> {
        match self {
            Entry::File(file) => Some(file),
            _ => None,
        }
    }

    /// If `self` is a directory, returns `Some` of the directory. Otherwise
    /// returns `None`.
    fn into_dir(self) -> Option<<Self as traits::Entry>::Dir> {
        match self {
            Entry::Dir(dir) => Some(dir),
            _ => None,
        }
    }

    /// Returns `true` if this entry is a file or `false` otherwise.
    fn is_file(&self) -> bool {
        self.as_file().is_some()
    }

    /// Returns `true` if this entry is a directory or `false` otherwise.
    fn is_dir(&self) -> bool {
        self.as_dir().is_some()
    }
}
