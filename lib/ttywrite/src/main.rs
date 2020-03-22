mod parsers;

use serial;
use structopt;
use structopt_derive::StructOpt;
use xmodem::{Xmodem, Progress};

use std::io::Write;
use std::path::PathBuf;
use std::time::Duration;

use structopt::StructOpt;
use serial::core::{CharSize, BaudRate, StopBits, FlowControl, SerialDevice, SerialPortSettings};

use parsers::{parse_width, parse_stop_bits, parse_flow_control, parse_baud_rate};

#[derive(StructOpt, Debug)]
#[structopt(about = "Write to TTY using the XMODEM protocol by default.")]
struct Opt {
    #[structopt(short = "i", help = "Input file (defaults to stdin if not set)", parse(from_os_str))]
    input: Option<PathBuf>,

    #[structopt(short = "b", long = "baud", parse(try_from_str = "parse_baud_rate"),
                help = "Set baud rate", default_value = "115200")]
    baud_rate: BaudRate, // used in set_baud_rate

    #[structopt(short = "t", long = "timeout", parse(try_from_str),
                help = "Set timeout in seconds", default_value = "10")]
    timeout: u64, // used in setting_time_out

    #[structopt(short = "w", long = "width", parse(try_from_str = "parse_width"),
                help = "Set data character width in bits", default_value = "8")]
    char_width: CharSize, // used in set_char_size

    #[structopt(help = "Path to TTY device", parse(from_os_str))]
    tty_path: PathBuf, // used

    #[structopt(short = "f", long = "flow-control", parse(try_from_str = "parse_flow_control"),
                help = "Enable flow control ('hardware' or 'software')", default_value = "none")]
    flow_control: FlowControl, // used in set_flow_control

    #[structopt(short = "s", long = "stop-bits", parse(try_from_str = "parse_stop_bits"),
                help = "Set number of stop bits", default_value = "1")]
    stop_bits: StopBits, // used in set_stop_bitss

    #[structopt(short = "r", long = "raw", help = "Disable XMODEM")]
    raw: bool, // check whether raw transfer or Xmodem
}

fn main() {
    use std::fs::File;
    use std::io::{self, BufReader};

    let opt = Opt::from_args();
    let mut port = serial::open(&opt.tty_path).expect("path points to invalid TTY");

    // FIXME: Implement the `ttywrite` utility.
    let mut settings = port.read_settings().expect("failed to load settings");
    settings.set_baud_rate(opt.baud_rate).expect("baud rate should be valid");
    settings.set_char_size(opt.char_width);
    settings.set_flow_control(opt.flow_control);
    settings.set_stop_bits(opt.stop_bits);
    

    port.write_settings(&settings).expect("settings should be valid");
    port.set_timeout(Duration::from_secs(opt.timeout)).expect("timeout should be valid");

    if opt.raw {
        // Raw Input
        match opt.input {
            Some(ref path) => {
                let mut input = BufReader::new(File::open(path).expect("file does not exist"));
                io::copy(&mut input, &mut port).expect("io transfer does not success");
            },
            None => {
                let mut input = io::stdin();
                io::copy(&mut input, &mut port).expect("io transfer does not success");
            }
        }
    }
    else {
        //Xmodem
        match opt.input {
            Some(ref path) => {
                let mut input = BufReader::new(File::open(path).expect("file does not exist"));
                Xmodem::transmit_with_progress(&mut input, &mut port, |progress| {
                    if let Progress::Packet(_) = progress {
                        print!(".");
                        io::stdout().flush().unwrap();
                    } else if let Progress::Started = progress {
                        println!("Started");
                    } else if let Progress::Waiting = progress {
                        println!("Ready");
                    } else {
                        assert!(false);
                    }
                }).expect("io transfer does not success");
            },
            None => {
                let mut input = io::stdin();
                Xmodem::transmit_with_progress(&mut input, &mut port, |progress| {
                    if let Progress::Packet(_) = progress {
                        print!(".");
                        io::stdout().flush().unwrap();
                    } else if let Progress::Waiting = progress {
                        println!("Ready");
                    } else {
                        assert!(false);
                    }
                }).expect("io transfer does not success");
            }
        }
    }
}
