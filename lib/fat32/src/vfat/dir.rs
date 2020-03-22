use alloc::string::String;
use alloc::vec::Vec;

use shim::const_assert_size;
use shim::ffi::OsStr;
use shim::io;
use shim::newioerr;

use crate::traits;
use crate::util::VecExt;
use crate::vfat::{Attributes, Date, Metadata, Time, Timestamp};
use crate::vfat::{Cluster, Entry, File, VFatHandle};

#[derive(Debug)]
pub struct Dir<HANDLE: VFatHandle> {
    // FIXME: Fill me in.
    pub cluster: Cluster,
    pub vfat: HANDLE,
    pub short_name: String,
    pub long_name: String,
    pub metadata: Metadata,
}

#[repr(C, packed)]
#[derive(Copy, Clone)]
pub struct VFatRegularDirEntry {
    // FIXME: Fill me in.
    file_name: [u8; 8],
    file_extension: [u8; 3],
    pub metadata: Metadata,
    size: u32,
}

const_assert_size!(VFatRegularDirEntry, 32);

#[repr(C, packed)]
#[derive(Copy, Clone)]
pub struct VFatLfnDirEntry {
    // FIXME: Fill me in.
    sequence: u8,
    name: [u16; 5],
    attributes: Attributes,
    entry_type: u8,
    checksum: u8,
    name_2: [u16; 6],
    reserved: u16,
    name_3: [u16; 2], 
}

const_assert_size!(VFatLfnDirEntry, 32);

#[repr(C, packed)]
#[derive(Copy, Clone)]
pub struct VFatUnknownDirEntry {
    // FIXME: Fill me in.
    entry_type: u8,
    reserved_1: [u8; 10],
    attributes: u8,
    reserved_2: [u8; 20],
}

const_assert_size!(VFatUnknownDirEntry, 32);

pub union VFatDirEntry {
    unknown: VFatUnknownDirEntry,
    regular: VFatRegularDirEntry,
    long_filename: VFatLfnDirEntry,
}

impl VFatUnknownDirEntry {
    fn is_deleted_or_unused(&self) -> bool {
        self.entry_type == 0xE5
    }

    fn prev_is_last_entry(&self) -> bool {
        self.entry_type == 0x00
    }

    fn is_regular_directory(&self) -> bool {
        (self.attributes & 0x10) == 0x10
    }

    fn is_lnf(&self) -> bool {
        self.attributes == (0x01 | 0x02 | 0x04 | 0x08)
    }
}

impl<HANDLE: VFatHandle> Dir<HANDLE> {
    /// Finds the entry named `name` in `self` and returns it. Comparison is
    /// case-insensitive.
    ///
    /// # Errors
    ///
    /// If no entry with name `name` exists in `self`, an error of `NotFound` is
    /// returned.
    ///
    /// If `name` contains invalid UTF-8 characters, an error of `InvalidInput`
    /// is returned.
    pub fn find<P: AsRef<OsStr>>(&self, name: P) -> io::Result<Entry<HANDLE>> {
        use traits::{Dir, Entry};

        let name_str = match name.as_ref().to_str() {
            Some(name_str) => name_str,
            None => {
                return Err(io::Error::new(io::ErrorKind::InvalidInput, "invalid UTF-8 name"));
            }
        };

        // let name_str = name.as_ref().to_str().unwrap();

        for t in self.entries()? {
            if name_str.eq_ignore_ascii_case(t.name()) {
                return Ok(t);
            }
        }
        Err(io::Error::new(io::ErrorKind::NotFound, format!("not found, {}", name_str)))
    }

    pub fn name(&self) -> &str {
        if !self.long_name.is_empty() {
            self.long_name.as_str()
        }
        else {
            self.short_name.as_str()
        }
    }
}

impl<HANDLE: VFatHandle> traits::Dir for Dir<HANDLE> {
    // FIXME: Implement `trait::Dir` for `Dir`.
    type Entry = Entry<HANDLE>;
    type Iter =  EntryIterator<HANDLE>;

    fn entries(&self) -> io::Result<Self::Iter> {
        let mut buf = Vec::new();
        self.vfat.lock(|vfat| vfat.read_chain(self.cluster, &mut buf))?;
        Ok(EntryIterator{
            vfat: self.vfat.clone(),
            curr_index: 0,
            data: unsafe {buf.cast()},
        })
    }
}

