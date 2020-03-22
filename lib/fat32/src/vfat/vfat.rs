use core::fmt::Debug;
use core::marker::PhantomData;
use core::mem::size_of;
use core::borrow::BorrowMut;

use alloc::string::String;
use alloc::vec::Vec;

use shim::io;
use shim::ioerr;
use shim::newioerr;
use shim::path;
use shim::path::Path;

use crate::mbr::MasterBootRecord;
use crate::traits::{BlockDevice, FileSystem};
use crate::util::SliceExt;
use crate::vfat::{BiosParameterBlock, CachedPartition, Partition, Metadata};
use crate::vfat::{Cluster, Dir, Entry, Error, FatEntry, File, Status};

/// A generic trait that handles a critical section as a closure
pub trait VFatHandle: Clone + Debug + Send + Sync {
    fn new(val: VFat<Self>) -> Self;
    fn lock<R>(&self, f: impl FnOnce(&mut VFat<Self>) -> R) -> R;
}

#[derive(Debug)]
pub struct VFat<HANDLE: VFatHandle> {
    phantom: PhantomData<HANDLE>,
    device: CachedPartition,
    pub bytes_per_sector: u16,
    pub sectors_per_cluster: u8,
    sectors_per_fat: u32,
    fat_start_sector: u64,
    data_start_sector: u64,
    pub root_dir_cluster: Cluster,
}

impl<HANDLE: VFatHandle> VFat<HANDLE> {
    pub fn from<T>(mut device: T) -> Result<HANDLE, Error>
    where
        T: BlockDevice + 'static,
    {
        let mbr: MasterBootRecord = MasterBootRecord::from(&mut device)?;
        for i in 0..4 {
            let _partition = mbr.get_partition(i);
            match _partition.partition_type {
                0xB | 0xC => {
                    let partition_start = _partition.relative_sector as u64;
                    let ebpb = BiosParameterBlock::from(&mut device, partition_start).unwrap(); // break point
                    let logical_sectors_number = if ebpb.total_logical_sectors != 0 {
                        ebpb.total_logical_sectors as u64
                    }
                    else {
                        ebpb.total_logical_sectors_2 as u64
                    };
                    let partition = Partition {
                        start: partition_start,
                        num_sectors: logical_sectors_number as u64, // TODO
                        sector_size: ebpb.bytes_per_sector as u64, 
                    };

                    let cached_device = CachedPartition::new(device, partition);
                    let fat_start_sector = ebpb.reserved_sectors as u64;
                    let data_start_sector = ebpb.reserved_sectors as u64 + ebpb.sectors_per_fat_32 as u64 * ebpb.number_of_fat as u64; //TODO
                    let vfat =  VFat {
                        phantom: PhantomData,
                        device: cached_device,
                        bytes_per_sector: ebpb.bytes_per_sector,
                        sectors_per_cluster: ebpb.sectors_per_cluster,
                        sectors_per_fat: ebpb.sectors_per_fat_32,
                        fat_start_sector: fat_start_sector,
                        data_start_sector: data_start_sector,
                        root_dir_cluster: Cluster::from(ebpb.root_dir_cluster_number),
                    };
                    return Ok(VFatHandle::new(vfat));
                },
                _ => {}
            }
        }
        Err(Error::Io(io::Error::new(io::ErrorKind::InvalidData, "fat32 not found!")))
    }

    // TODO: The following methods may be useful here:
    
    //  * A method to read from an offset of a cluster into a buffer.
    
