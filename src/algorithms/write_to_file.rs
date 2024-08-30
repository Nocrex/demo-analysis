use std::fs;
use std::io::Write;

use anyhow::Error;
use serde_json::Value;
use tf_demo_parser::demo::header::Header;

use crate::DemoTickEvent;

// Implement the DemoTickEvent trait. This is where the bulk of your algorithm resides.
// Return values:
// Some(Value) - for any interesting results. Should include a tick number and/or other relevant information necessary for analysis.
// None - No results of interest.
// Error - If something breaks.

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

// Specify algorithm specific variables here.
pub struct DemoAnalysisFileWriter<'a> {
    state_history: Vec<Value>,
    file: fs::File,
    header: &'a Header
}

// At minimum, implement a pub fn new.
// Use the new() function to initalize any variables specified in the struct
// Additional helper functions and consts also go here

impl<'a> DemoAnalysisFileWriter<'a> {
    const MAX_STATES_IN_MEMORY: usize = 1024;

    fn write_states_to_file(&mut self) {
        let out = self.state_history.iter()
                    .map(|j| serde_json::to_string(&j).unwrap())
                    .collect::<Vec<String>>().join(",\n") + ",\n";
    
        writeln!(self.file, "{}", out).unwrap();
    }

    pub fn new (header: &'a Header) -> DemoAnalysisFileWriter<'a> {
        let mut out = DemoAnalysisFileWriter {
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
        };
        writeln!(out.file, "[").unwrap();
        return out;
    }
}



