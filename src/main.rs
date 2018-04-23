extern crate getopts;
extern crate memmap;
extern crate mach_object;

use getopts::Options;
use memmap::Mmap;
use mach_object::OFile;

use std::env;
use std::fs;
use std::path::Path;
use std::process::exit;
use std::io::{Cursor};

mod context;
use context::FileContext;
mod process;
use process::MachOProcess;

fn print_usage(program: &str, opts: Options) {
    let brief = format!(
        "Usage: {} [options] <object file> ...",
        program
    );

    print!("{}", opts.usage(&brief));
}

fn main() {

    let args : Vec<String> = env::args().collect();
    let program = Path::new(args[0].as_str())
        .file_name()
        .unwrap()
        .to_str()
        .unwrap();

    let mut opts = Options::new();
    opts.optopt("", "sdk", "Specifies the sdk path", "sdk_path");
    opts.optopt("", "tool", "Specifies the ToolChain path", "tool_chain");

    let matchs = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(_) => {
            print_usage(&program, opts);

            exit(-1);
        }
    };

    if matchs.free.is_empty() {
        println!("at least one file must be specified");

        print_usage(&program, opts);

        exit(-1);
    }

    let mut mach_process = MachOProcess::new(matchs.opt_str("sdk"), matchs.opt_str("tool"));
    for file_name in matchs.free {

        let file = fs::File::open(file_name).unwrap();
        let mmap = unsafe { Mmap::map(&file).unwrap() };
        let payload: &[u8] = mmap.as_ref();
        let mut cur = Cursor::new(payload);
        let ofile = OFile::parse(&mut cur).unwrap();
        let mut filectx = FileContext::new(payload);

        mach_process.handle_ofile(&ofile, &mut filectx);
    }
}
