use core::fmt;
use shim::const_assert_size;

use crate::traits::BlockDevice;
use crate::vfat::Error;

#[repr(C, packed)]
pub struct BiosParameterBlock {
    // FIXME: Fill me in.
    // BPB
    jump_instruction: [u8; 3],
    oem_dientifier: [u8; 8],
    pub bytes_per_sector: u16,
    pub sectors_per_cluster: u8,
    pub reserved_sectors: u16,
    pub number_of_fat: u8,
    max_directory_entries: u16,
    pub total_logical_sectors: u16,
    descriptor_type: u8,
    pub sectors_per_fat: u16,
    pub sectors_per_track: u16,
    heads_or_sides: u16,
    num_hidden_sectors: u32,
    pub total_logical_sectors_2: u32,

    // EBPB
    pub sectors_per_fat_32: u32,
    flags: u16,
    fat_version: u16,
    pub root_dir_cluster_number: u32,
    fs_info_sector_number: u16,
    back_up_boot_sector_number: u16,
    formated_reserve: [u8; 12],
    drive_number: u8,
    windows_flag: u8,
    signature: u8,
    volume_id: u32,
    volume_label: [u8; 11],
    identifier_string: [u8; 8],
    boot_code: [u8; 420],
    bootable_partition_signature: u16,
}

const_assert_size!(BiosParameterBlock, 512);

impl BiosParameterBlock {
    /// Reads the FAT32 extended BIOS parameter block from sector `sector` of
    /// device `device`.
    ///
    /// # Errors
    ///
    /// If the EBPB signature is invalid, returns an error of `BadSignature`.
    pub fn from<T: BlockDevice>(mut device: T, sector: u64) -> Result<BiosParameterBlock, Error> {
        let mut buf = [0u8; 512];
        match device.read_sector(sector, &mut buf) {
            Ok(size) => {
                if size != 512 {
                    return Err(Error::Io(shim::io::Error::new(shim::io::ErrorKind::UnexpectedEof, "EBPB should be 512 bytes")));
                }
                else {
                    let result: BiosParameterBlock = unsafe {core::mem::transmute(buf)};
                    if result.bootable_partition_signature != 0xaa55 {
                        return Err(Error::BadSignature);
                    }
                    else {
                        Ok(result)
                    }
                }
            }
            Err(err) => Err(Error::Io(err)),
        }
    }
}

impl fmt::Debug for BiosParameterBlock {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("BioParameterBlock")
            .finish()
    }
}
