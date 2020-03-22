use shim::io;
use shim::path::{Path, PathBuf};

use stack_vec::StackVec;

use pi::atags::Atags;
use pi::atags::Atag;

use alloc::string::String;

use fat32::traits::FileSystem;
use fat32::traits::{Dir, Entry, Metadata, Timestamp};

use crate::console::{kprint, kprintln, CONSOLE};
use crate::ALLOCATOR;
use crate::FILESYSTEM;

/// Error type for `Command` parse failures.
#[derive(Debug)]
enum Error {
    Empty,
    TooManyArgs,
}

/// A structure representing a single shell command.
struct Command<'a> {
    args: StackVec<'a, &'a str>,
}

impl<'a> Command<'a> {
    /// Parse a command from a string `s` using `buf` as storage for the
    /// arguments.
    ///
    /// # Errors
    ///
    /// If `s` contains no arguments, returns `Error::Empty`. If there are more
    /// arguments than `buf` can hold, returns `Error::TooManyArgs`.
    fn parse(s: &'a str, buf: &'a mut [&'a str]) -> Result<Command<'a>, Error> {
        let mut args = StackVec::new(buf);
        for arg in s.split(' ').filter(|a| !a.is_empty()) {
            args.push(arg).map_err(|_| Error::TooManyArgs)?;
        }

        if args.is_empty() {
            return Err(Error::Empty);
        }

        Ok(Command { args })
    }

    fn split_path(s: &'a str, buf: &'a mut [&'a str]) -> Result<Command<'a>, Error> {
        let mut args = StackVec::new(buf);
        for arg in s.split('/').filter(|a| !a.is_empty()) {
            args.push(arg).map_err(|_| Error::TooManyArgs)?;
        }
        if args.is_empty() {
            return Err(Error::Empty);
        }

        Ok(Command { args })
    }

    /// Returns this command's path. This is equivalent to the first argument.
    fn path(&self) -> &str {
        self.args[0]
    }
}

fn echo_command(args: &[&str]) {
    let len = args.len();

    if len > 0 {
        for t in args[..len-1].iter() {
            kprint!("{}", t);
            kprint!(" ");
        }
        kprintln!("{}", args[len-1]);
    }
}

fn ls_command(mut args: &[&str], pwd: &mut PathBuf) {
    if args.len() > 2 {
        kprintln!("Invalid Input. Usage:");
        kprintln!("ls [-a] [directory]");
        kprintln!();
        return;
    }

    let show_hidden = args.len() > 0 && args[0] == "-a";
    if show_hidden {
        args = &args[1..];
    }

    let mut dir = pwd.clone();

    if !args.is_empty() {
        if args[0] == "." {
            // do nothing
        }
        else if args[0] == ".." {
            dir.pop();
        }
        else {
            dir.push(args[0]);
        }
    }

    let entry_result = FILESYSTEM.open(dir.as_path());

    match entry_result {
        Err(e) => {
            kprintln!("Path not found! Error Msg: {:?}", e);
            return;
        },
        Ok(entry) => {
            if let Some(dir_entry) = entry.into_dir() {
                let mut entries = dir_entry.entries().expect("List dir");
                for item in entries {
                    if show_hidden || !item.metadata().hidden() {
                        print_entry(&item);
                        kprintln!();
                    }
                }
            } else {
                kprintln!("Not a directory.");
            }
            return;
        }
    }
}

fn cat_command(args: &[&str], pwd: &PathBuf) {
    if args.len() != 1 {
        kprintln!("Invalid Input. Usage:");
        kprintln!("cat <path..>");
        kprintln!();
        return;
    }

    let mut dir = pwd.clone();

    for arg in args[0].split("/").filter(|a| !a.is_empty()) {
        if arg == "." {
            // do nothing
        }
        else if arg == ".." {
            dir.pop();
        }
        else {
            dir.push(arg);
        }
    }

    let entry_result = FILESYSTEM.open(dir.as_path());
    if entry_result.is_err() {
        kprintln!("file not found");
        return;
    }
    let entry = entry_result.unwrap();
    if let Some(ref mut file) = entry.into_file() {
        loop {
            use shim::io::Read;

            let mut buffer = [0u8; 512];
            match file.read(&mut buffer) {
                Ok(0) => break,
                Ok(_) => kprint!("{}", String::from_utf8_lossy(&buffer)),
                Err(e) => kprint!("Failed to read file: {:?}", e)
            }
        }
        kprintln!("");
    }
    else {
        kprintln!("Not a file.");
    }
}

fn pwd_command(args: &[&str], pwd: &PathBuf) {
    let len = args.len();
    if len > 0 {
        kprintln!("Too many args. Usage: ");
        kprintln!("pwd");
        kprintln!();
        return;
    }
    kprintln!("{}", pwd.as_path().display());
}

fn cd_command(args: &[&str], pwd: &mut PathBuf) {
    let len = args.len();

    if len == 0 {
        return;
    }

    if len != 1 {
        kprintln!("Invalid Input. Usage:");
        kprintln!("cd <directory>");
        kprintln!();
        return;
    }
    
    for arg in args[0].split("/").filter(|a| !a.is_empty()) {
        if arg == "." {
            // do nothing
        }
        else if arg == ".." {
            pwd.pop();
        }
        else {
            let dir = Path::new(arg);
            let mut path = pwd.clone();
            path.push(dir);
    
            let entry = FILESYSTEM.open(path.as_path());
            match entry {
                Err(e) => {
                    kprintln!("cd command: Path not found!");
                    return;
                },
                Ok(possible_dir) => {
                    match possible_dir.as_dir() {
                        Some(something) => {
                            // kprintln!("add {:?} to path, something is {:?}", dir, something);
                            pwd.push(dir);
                        },
                        None => {
                            kprintln!("Not a directory");
                            return;
                        }
                    }
                },
            }
        }
    }
}

fn print_status(b: bool, c: char) {
    if b {kprint!("{}", c);} else {kprint!("-");}
}

fn print_timestamp<T: Timestamp> (ts: T) {
    kprint!("{:02}/{:02}/{} {:02}:{:02}:{:02} ",
            ts.month(), ts.day(), ts.year(), ts.hour(), ts.minute(), ts.second());
}

fn print_entry<E: Entry> (entry: &E) {
    print_status(entry.is_dir(), 'd');
    print_status(entry.is_file(), 'f');
    print_status(entry.metadata().read_only(), 'r');
    print_status(entry.metadata().hidden(), 'h');
    kprint!("\t");

    print_timestamp(entry.metadata().created());
    print_timestamp(entry.metadata().modified());
    print_timestamp(entry.metadata().accessed());
    kprint!("\t");

    // kprint!(entry.metadata().)

    kprint!("{}", entry.name());
}

const BS: u8 = 0x08;
const BEL: u8 = 0x07;
const LF: u8 = 0x0A;
const CR: u8 = 0x0D;
const DEL: u8 = 0x7F;


fn readline(buf: &mut [u8]) -> &str {
    let mut read = 0;
    loop {
        let b = CONSOLE.lock().read_byte();
        match b {
            BS | DEL if read > 0 => {
                read -= 1;
                kprint!("{}", BS as char);
                kprint!(" ");
                kprint!("{}", BS as char);
            }
            LF | CR => {
                kprintln!();
                break;
            }
            _ if read == buf.len() => {
                kprint!("{}", BEL as char);
            }
            byte @ b' '... b'~' => {
                buf[read] = byte;
                read += 1;
                kprint!("{}", byte as char);
            }
            _ => kprint!("{}", BEL as char),
        }
    }
    return core::str::from_utf8(&buf[..read]).unwrap();
}

/// Starts a shell using `prefix` as the prefix for each line. This function
/// never returns.
pub fn shell(prefix: &str) -> ! {
    let mut buf_storage = [0u8; 512];
    let mut pwd = PathBuf::from("/");
    
    loop {
        kprint!("{}", prefix);
        kprint!("(");
        kprint!("{:?}", pwd);
        kprint!(")");
        let command = readline(&mut buf_storage);

        match Command::parse(command, &mut [""; 64]) {
            Err(Error::TooManyArgs) => kprintln!("too many arguments"),
            Err(Error::Empty) => {},
            Ok(command) => {
                match command.path() {
                    "echo" => echo_command(&command.args[1..]),
                    "ls" => ls_command(&command.args[1..], &mut pwd),
                    "cat" => cat_command(&command.args[1..], &mut pwd),
                    "cd" => cd_command(&command.args[1..], &mut pwd),
                    "pwd" => pwd_command(&command.args[1..], &pwd),
                    v => kprintln!("unknown command: {}", v),
                }
            }
        }
    }
}