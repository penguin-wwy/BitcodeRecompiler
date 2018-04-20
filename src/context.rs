extern crate failure;

use self::failure::Error;

use std::path::Path;
use std::io::{Cursor, Read, Write, stderr};

const DEFAULT_SDK: &'static str = "/Applications/Xcode.app/Contents/Developer/Platforms/iPhoneOS.platform/Developer/SDKs/iPhoneOS.sdk";
const DEFAULT_TOOLCHAIN: &'static str = "/Applications/Xcode.app/Contents/Developer/Toolchains/XcodeDefault.xctoolchain";

pub struct FileContext<'a> {
    pub cur: Cursor<&'a [u8]>,
}

impl<'a> FileContext<'a> {
    pub fn new(payload: &'a [u8]) -> FileContext<'a> {
        FileContext {
            cur: Cursor::new(payload),
        }
    }

    pub fn hexdump(&mut self, addr: usize, size: usize) -> Result<Vec<u8>, Error> {
        let mut w = Vec::new();

        for off in 0..size {
            if (off % 16) == 0 {
                if off > 0 {
                    writeln!(&mut w, "")?;
                }

                write!(&mut w, "{:016x}\t", addr + off)?;
            }

            write!(&mut w, "{:02x} ", self.read_u8()?)?;
        }

        writeln!(&mut w, "")?;

        Ok(w)
    }

    pub fn section_hex(&mut self, _addr: usize, size: usize) -> Result<Vec<u8>, Error> {

        let mut w = Vec::new();

        for _ in 0..size {
            w.push(self.read_u8()?);
        }

        Ok(w)
    }

    #[inline]
    fn read_u8(&mut self) -> Result<u8, Error> {

        let mut buf = [0; 1];
        try!(self.cur.read_exact(&mut buf));
        Ok(buf[0])
    }
}

pub struct ReCompilerInfo {
    sdk_path: String,
    pub obj_file: Vec<String>,
    pub link_framework: Vec<String>,
    pub link_options: Vec<String>,
    pub file_compile: Vec<Vec<String>>,
}

impl ReCompilerInfo {
    pub fn new(sdk_path: Option<String>) -> ReCompilerInfo {
        let mut sdk = String::new();
        match sdk_path {
            Some(s) => {
                println!("ReCompilerInfo new ...");
                if Path::new(&s).exists() {
                    sdk.push_str(s.as_str());
                } else {
                    writeln!(stderr(), "sdk path is not exist.");
                }
            },
            None => {
                println!("ReCompilerInfo err ...");
                if Path::new(DEFAULT_SDK).exists() {
                    sdk.push_str(DEFAULT_SDK);
                }
            },
        }

        ReCompilerInfo {
            sdk_path: sdk,
            obj_file: Vec::new(),
            link_framework: Vec::new(),
            link_options: Vec::new(),
            file_compile: Vec::new(),
        }
    }

    pub fn push_framework(&mut self, framework: &mut String) {

        if let Some(_) = framework.find("/usr/lib") {
            return;
        }

        let off = match framework.rfind('/') {
            Some(index) => {
                index + 1
            }
            None => {
                0
            }
        };
        self.link_framework.push(framework.split_off(off));
    }
    pub fn push_option(&mut self, option: String) {
        self.link_options.push(option);
    }
    pub fn add_new_file_vec(&mut self, file_name: String) {
        self.file_compile.push(Vec::new());
        self.file_compile.last_mut().unwrap().push(file_name);
    }
    pub fn add_file_cmd(&mut self, cmd: String) {
        self.file_compile.last_mut().unwrap().push(cmd);
    }
}