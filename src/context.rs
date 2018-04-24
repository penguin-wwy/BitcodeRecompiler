extern crate failure;

use self::failure::Error;

use std::path::Path;
use std::io::{Cursor, Read, Write, stderr};

const DEFAULT_IPHONE_SDK: &'static str = "/Applications/Xcode.app/Contents/Developer/Platforms/iPhoneOS.platform/Developer/SDKs/iPhoneOS.sdk";
const DEFAULT_MACOSX_SDK: &'static str = "/Applications/Xcode.app/Contents/Developer/Platforms/MacOSX.platform/Developer/SDKs/MacOSX.sdk";
const DEFAULT_TOOLCHAIN: &'static str = "/Applications/Xcode.app/Contents/Developer/Toolchains/XcodeDefault.xctoolchain/";

const LIB_CLANG_OSX: &'static str = "libclang_rt.osx.a";
const LIB_CLANG_IOS: &'static str = "libclang_rt.ios.a";

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
    pub platform: String,
    pub lib_clang: String,
    pub tool_chain: String,
    pub sdk_path: String,
    pub obj_file: Vec<String>,
    pub link_framework: Vec<String>,
    pub link_options: Vec<String>,
    pub file_compile: Vec<Vec<String>>,
}

impl ReCompilerInfo {
    pub fn new() -> ReCompilerInfo {

        ReCompilerInfo {
            platform: String::new(),
            lib_clang: String::new(),
            tool_chain: String::new(),
            sdk_path: String::new(),
            obj_file: Vec::new(),
            link_framework: Vec::new(),
            link_options: Vec::new(),
            file_compile: Vec::new(),
        }
    }

    pub fn set_platform(&mut self, data: String) {
        self.platform = data;
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

    pub fn choose_path_from_platform(&mut self, sdk_path: Option<String>, tool_chain: Option<String>) -> Result<bool, Error> {

        match self.platform.as_ref() {
            "MacOSX" => {
                self.lib_clang.push_str(LIB_CLANG_OSX);
            },
            "iPhoneOS" => {
                self.lib_clang.push_str(LIB_CLANG_IOS);
            },
            _ => {
                writeln!(stderr(), "no support platform.");
                return Ok(false);
            }
        };

        match sdk_path {
            Some(s) => {
                if !Path::new(&s).exists() {
                    writeln!(stderr(), "sdk path is not exist.");
                    return Ok(false);
                }
                match s.find(self.platform.as_str()) {
                    Some(_) => {},
                    None => {
                        writeln!(stderr(), "the sdk is inconsistent with the platform.");
                        return Ok(false);
                    },
                };
                self.sdk_path.push_str(s.as_str());
            },
            None => {
                let s = String::from(match self.platform.as_ref() {
                    "MacOSX" => {
                        DEFAULT_MACOSX_SDK
                    },
                    "iPhoneOS" => {
                        DEFAULT_IPHONE_SDK
                    },
                    _ => {
                        writeln!(stderr(), "no support platform.");
                        return Ok(false);
                    }
                });
                if !Path::new(&s).exists() {
                    writeln!(stderr(), "no sdk path.");
                    return Ok(false);
                }
                self.sdk_path.push_str(s.as_str());
            },
        };

        match tool_chain {
            Some(s) => {
                if !Path::new(&s).exists() {
                    writeln!(stderr(), "ToolChain path is not exist.");
                    return Ok(false);
                }
                self.tool_chain.push_str(s.as_str());
            },
            None => {
                if Path::new(DEFAULT_TOOLCHAIN).exists() {
                    self.tool_chain.push_str(DEFAULT_TOOLCHAIN);
                } else {
                    writeln!(stderr(), "no ToolChain path.");
                    return Ok(false);
                }
            },
        };

        return Ok(true);
    }
}