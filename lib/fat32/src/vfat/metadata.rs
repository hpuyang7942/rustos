use core::fmt;

use alloc::string::String;

use crate::traits;

/// A date as represented in FAT32 on-disk structures.
#[repr(C, packed)]
#[derive(Default, Debug, Copy, Clone, PartialEq, Eq)]
pub struct Date(u16);

/// Time as represented in FAT32 on-disk structures.
#[repr(C, packed)]
#[derive(Default, Debug, Copy, Clone, PartialEq, Eq)]
pub struct Time(u16);

/// File attributes as represented in FAT32 on-disk structures.
#[repr(C, packed)]
#[derive(Default, Debug, Copy, Clone, PartialEq, Eq)]
pub struct Attributes(u8);

/// A structure containing a date and time.
#[derive(Default, Copy, Clone, Debug, PartialEq, Eq)]
pub struct Timestamp {
    pub date: Date,
    pub time: Time,
}

/// Metadata for a directory entry.
#[repr(packed(1))]
#[derive(Default, Debug, Clone, Copy)]
pub struct Metadata {
    // FIXME: Fill me in.
    pub attributes: Attributes,
    __reserved: u8,
    tenths_creation_time: u8,
    creation_time: Time,
    creation_date: Date,
    last_access_date: Date,
    high_cluster_number: u16,
    last_modification_time: Time,
    last_modification_date: Date,
    low_cluster_number: u16,
}

impl Metadata {
    pub fn start_cluster(&self) -> u32 {
        ((self.high_cluster_number as u32) << 16) + self.low_cluster_number as u32
    }
}

impl Attributes {
    pub fn directory(&self) -> bool {
        (self.0 & 0x10) != 0
    }
}

// FIXME: Implement `traits::Timestamp` for `Timestamp`.
impl traits::Timestamp for Timestamp {
    fn year(&self) -> usize {
        ((self.date.0 >> 9) & 0x7F) as usize + 1980
    }

    fn month(&self) -> u8 {
        (self.date.0 >> 5) as u8 & 0b1111
    }

    fn day(&self) -> u8 {
        self.date.0 as u8 & 0b11111
    }

    fn hour(&self) -> u8 {
        (self.time.0 >> 11) as u8
    }

    fn minute(&self) -> u8 {
        (self.time.0 >> 5) as u8 & 0b111111
    }

    fn second(&self) -> u8 {
        ((self.time.0 as u8) & 0b11111) * 2
    }
}

// FIXME: Implement `traits::Metadata` for `Metadata`.
impl traits::Metadata for Metadata {
    type Timestamp = Timestamp;

    fn read_only(&self) -> bool {
        self.attributes.0 & 0x01 != 0
    }

    fn hidden(&self) -> bool {
        self.attributes.0 & 0x02 != 0
    }

    fn created(&self) -> Self::Timestamp {
        Timestamp {
            date: self.creation_date,
            time: self.creation_time,
        }
    }

    fn accessed(&self) -> Self::Timestamp {
        Timestamp {
            date: self.last_access_date,
            time: Time(0),
        }
    }

    fn modified(&self) -> Self::Timestamp {
        Timestamp {
            date: self.last_modification_date,
            time: self.last_modification_time,
        }
    }
}

// FIXME: Implement `fmt::Display` (to your liking) for `Metadata`.
impl fmt::Display for Metadata {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        use traits::Metadata;
        f.debug_struct("Metadata")
            .field("cluster", &self.start_cluster())
            .field("read_only", &self.read_only())
            .field("hidden", &self.hidden())
            .field("created", &self.created())
            .field("accessed", &self.accessed())
            .field("modified", &self.modified())
            .finish()
    }
}

impl fmt::Display for Timestamp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        use traits::Timestamp;
        f.write_fmt(format_args!(
            "{:4} - {:2} - {:2}  {:2}:{:2}:{:2}",
            self.year(), self.month(), self.day(), self.hour(), self.minute(), self.second()
        ))
    }
}