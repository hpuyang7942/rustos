use alloc::string::String;

use shim::io::{self, SeekFrom};

use crate::traits;
use crate::vfat::{Cluster, Metadata, VFatHandle, FatEntry};

#[derive(Debug)]
pub struct File<HANDLE: VFatHandle> {
    // FIXME: Fill me in.
    pub short_name: String,
    pub long_name: String,
    pub metadata: Metadata,
    pub start_cluster: Cluster,
    pub vfat: HANDLE,
    pub size: u32,
    pub offset: u32,
    pub curr_cluster: Option<Cluster>,
}

impl<HANDLE: VFatHandle> File<HANDLE> {
    pub fn new(short_name: String, long_name: String, metadata: Metadata, start_cluster: Cluster, vfat: HANDLE, size: u32) -> File<HANDLE> {
        File {
            short_name,
            long_name,
            metadata,
            start_cluster,
            vfat,
            size,
            offset: 0,
            curr_cluster: Some(start_cluster),
        }
    }
}

impl<HANDLE: VFatHandle> File<HANDLE> {
    pub fn name(&self) -> &str {
        if !self.long_name.is_empty() {
            self.long_name.as_str()
        }
        else {
            self.short_name.as_str()
        }
    }
}

// FIXME: Implement `traits::File` (and its supertraits) for `File`.
impl<HANDLE: VFatHandle> traits::File for File<HANDLE> {
    fn sync(&mut self) -> io::Result<()> {
        unimplemented!()
    }

    fn size(&self) -> u64 {
        self.size as u64
    }
}

impl<HANDLE: VFatHandle> io::Read for File<HANDLE> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let read_size = ::core::cmp::min(buf.len(), self.size as usize - self.offset as usize);
        let mut rest_size = read_size;
        let bytes_per_sector = self.vfat.lock(|vfat| vfat.bytes_per_sector) as u32;
        let sectors_per_cluster = self.vfat.lock(|vfat| vfat.sectors_per_cluster) as u32;
        let bytes_per_cluster = bytes_per_sector * sectors_per_cluster;
        let mut current_cluster = self.curr_cluster;
        let mut current_offset_in_cluster = (self.offset % bytes_per_cluster) as usize;
        let mut buffer_offset = 0;
        while rest_size > 0 {
            let newly_read_size = match self.vfat.lock(|vfat| vfat.read_cluster(
                current_cluster.unwrap(), current_offset_in_cluster, &mut buf[buffer_offset..read_size]
            )) {
                Ok(new_size) => new_size,
                Err(e) => 0,
            };
            if newly_read_size == bytes_per_cluster as usize - current_offset_in_cluster {
                match self.vfat.lock(|vfat| vfat.find_next_cluster(current_cluster.unwrap())) {
                    Ok(next_cluster) => {
                        current_cluster = Some(next_cluster);
                    },
                    Err(e) => {
                        match e.kind() {
                            io::ErrorKind::Other => current_cluster = None,
                            _ => (),
                        }
                    },
                }
            }
            buffer_offset += newly_read_size;
            rest_size -= newly_read_size;
            current_offset_in_cluster = 0;
        }
        self.offset += read_size as u32;
        self.curr_cluster = current_cluster;
        Ok(read_size)
    }
}

impl<HANDLE: VFatHandle> io::Write for File<HANDLE> {
    fn write(&mut self, _buf: &[u8]) -> io::Result<usize> {
        unimplemented!()
    }

    fn flush(&mut self) -> io::Result<()> {
        unimplemented!()
    }
}


impl<HANDLE: VFatHandle> io::Seek for File<HANDLE> {
    /// Seek to offset `pos` in the file.
    ///
    /// A seek to the end of the file is allowed. A seek _beyond_ the end of the
    /// file returns an `InvalidInput` error.
    ///
    /// If the seek operation completes successfully, this method returns the
    /// new position from the start of the stream. That position can be used
    /// later with SeekFrom::Start.
    ///
    /// # Errors
    ///
    /// Seeking before the start of a file or beyond the end of the file results
    /// in an `InvalidInput` error.
    fn seek(&mut self, _pos: SeekFrom) -> io::Result<u64> {
        let seek_offset = match _pos {
            SeekFrom::Current(offset) => self.offset + offset as u32,
            SeekFrom::End(offset) => self.size + offset as u32,
            SeekFrom::Start(offset) => offset as u32,
        };

        if seek_offset >= self.size {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, ""));
        }
        else {
            self.offset = seek_offset;
            let mut vfat =  &self.vfat;
            let bytes_per_cluster = vfat.lock(|vfat| vfat.bytes_per_sector) as u32 * vfat.lock(|vfat| vfat.sectors_per_cluster) as u32;
            let cluster = self.offset/bytes_per_cluster;
            self.curr_cluster = Some(self.start_cluster);
            for i in 0..cluster {
                self.curr_cluster = match vfat.lock(|vfat| vfat.find_next_cluster(self.curr_cluster.unwrap())) {
                    Ok(next_cluster) => Some(next_cluster),
                    Err(e) => {
                        match e.kind() {
                            io::ErrorKind::Other => None,
                            _ => None,
                        }
                    },
                };
            }
            Ok(self.offset as u64)
        }
    }
}
