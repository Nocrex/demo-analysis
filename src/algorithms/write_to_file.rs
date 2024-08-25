use std::fs;
use std::io::Write;

use anyhow::Error;
use serde_json::Value;
use tf_demo_parser::demo::header::{self, Header};

use crate::DemoTickEvent;

impl<'a> DemoTickEvent for DemoAnalysisFileWriter<'a> {
    
    fn on_tick(&mut self, state: Value) -> Result<Option<Value>, Error> {
        self.state_history.push(state);
    
        if self.state_history.len() > DemoAnalysisFileWriter::MAX_STATES_IN_MEMORY {
            self.write_states_to_file();
    
            self.state_history.clear();
        }

        Ok(None)
    }

    fn finish(&mut self) -> Result<Option<Value>, Error> {

        if self.state_history.len() > 0 {
            self.write_states_to_file();
            self.state_history.clear();
        }

        writeln!(self.file, "]").unwrap();
        let _ = self.file.flush();

        Ok(None)
    }
}

pub struct DemoAnalysisFileWriter<'a> {
    state_history: Vec<Value>,
    file: fs::File,
    header: &'a Header
}

impl<'a> DemoAnalysisFileWriter<'a> {
    const MAX_STATES_IN_MEMORY: usize = 1024;

    fn write_states_to_file(&mut self) {
        let out = self.state_history.iter()
                    .map(|j| serde_json::to_string(&j).unwrap())
                    .collect::<Vec<String>>().join(",\n") + ",\n";
    
        writeln!(self.file, "{}", out).unwrap();
    }

    pub fn new (header: &'a Header) -> DemoAnalysisFileWriter<'a> {
        DemoAnalysisFileWriter {
            state_history: Vec::new(),
            file: match fs::File::create("./test/test.json") {
                Ok(file) => file,
                Err(err) => {
                    if err.kind() != std::io::ErrorKind::AlreadyExists {
                        panic!("Error creating file: {}", err);
                    }
                    fs::remove_file("./test/test.json").unwrap();
                    fs::File::create("./test/test.json").unwrap()
                }
            },
            header,
        }
    }
}



