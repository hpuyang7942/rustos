use core::time::Duration;
use shim::io;
use shim::ioerr;

use fat32::traits::BlockDevice;

use crate::console::kprintln;

use pi::timer::spin_sleep;

extern "C" {
    /// A global representing the last SD controller error that occured.
    static sd_err: i64;

    /// Initializes the SD card controller.
    ///
    /// Returns 0 if initialization is successful. If initialization fails,
    /// returns -1 if a timeout occured, or -2 if an error sending commands to
    /// the SD controller occured.
    fn sd_init() -> i32;

    /// Reads sector `n` (512 bytes) from the SD card and writes it to `buffer`.
    /// It is undefined behavior if `buffer` does not point to at least 512
    /// bytes of memory. Also, the caller of this function should make sure that
    /// `buffer` is at least 4-byte aligned.
    ///
    /// On success, returns the number of bytes read: a positive number.
    ///
    /// On error, returns 0. The true error code is stored in the `sd_err`
    /// global. `sd_err` will be set to -1 if a timeout occured or -2 if an
    /// error sending commands to the SD controller occured. Other error codes
    /// are also possible but defined only as being less than zero.
    fn sd_readsector(n: i32, buffer: *mut u8) -> i32;
}

// FIXME: Define a `#[no_mangle]` `wait_micros` function for use by `libsd`.
// The `wait_micros` C signature is: `void wait_micros(unsigned int);`

#[no_mangle]
pub fn wait_micros(micros: u32) {
    spin_sleep(Duration::from_micros((micros * 100).into()));
}

#[derive(Debug)]
pub enum Error {
    TimedOut,
    SendingCommandFailed,
    UnknownError(i64),
}

/// A handle to an SD card controller.
#[derive(Debug)]
pub struct Sd;

impl From<Error> for io::Error {
    fn from(err: Error) -> io::Error {
        match err {
            Error::TimedOut => {
                io::Error::new(io::ErrorKind::TimedOut, "operation timed out")
            },
            Error::SendingCommandFailed => io::Error::new(
                io::ErrorKind::Other,
                "Sending command to SD card controller failed",
            ),
            Error::UnknownError(err) => io::Error::new(
                io::ErrorKind::Other,
                "Got unknown card error: {}",
            ),
        }
    }
}

impl Sd {
    /// Initializes the SD card controller and returns a handle to it.
    /// The caller should assure that the method is invoked only once during the
    /// kernel initialization. We can enforce the requirement in safe Rust code
    /// with atomic memory access, but we can't use it yet since we haven't
    /// written the memory management unit (MMU).
    pub unsafe fn new() -> Result<Sd, io::Error> {
        let result = sd_init();
        if result == 0 {
            Ok(Sd{})
        }
        else if result == -1 {
            Err(io::Error::new(io::ErrorKind::TimedOut, "operation timed out"))
        }
        else if result == -2 {
            Err(io::Error::new(
                io::ErrorKind::Other,
                "Sending command to SD card controller failed",
            ))
        }
        else {
            Err(io::Error::new(
                io::ErrorKind::Other,
                "Got unknown card error",
            ))
        }
    }
}

impl BlockDevice for Sd {
    /// Reads sector `n` from the SD card into `buf`. On success, the number of
    /// bytes read is returned.
    ///
    /// # Errors
    ///
    /// An I/O error of kind `InvalidInput` is returned if `buf.len() < 512` or
    /// `n > 2^31 - 1` (the maximum value for an `i32`).
    ///
    /// An error of kind `TimedOut` is returned if a timeout occurs while
    /// reading from the SD card.
    ///
    /// An error of kind `Other` is returned for all other errors.
    fn read_sector(&mut self, n: u64, buf: &mut [u8]) -> io::Result<usize> {
        if buf.len() < 512 || n > 0x7FFFFFFF {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "wrong parameter received"));
        }
        let result = unsafe {sd_readsector(n as i32, buf.as_mut_ptr())};

        if result > 0 {
            Ok(result as usize)
        }
        else {
            let errno = unsafe {sd_err};
            if errno == -1 {
                Err(Error::TimedOut.into())
            }
            else if errno == -2 {
                Err(Error::SendingCommandFailed.into())
            }
            else {
                Err(Error::UnknownError(errno).into())
            }
        }
    }

    fn write_sector(&mut self, _n: u64, _buf: &[u8]) -> io::Result<usize> {
        unimplemented!("SD card and file system are read only")
    }
}