pub struct EntryIterator<HANDLE: VFatHandle> {
    pub vfat: HANDLE,
    curr_index: usize,
    data: Vec<VFatDirEntry>,
}

impl<HANDLE: VFatHandle> Iterator for EntryIterator<HANDLE> {
    type Item = Entry<HANDLE>;

    fn next(&mut self) -> Option<Self::Item> {
        let mut raw_long_file_name = [0u16; 260];
        let current_index = self.curr_index;
        while self.curr_index < self.data.len() {
            let entry: &VFatDirEntry = self.data.get(self.curr_index).unwrap();

            let unknown = unsafe {entry.unknown};

            if unknown.is_deleted_or_unused() {
                // Deleted entry
                self.curr_index += 1;
                continue;
            }
            else if unknown.prev_is_last_entry() {
                // End of FAT
                return None;
            }
            
            self.curr_index += 1;
            if unknown.is_lnf() {
                let lnf = unsafe {entry.long_filename};

                let lnf_index = (lnf.sequence & 0b11111) as usize - 1;
                let slot = lnf_index * 13;
                
                unsafe {
                    raw_long_file_name[slot..slot+5].copy_from_slice(&lnf.name);
                    raw_long_file_name[slot + 5..slot + 11].copy_from_slice(&lnf.name_2);
                    raw_long_file_name[slot + 11..slot + 13].copy_from_slice(&lnf.name_3);
                }
            }
            else {
                let regular_entry = unsafe { entry.regular };

                let mut short_file_name = regular_entry.file_name;

                if short_file_name[0] == 0x05 {
                    short_file_name[0] = 0xE5;
                }
                let name = ::core::str::from_utf8(&short_file_name).unwrap().trim_end();
                let ext = ::core::str::from_utf8(&regular_entry.file_extension).unwrap().trim_end();

                let mut short_name = String::from(name);
                if !ext.is_empty() {
                    short_name.push_str(".");
                    short_name.push_str(ext);
                }
                let mut nul_byte_index = None;
                for (i, byte) in raw_long_file_name.iter().enumerate() {
                    if *byte == 0 {
                        nul_byte_index = Some(i);
                        break;
                    }
                }
                let long_name = String::from_utf16(if let Some(len) = nul_byte_index {
                    &raw_long_file_name[0..len]
                } else {
                    &raw_long_file_name
                }).unwrap();

                if regular_entry.metadata.attributes.directory() {
                    return Some(Entry::Dir(Dir {
                        cluster: Cluster::from(regular_entry.metadata.start_cluster()),
                        vfat: self.vfat.clone(),
                        short_name,
                        long_name,
                        metadata: regular_entry.metadata,
                    }));
                }
                else {
                    return Some(Entry::File(File {
                        short_name: short_name,
                        long_name: long_name, 
                        metadata: regular_entry.metadata,
                        start_cluster: Cluster::from(regular_entry.metadata.start_cluster()),
                        vfat: self.vfat.clone(),
                        size: regular_entry.size,
                        offset: 0,
                        curr_cluster: Some(Cluster::from(regular_entry.metadata.start_cluster())),
                    }));
                }
                // let file_name = if short_file_name[0] == 0x00 {
                //     let name = core::str::from_utf8(&regular_entry.file_name).unwrap().trim_end();
                //     let extension = core::str::from_utf8(&regular_entry.file_extension).unwrap().trim_end();
                //     let mut short_name = String::from(name);
                //     if !extension.is_empty() {
                //         short_name.push('.');
                //         short_name.push_str(extension);
                //     }
                //     short_name
                // } else {
                //     let mut len = short_file_name.len();
                //     for i in 0..len {
                //         if short_file_name[i] == 0x00 || short_file_name[i] == 0xFF {
                //             len = i;
                //             break;
                //         }
                //     }

                //     let long_file_name = ::core::char::decode_utf16(raw_long_file_name[0..len].iter().cloned())
                //         .map(|r| r.unwrap())
                //         .collect::<String>();
                //     long_file_name
                // };
            }
        }
        None
    }
}
