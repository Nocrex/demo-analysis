use std::fs::{self, File};
use std::io::Write;

use anyhow::Error;
use serde_json::Value;
use tf_demo_parser::demo::header::Header;

use crate::{DemoTickEvent, Detection};

// header is not needed for this algorithm, but is included to serve as an example of how to handle the lifetimes.
#[allow(dead_code)]
pub struct DemoAnalysisFileWriter<'a> {
    state_history: Vec<Value>,
    file: Option<File>,
    first_write: bool,
    header: &'a Header
}

impl<'a> DemoAnalysisFileWriter<'a> {
    const MAX_STATES_IN_MEMORY: usize = 1024;

    fn write_states_to_file(&mut self) {

        if self.first_write {
            self.first_write = false;
        } else {
            writeln!(self.file.as_mut().unwrap(), ",").unwrap();
        }

        let out = self.state_history.iter()
            .map(|j| serde_json::to_string(&j).unwrap())
            .collect::<Vec<String>>().join(",\n"); 
    
        write!(self.file.as_mut().unwrap(), "{}", out).unwrap();
    }

    pub fn init_file(&mut self, file_path: &str) {
        self.file = Some(match fs::File::create(file_path) {
            Ok(file) => file,
            Err(err) => {
                if err.kind() != std::io::ErrorKind::AlreadyExists {
                    panic!("Error creating file: {}", err);
                }
                fs::remove_file(file_path).unwrap();
                fs::File::create(file_path).unwrap()
            }
        });
    }

    pub fn new (header: &'a Header) -> DemoAnalysisFileWriter<'a> {
        DemoAnalysisFileWriter {
            state_history: Vec::new(),
            file: None,
            first_write: true,
            header
        }
    }
}

impl<'a> DemoTickEvent for DemoAnalysisFileWriter<'a> {

    
    fn init(&mut self) -> Result<Vec<Detection>, Error> {
        self.init_file("./test/write_to_file.json");

        writeln!(self.file.as_mut().unwrap(), "[").unwrap();

        Ok(vec![])
    }
    
    fn on_tick(&mut self, state: Value) -> Result<Vec<Detection>, Error> {
        self.state_history.push(state);
    
        if self.state_history.len() > DemoAnalysisFileWriter::MAX_STATES_IN_MEMORY {
            self.write_states_to_file();
    
            self.state_history.clear();
        }

        Ok(vec![])
    }

    fn finish(&mut self) -> Result<Vec<Detection>, Error> {

        if self.state_history.len() > 0 {
            self.write_states_to_file();
            self.state_history.clear();
        }

        writeln!(self.file.as_mut().unwrap(), "\n]").unwrap();
        let _ = self.file.as_mut().unwrap().flush();

        Ok(vec![])
    }
}



