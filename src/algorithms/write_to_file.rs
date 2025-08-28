use std::collections::HashMap;
use std::fs::{self, File};
use std::io::Write;

use anyhow::Error;
use tf_demo_parser::ParserState;

use crate::base::cheat_analyser_base::CheatAnalyserState;
use crate::util::helpers::get_parameter_value;
use crate::{CheatAlgorithm, Detection, Parameter, Parameters};

#[allow(dead_code)]
pub struct WriteToFile {
    state_history: Vec<CheatAnalyserState>,
    file: Option<File>,
    first_write: bool,
    params: Parameters,
}

impl WriteToFile {

    fn write_states_to_file(&mut self) {
        if self.first_write {
            self.first_write = false;
        } else {
            writeln!(self.file.as_mut().unwrap(), ",").unwrap();
        }

        let out = self.state_history.iter()
            .map(|j| {
                serde_json::to_string(&j).unwrap()
            })
            .collect::<Vec<String>>().join(",\n"); 
    
        write!(self.file.as_mut().unwrap(), "{}", out).unwrap();
        self.state_history.clear();
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

    pub fn new () -> WriteToFile {
        let params = HashMap::from([
            ("write_batch_size".to_string(), Parameter::Int(1024))
        ]);
        let write_batch_size: i32 = get_parameter_value(&params, "write_batch_size");
        WriteToFile {
            state_history: Vec::with_capacity(write_batch_size as usize),
            file: None,
            first_write: true,
            params
        }
    }
}

// lifetimes not needed for this algorithm, but is included to serve as an example of how to handle them.
impl CheatAlgorithm<'_> for WriteToFile {
    fn default(&self) -> bool {
        false
    }

    fn algorithm_name(&self) -> &str {
        "write_to_file"
    }

    fn init(&mut self) -> Result<(), Error> {
        self.init_file("./test/write_to_file.json");

        writeln!(self.file.as_mut().unwrap(), "[").unwrap();

        Ok(())
    }
    
    fn on_tick(&mut self, state: &CheatAnalyserState, _: &ParserState) -> Result<Vec<Detection>, Error> {
        self.state_history.push(state.clone());
        let write_batch_size: i32 = get_parameter_value(&self.params, "write_batch_size");
        if self.state_history.len() > write_batch_size as usize {
            self.write_states_to_file();
        }

        Ok(vec![])
    }

    fn finish(&mut self) -> Result<Vec<Detection>, Error> {
        if self.state_history.len() > 0 {
            self.write_states_to_file();
        }

        writeln!(self.file.as_mut().unwrap(), "\n]").unwrap();
        let _ = self.file.as_mut().unwrap().flush();

        Ok(vec![])
    }

    fn params(&mut self) -> Option<&mut Parameters> {
        Some(&mut self.params)
    }
}



