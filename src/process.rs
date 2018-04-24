extern crate mach_object;
extern crate failure;
extern crate xml;

use mach_object::*;
use self::failure::Error;
use self::xml::reader::{EventReader, XmlEvent};

use std::process::{Command, Stdio, exit};
use std::fs;
use std::io::{Write, BufReader, stderr, stdout, SeekFrom, Seek, Read};

use context::*;

const SEGMENT_NAME: &'static str = "__LLVM";
const SECTION_NAME: &'static str = "__bundle";

pub struct MachOProcess {
    re_info: ReCompilerInfo,
    xml_file: String,
    xar_file: String,
}

impl MachOProcess {
    pub fn new() -> MachOProcess {
        MachOProcess {
            re_info: ReCompilerInfo::new(),
            xml_file: String::from("tmp.xml"),
            xar_file: String::from("tmp.xar"),
        }
    }

    pub fn handle_ofile(&mut self, ofile: &OFile, ctxt: &mut FileContext, sdk_path: Option<String>, tool_chain: Option<String>) -> Result<bool, Error> {

        match ofile {
            &OFile::MachFile {
                ref header,
                ref commands,
            } => {
                if self.handle_macho_file(header, commands, ctxt).unwrap() == true {
                    if self.parse_xml(sdk_path, tool_chain).unwrap() == false {
                        writeln!(stderr(), "parse xml failed.")?;
                        return Ok(false);
                    }
                    if self.compiler_files().unwrap() == false {
                        writeln!(stderr(), "compile file failed.")?;
                        return Ok(false);
                    }
                    if self.link_objects().unwrap() == false {
                        writeln!(stderr(), "link error.")?;
                        return Ok(false);
                    }
                }
                return Ok(true);
            },
            &OFile::FatFile { magic, ref files } => {
                writeln!(stderr(), "No support");
                Ok(false)
            },
            &OFile::ArFile { ref files } => {
                writeln!(stderr(), "No support");
                Ok(false)
            },
            &OFile::SymDef { ref ranlibs } => {
                writeln!(stderr(), "No support");
                Ok(false)
            },
        }
    }

    fn parse_element<T>(&mut self, parse: &mut EventReader<T>, local_name: &String) -> Result<bool, Error> where T: Read {

        match local_name.as_str() {
            "platform" => {
                loop {
                    match parse.next().unwrap() {
                        XmlEvent::Characters( data ) => {
                            self.re_info.set_platform(data.clone());
                        },
                        XmlEvent::EndElement { name } => {
                            break;
                        },
                        _ => {}
                    }
                };
            },
            "option" => {
                loop {
                    match parse.next().unwrap() {
                        XmlEvent::Characters( data ) => {
                            //println!("character option: {}", data);
                            self.re_info.push_option(data);
                        },
                        XmlEvent::EndElement { name } => {
                            //println!("xml node end: {}", name);
                            break;
                        },
                        _ => {}
                    };
                };
            },
            "lib" => {
                loop {
                    match parse.next().unwrap() {
                        XmlEvent::Characters( data ) => {
                            //println!("character framework: {}", data);
                            self.re_info.push_framework(&mut data.clone());
                        },
                        XmlEvent::EndElement { name } => {
                            //println!("xml node end: {}", name);
                            break;
                        },
                        _ => {}
                    };
                };
            },
            "name" => {
                loop {
                    match parse.next().unwrap() {
                        XmlEvent::Characters( data ) => {
                            //println!("character file name: {}", data);
                            self.re_info.add_new_file_vec(data);
                        },
                        XmlEvent::EndElement { name } => {
                            //println!("xml node end: {}", name);
                            break;
                        },
                        _ => {}
                    };
                };
            },
            "cmd" => {
                loop {
                    match parse.next().unwrap() {
                        XmlEvent::Characters( data ) => {
                            //println!("character file cmd: {}", data);
                            self.re_info.add_file_cmd(data);
                        },
                        XmlEvent::EndElement { name } => {
                            //println!("xml node end: {}", name);
                            break;
                        },
                        _ => {}
                    };
                };
            }
            _ => { }
        }
        return Ok(true);
    }

