extern crate mach_object;
extern crate failure;
extern crate xml;

use mach_object::*;
use self::failure::Error;
use self::xml::reader::{EventReader, XmlEvent};

use std::process::{Command, Stdio, exit};
use std::fs;
use std::io::{Write, BufReader, stderr, stdout, SeekFrom, Seek};

use context::*;

const SEGMENT_NAME: &'static str = "__LLVM";
const SECTION_NAME: &'static str = "__bundle";

pub struct MachOProcess {
    re_info: ReCompilerInfo,
    xml_file: String,
    xar_file: String,
}

impl MachOProcess {
    pub fn new(sdk_path: Option<String>) -> MachOProcess {
        MachOProcess {
            re_info: ReCompilerInfo::new(sdk_path),
            xml_file: String::from("tmp.xml"),
            xar_file: String::from("tmp.xar"),
        }
    }

    pub fn handle_ofile(&mut self, ofile: &OFile, ctxt: &mut FileContext) -> Result<(), Error> {

        match ofile {
            &OFile::MachFile {
                ref header,
                ref commands,
            } => self.handle_macho_file(header, commands, ctxt),
            &OFile::FatFile { magic, ref files } => {
                Ok(())
            },
            &OFile::ArFile { ref files } => {
                Ok(())
            },
            &OFile::SymDef { ref ranlibs } => {
                Ok(())
            },
        }
    }

    fn handle_macho_file(&mut self, header: &MachHeader, commands: &[MachCommand], ctxt: &mut FileContext) -> Result<(), Error> {
        let commands = commands.iter()
            .map(|load| load.command())
            .cloned()
            .collect::<Vec<LoadCommand>>();

        'outer: for cmd in &commands {
            match *cmd {
                LoadCommand::Segment {ref sections, ..} | LoadCommand::Segment64 {ref sections, ..} => {
                    for ref sect in sections {
                        let name = Some((sect.segname.clone(), Some(sect.sectname.clone())));

                        if name == Some((String::from(SEGMENT_NAME), Some(String::from(SECTION_NAME)))) {
                            writeln!(stdout(), "Spayloads of ({}, {}) section", sect.segname, sect.sectname)?;

                            ctxt.cur.seek(SeekFrom::Start(sect.offset as u64))?;

                            let dump = ctxt.section_hex(sect.addr, sect.size)?;

                            let mut output_file = fs::File::create(self.xar_file.as_str()).unwrap();
                            output_file.write(&dump[..])?;

                            let mut process = Command::new("xar")
                                .arg("--dump-toc=".to_owned() + self.xml_file.as_str())
                                .arg("-f")
                                .arg(self.xar_file.as_str())
                                .stdout(Stdio::piped())
                                .stdin(Stdio::piped())
                                .spawn()
                                .expect("xar failed.");
                            let _result = process.wait().unwrap();
                            if _result.code().unwrap() == 0 {
                                let xml_file = fs::File::open(self.xml_file.as_str())?;
                                let xml_file = BufReader::new(xml_file);

                                let mut parse = EventReader::new(xml_file);

                                loop {
                                    let node = parse.next().unwrap();
                                    match node {
                                        XmlEvent::EndDocument => {
                                            break;
                                        },
                                        XmlEvent::StartDocument { .. } => {
                                            println!("start to parse xml");
                                        }
                                        XmlEvent::StartElement { name, .. } => {
                                            println!("xml node start: {}", name);
                                            match name.local_name.as_str() {
                                                "option" => {
                                                    loop {
                                                        match parse.next().unwrap() {
                                                            XmlEvent::Characters( data ) => {
                                                                println!("character option: {}", data);
                                                                self.re_info.push_option(data);
                                                            },
                                                            XmlEvent::EndElement { name } => {
                                                                println!("xml node end: {}", name);
                                                                break;
                                                            },
                                                            _ => {}
                                                        }
                                                    }
                                                },
                                                "lib" => {
                                                    loop {
                                                        match parse.next().unwrap() {
                                                            XmlEvent::Characters( data ) => {
                                                                println!("character framework: {}", data);
                                                                self.re_info.push_framework(&mut data.clone());
                                                            },
                                                            XmlEvent::EndElement { name } => {
                                                                println!("xml node end: {}", name);
                                                                break;
                                                            },
                                                            _ => {}
                                                        }
                                                    }
                                                },
                                                "name" => {
                                                    loop {
                                                        match parse.next().unwrap() {
                                                            XmlEvent::Characters( data ) => {
                                                                println!("character file name: {}", data);
                                                                self.re_info.add_new_file_vec(data);
                                                            },
                                                            XmlEvent::EndElement { name } => {
                                                                println!("xml node end: {}", name);
                                                                break;
                                                            },
                                                            _ => {}
                                                        }
                                                    }
                                                },
                                                "cmd" => {
                                                    loop {
                                                        match parse.next().unwrap() {
                                                            XmlEvent::Characters( data ) => {
                                                                println!("character file cmd: {}", data);
                                                                self.re_info.add_file_cmd(data);
                                                            },
                                                            XmlEvent::EndElement { name } => {
                                                                println!("xml node end: {}", name);
                                                                break;
                                                            },
                                                            _ => {}
                                                        }
                                                    }
                                                }
                                                _ => { }
                                            }
                                        },
                                        XmlEvent::EndElement { name } => {
                                            println!("xml node end: {}", name);
                                        },
                                        _ => {},
                                    }
                                }

                                for index in 0..self.re_info.file_compile.len() {
                                    let mut out = self.re_info.file_compile[index][0].clone();
                                    out.push_str(".o");
                                    self.re_info.obj_file.push(out.clone());
                                    let mut list = vec!["-x".to_string(), "ir".to_string(), "-o".to_string(), out];
                                    self.re_info.file_compile[index].append(&mut list);
                                }
                                println!("link framework: {:?}", self.re_info.link_framework);
                                println!("link option: {:?}", self.re_info.link_options);
                                println!("file cmd: {:?}", self.re_info.file_compile);

                            } else {
                                writeln!(stderr(), "xar dump xml failed.")?;
                            }

                            let mut process = Command::new("xar")
                                .arg("-xf")
                                .arg(self.xar_file.as_str())
                                .stdout(Stdio::piped())
                                .stdin(Stdio::piped())
                                .spawn()
                                .expect("xar failed.");

                            let _result = process.wait().unwrap();
                            if _result.code().unwrap() == 0 {
                                for v in &self.re_info.file_compile {
                                    let mut process_compile = Command::new("clang").arg("-cc1").args(v).spawn().unwrap();
                                    if process.wait().unwrap().code().unwrap() != 0 {
                                        writeln!(stderr(), "file compiled failed.")?;
                                        exit(-1);
                                    } else {
                                        println!("file {} compiled...", v[0]);
                                    }
                                }
                            } else {
                                writeln!(stderr(), "xar extracts failed.")?;
                            }
                            println!("compile finish.");
                            break 'outer;
                        }
                    }
                }

                _ => {}
            }
        }
        Ok(())
    }
}