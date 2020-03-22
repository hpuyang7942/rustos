use core::fmt;
use shim::const_assert_size;
use shim::io;

use crate::traits::BlockDevice;

#[repr(C)]
#[derive(Copy, Clone)]
pub struct CHS {
    // FIXME: Fill me in.
    header: u8,
    sector: u8,
    cylinder: u8
}

// FIXME: implement Debug for CHS

impl fmt::Debug for CHS {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("CHS").finish()
    }
}

const_assert_size!(CHS, 3);

#[repr(C, packed)]
pub struct PartitionEntry {
    // FIXME: Fill me in.
    boot_indicator: u8,
    starting_chs: CHS,
    pub partition_type: u8,
    ending_chs: CHS,
    pub relative_sector: u32,
    pub total_sectors: u32,
}

// FIXME: implement Debug for PartitionEntry

impl fmt::Debug for PartitionEntry {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("PartitionEntry").finish()
    }
}

const_assert_size!(PartitionEntry, 16);

/// The master boot record (MBR).
#[repr(C, packed)]
pub struct MasterBootRecord {
    // FIXME: Fill me in.
    bootstrap: [u8; 436],
    disk_id: [u8; 10],
    partition_table: [PartitionEntry; 4],
    signature: u16,
}

// FIXME: implemente Debug for MaterBootRecord

impl fmt::Debug for MasterBootRecord {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("MasterBootRecord")
            .field("disk id", &self.disk_id)
            .field("partitions", &self.partition_table)
            .field("signature", &self.signature)
            .finish()
    }
}

const_assert_size!(MasterBootRecord, 512);

#[derive(Debug)]
pub enum Error {
    /// There was an I/O error while reading the MBR.
    Io(io::Error),
    /// Partiion `.0` (0-indexed) contains an invalid or unknown boot indicator.
    UnknownBootIndicator(u8),
    /// The MBR magic signature was invalid.
    BadSignature,
}

impl From<io::Error> for Error {
    fn from(error: io::Error) -> Self {
        Error::Io(error)
    }
}

impl MasterBootRecord {
    /// Reads and returns the master boot record (MBR) from `device`.
    ///
    /// # Errors
    ///
    /// Returns `BadSignature` if the MBR contains an invalid magic signature.
    /// Returns `UnknownBootIndicator(n)` if partition `n` contains an invalid
    /// boot indicator. Returns `Io(err)` if the I/O error `err` occured while
    /// reading the MBR.
    pub fn from<T: BlockDevice>(mut device: T) -> Result<MasterBootRecord, Error> {
        let mut data = [0u8; 512];
        let bytes = device.read_sector(0, &mut data)?;

        if bytes != 512 {
            return Err(Error::Io(io::Error::new(io::ErrorKind::UnexpectedEof, "MBR should be 512 bytes")));
        }
    
        let masterbootrecord: MasterBootRecord = unsafe {core::mem::transmute(data)};
        if masterbootrecord.signature != 0xAA55 {
            return Err(Error::BadSignature);
        }

        for i in 0..4 {
            let indicator = masterbootrecord.partition_table[i].boot_indicator;
            if indicator != 0x80 && indicator != 0 {
                return Err(Error::UnknownBootIndicator(i as u8));
            }
        }

        Ok(masterbootrecord)
    }

    pub fn get_partition(&self, index: usize) -> &PartitionEntry {
        &self.partition_table[index]
    }
}