    fn parse_xml(&mut self, sdk_path: Option<String>, tool_chain: Option<String>) -> Result<bool, Error> {

        let xml_file = fs::File::open(self.xml_file.as_str()).expect("tmp xml file open failed.");
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
                    //println!("xml node start: {}", name);
                    if self.parse_element::<BufReader<fs::File>>(&mut parse, &name.local_name).unwrap() == false {
                        return Ok(false);
                    }
                },
                XmlEvent::EndElement { name } => {
                    //println!("xml node end: {}", name);
                },
                _ => {},
            }
        }

        if self.re_info.choose_path_from_platform(sdk_path, tool_chain).unwrap() == false {
            return Ok(false);
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
        return Ok(true);
    }

    fn compiler_files(&mut self) -> Result<bool, Error> {

        let mut process = Command::new("xar")
            .arg("-xf")
            .arg(self.xar_file.as_str())
            .stdout(Stdio::piped())
            .stdin(Stdio::piped())
            .spawn()
            .expect("xar error.");

        let _result = process.wait().unwrap();
        if _result.code().unwrap() == 0 {
            for v in &self.re_info.file_compile {
                let mut process_compile = Command::new("clang").arg("-cc1").args(v).spawn().unwrap();
                if process.wait().unwrap().code().unwrap() != 0 {
                    writeln!(stderr(), "file compiled failed.")?;
                    return Ok(false);
                } else {
                    println!("file {} compiled...", v[0]);
                }
            }
        } else {
            writeln!(stderr(), "xar extracts failed.")?;
            return Ok(false);
        }
        return Ok(true);
    }

    fn handle_macho_file(&mut self, header: &MachHeader, commands: &[MachCommand], ctxt: &mut FileContext) -> Result<bool, Error> {
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
                                writeln!(stdout(), "xar dump xml finish.")?;
                                return Ok(true);
                            } else {
                                writeln!(stderr(), "xar dump xml failed.")?;
                                return Ok(false);
                            }
                            break 'outer;
                        }
                    }
                }
                _ => {}
            }
        }
        return Ok(false);
    }

    fn link_objects(&mut self) -> Result<bool, Error> {

        let mut link_options = Vec::new();

        let mut out_path = false;
        for opt in &mut self.re_info.link_options {
            if out_path {
                let mut output = String::new();
                output.push_str("./");
                let off = match opt.rfind("/") {
                    Some(index) => {
                        index + 1
                    }
                    None => 0
                };
                output.push_str(opt.split_off(off).as_str());
                link_options.push(output);
                out_path = false;
            } else {
                link_options.push(opt.clone());
            }
            if opt.as_str() == "-executable_path" {
                out_path = true;
            }
        }
        link_options.push("-syslibroot".to_string());
        link_options.push(self.re_info.sdk_path.clone());
        for framework in &self.re_info.link_framework {
            link_options.push("-framework".to_string());
            link_options.push(framework.clone());
        }

        let mut search = Command::new("find")
            .arg(self.re_info.tool_chain.clone())
            .arg("-name")
            .arg(self.re_info.lib_clang.clone())
            .output()
            .expect("find libclang_rt error.");
        if search.status.success() {
            if let Ok(mut lib_clang)= String::from_utf8(search.stdout) {
                //lib_clang.retain(|c| c != '\n');
                lib_clang.pop();
                link_options.push("-lSystem".to_string());
                link_options.push(lib_clang);
            } else {
                writeln!(stderr(), "find libclang_rt error.");
                return Ok(false);
            }
        } else {
            //println!("{}, {}, {}",self.re_info.sdk_path, self.re_info.tool_chain, self.re_info.lib_clang);
            writeln!(stderr(), "find libclang_rt failed.");
            return Ok(false);
        }

        for obj in &self.re_info.obj_file {
            link_options.push(obj.clone());
        }

        println!("link_options: {:?}", link_options);

        let mut link_process = Command::new("ld").args(link_options).spawn().unwrap();
        let _result = link_process.wait().unwrap();
        if _result.code().unwrap() == 0 {
            println!("success!");
        } else {
            writeln!(stderr(), "ld error!");
            return Ok(false);
        }

        return Ok(true);
    }
}