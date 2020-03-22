use crate::vfat::*;
use core::fmt;

use self::Status::*;

#[derive(Debug, PartialEq)]
pub enum Status {
    /// The FAT entry corresponds to an unused (free) cluster.
    Free,
    /// The FAT entry/cluster is reserved.
    Reserved,
    /// The FAT entry corresponds to a valid data cluster. The next cluster in
    /// the chain is `Cluster`.
    Data(Cluster),
    /// The FAT entry corresponds to a bad (disk failed) cluster.
    Bad,
    /// The FAT entry corresponds to a valid data cluster. The corresponding
    /// cluster is the last in its chain.
    Eoc(u32),
}

#[repr(C, packed)]
pub struct FatEntry(pub u32);

impl FatEntry {
    /// Returns the `Status` of the FAT entry `self`.
    pub fn status(&self) -> Status {
        match self.0 & 0x0FFFFFFF {
            0x00000000 => Free,
            0x00000001 => Reserved,
            cluster @ 0x00000002..=0x0FFFFFEF => Data(Cluster::from(cluster)),
            0x0FFFFFF0..=0x0FFFFFF6 => Reserved,
            0x0FFFFFF7 => Bad,
            cluster @ 0x0FFFFFF8..=0x0FFFFFFFF => Eoc(cluster),
            _ => unreachable!()
        }
    }

    pub fn next_cluster(&self) -> Option<Cluster> {
        match self.status() {
            Data(cluster) => Some(cluster),
            Eoc(cluster) => Some(Cluster::from(cluster)),
            _ => None,
        }
    }
}

impl fmt::Debug for FatEntry {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("FatEntry")
            .field("value", &{ self.0 })
            .field("status", &self.status())
            .finish()
    }
}