    pub fn read_cluster(&mut self, cluster: Cluster, offset: usize, buf: &mut [u8]) -> io::Result<usize> {
        use core::cmp::min;

        if !cluster.is_valid() {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "Invalid Cluster"));
        }

        let sector_size = self.device.sector_size() as usize;
        let size = min(
            buf.len(),
            self.bytes_per_sector as usize * self.sectors_per_cluster as usize - offset,
        );

        let mut current_sector = self.data_start_sector
            + cluster.cluster_index() as u64 * self.sectors_per_cluster as u64
            + offset as u64 / self.bytes_per_sector as u64;

        let mut bytes_read = 0;
        let mut offset_in_sector = offset % self.bytes_per_sector as usize;
        
        while bytes_read < size {
            let content = self.device.get(current_sector)?;
            let copy_size = min(size - bytes_read, sector_size - offset_in_sector);
            buf[bytes_read..bytes_read+copy_size].copy_from_slice(&content[offset_in_sector..offset_in_sector+copy_size]);
            offset_in_sector = 0;
            bytes_read += copy_size;
            current_sector += 1;
        } 

        Ok(size)
    }
    
    //  * A method to read all of the clusters chained from a starting cluster
    //    into a vector.
    
    pub fn read_chain(&mut self, start: Cluster, buf: &mut Vec<u8>) -> io::Result<usize> {
        let mut bytes_read = 0;

        let mut current_cluster = start;
        let mut cluster_number = 0;
        loop {
            cluster_number = cluster_number + 1;
            let current_entry = self.fat_entry(current_cluster)?;
            match current_entry.status() {
                Status::Data(next_cluster) => {
                    let bytes_per_cluster = self.bytes_per_sector as usize * self.sectors_per_cluster as usize;
                    buf.resize(bytes_per_cluster * cluster_number, 0);
                    bytes_read += self.read_cluster(current_cluster, 0, &mut buf[bytes_per_cluster * (cluster_number -1)..])?;
                    current_cluster = next_cluster;
                },
                Status::Eoc(_) => {
                    let bytes_per_cluster = self.bytes_per_sector as usize * self.sectors_per_cluster as usize;
                    buf.resize(bytes_per_cluster * cluster_number, 0);
                    bytes_read += self.read_cluster(current_cluster, 0, &mut buf[bytes_per_cluster * (cluster_number -1)..])?;
                    return Ok(bytes_read);
                },
                _ => return Err(io::Error::new(io::ErrorKind::InvalidData, "invalid cluster chain")),
            }
        }
    }
    
    //  * A method to return a reference to a `FatEntry` for a cluster where the
    //    reference points directly into a cached sector.
    
    pub fn fat_entry(&mut self, cluster: Cluster) -> io::Result<&FatEntry> {
        // if !cluster.is_valid() {
        //     return Err(io::Error::new(io::ErrorKind::InvalidData, ""));
        // }

        // let fat_width = size_of::<FatEntry>();

        // let cluster_in_fat_sector = cluster.cluster_number() * fat_width as u32 / self.bytes_per_sector as u32;
        // let data = self.device.get(self.fat_start_sector + cluster_in_fat_sector as u64)?;

        // let index = (cluster.cluster_number() * fat_width as u32 - cluster_in_fat_sector * self.bytes_per_sector as u32) as usize;
        // let entry = unsafe {&data[index..index+fat_width].cast()[0]};
        // Ok(entry)

        let cluser_num_in_sector: u64 = cluster.cluster_number() as u64 * size_of::<FatEntry>() as u64
            / self.bytes_per_sector as u64;
        let entry_offset: usize = cluster.cluster_number() as usize * size_of::<FatEntry>() % self.bytes_per_sector as usize;
        let content = self.device.get(self.fat_start_sector + cluser_num_in_sector)?;
        let entries: &[FatEntry] = unsafe {content.cast()};
        Ok(&entries[entry_offset/size_of::<FatEntry>()])
    }

    pub fn find_next_cluster(&mut self, cluster: Cluster) -> io::Result<Cluster> {
        let cand_entry = self.fat_entry(cluster);
        match cand_entry {
            Ok(entry) => {
                match entry.status() {
                    Status::Data(next_cluster) => Ok(next_cluster),
                    Status::Eoc(cluster) => Err(io::Error::new(io::ErrorKind::Other, "break")),
                    _ => Err(io::Error::new(io::ErrorKind::InvalidData, "Invalid cluster chain")),
                }
            },
            Err(e) => Err(e),
        }
    }
}

impl<'a, HANDLE: VFatHandle> FileSystem for &'a HANDLE {
    type File = File<HANDLE>;
    type Dir = Dir<HANDLE>;
    type Entry = Entry<HANDLE>;

    fn open<P: AsRef<Path>>(self, path: P) -> io::Result<Self::Entry> {
        // let dir_entries = self.lock(|vfat| vfat.get_entries(path))?;
        use shim::path::Component;
        let path_ref = path.as_ref();
        if !path_ref.is_absolute() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "path must be absolute",
            ));
        }
        let mut dir_entries = Vec::new();
        for component in path_ref.components() {
            match component {
                Component::RootDir => {
                    dir_entries.truncate(0);
                    dir_entries.push(Entry::Dir(
                        // new root directory entry
                        Dir{
                            cluster: self.lock(|vfat| vfat.root_dir_cluster),
                            vfat: self.clone(),
                            short_name: String::new(),
                            long_name: String::new(),
                            metadata: Metadata::default(),
                        }
                    ))
                },
                Component::CurDir => {},
                Component::Normal(name) => {
                    use crate::traits::Entry;
                    let new_entry = match dir_entries.last() {
                        Some(current_entry) => match current_entry.as_dir() {
                            Some(dir) => dir.find(name)?,
                            None => return Err(io::Error::new(
                                io::ErrorKind::NotFound,
                                "file not found",
                            )),
                        }
                        None => return Err(io::Error::from(io::ErrorKind::NotFound)),
                    };
                    dir_entries.push(new_entry);
                },
                Component::ParentDir => {
                    if dir_entries.len() > 0 {
                        dir_entries.pop();
                    } else {
                        return Err(io::Error::from(io::ErrorKind::Other));
                    }
                },
                _ => unimplemented!()
            }
        }

        match dir_entries.into_iter().last() {
            Some(current_entry) => Ok(current_entry),
            None => Err(io::Error::from(io::ErrorKind::NotFound)),
        }
    }
}
