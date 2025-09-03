use std::collections::HashMap;
use std::fs::{self, File};
use std::io::Write;

use anyhow::Error;
use tf_demo_parser::demo::data::DemoTick;
use tf_demo_parser::demo::message::Message;
use tf_demo_parser::{MessageType, ParserState};

use crate::base::cheat_analyser_base::CheatAnalyserState;
use crate::dev_print;
use crate::lib::algorithm::{CheatAlgorithm, Detection};
use crate::lib::parameters::{get_parameter_value, Parameter, Parameters};

// header is not needed for this algorithm, but is included to serve as an example of how to handle the lifetimes.
#[allow(dead_code)]
pub struct AllMessages {
    msg_history: Vec<String>,
    file: Option<File>,
    first_write: bool,
    params: Parameters,
}

impl AllMessages {
    fn write_messages_to_file(&mut self) {
        let out = self.msg_history.join("\n"); 
        write!(self.file.as_mut().unwrap(), "{}\n", out).unwrap();
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

    pub fn new () -> AllMessages {
        AllMessages {
            msg_history: Vec::new(),
            file: None,
            first_write: true,
            params: HashMap::from([
                ("write_batch_size".to_string(), Parameter::Int(2048))
            ])
        }
    }
}

impl CheatAlgorithm<'_> for AllMessages {
    fn default(&self) -> bool {
        false
    }

    fn algorithm_name(&self) -> &str {
        "all_messages"
    }

    fn init(&mut self) -> Result<(), Error> {
        dev_print!("Initializing AllMessages algorithm with write_batch_size = {}", get_parameter_value::<i32>(&self.params, "write_batch_size"));
        self.init_file("./output/all_messages.txt");
        Ok(())
    }

    fn on_message(&mut self, message: &Message, _: &CheatAnalyserState, pstate: &ParserState, tick: DemoTick,) -> Result<Vec<Detection>, Error> {
        let mut message = format!("({tick}) {:#?}", message);

        while let Some(start) = message.find("ClassId(") {
            let end = message[start + 9..].find(")").unwrap() + start + 9;
            let id = message[start + 9..end].trim();
            let id = id.strip_suffix(",").unwrap_or(id);
            let id: u16 = id.parse().unwrap();
            let class = pstate
                .server_classes
                .iter()
                .find(|sc| u16::from(sc.id) == id)
                .unwrap();
            message.replace_range(start..=end, class.name.as_str());
        }

        self.msg_history.push(message);
        let write_batch_size: i32 = get_parameter_value(&self.params, "write_batch_size");
    
        if self.msg_history.len() > write_batch_size as usize {
            self.write_messages_to_file();
    
            self.msg_history.clear();
        }

        Ok(vec![])
    }

    fn handled_messages(&self) -> Result<Vec<MessageType>, bool> {
        Err(true)
    }

    fn finish(&mut self) -> Result<Vec<Detection>, Error> {

        if self.msg_history.len() > 0 {
            self.write_messages_to_file();
            self.msg_history.clear();
        }

        let _ = self.file.as_mut().unwrap().flush();

        Ok(vec![])
    }

    fn params(&mut self) -> Option<&mut Parameters> {
        Some(&mut self.params)
    }
}



