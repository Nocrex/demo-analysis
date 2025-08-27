use std::fs::{self, File};
use std::io::Write;

use anyhow::Error;
use tf_demo_parser::demo::data::DemoTick;
use tf_demo_parser::demo::message::Message;
use tf_demo_parser::demo::sendprop::SendPropIdentifier;
use tf_demo_parser::{MessageType, ParserState};

use crate::base::cheat_analyser_base::CheatAnalyserState;
use crate::{CheatAlgorithm, Detection};

// header is not needed for this algorithm, but is included to serve as an example of how to handle the lifetimes.
#[allow(dead_code)]
pub struct AllMessages {
    msg_history: Vec<String>,
    file: Option<File>,
    first_write: bool,
}

impl AllMessages {
    const MAX_MSGS_IN_MEMORY: usize = 2048;

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

    pub fn new() -> AllMessages {
        AllMessages {
            msg_history: Vec::new(),
            file: None,
            first_write: true,
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
        self.init_file("./test/all_messages.txt");
        Ok(())
    }

    fn on_message(
        &mut self,
        message: &Message,
        _: &CheatAnalyserState,
        pstate: &ParserState,
        _: DemoTick,
    ) -> Result<Vec<Detection>, Error> {
        let mut message = format!(
            "{:?} {}",
            message.get_message_type(),
            serde_json::to_string_pretty(&serde_json::to_value(message).unwrap()).unwrap()
        );
        if message.starts_with("PacketEntities") {
            let m = message.clone();
            let mut parts: Vec<String> = vec![];
            for part in m.split("\n") {
                if part.contains("\"identifier\"") {
                    let quotes: Vec<(usize, &str)> = part.match_indices('\"').collect();
                    let id = &part[quotes[quotes.len() - 2].0 + 1..quotes[quotes.len() - 1].0];
                    if let Some(names) = SendPropIdentifier::from_const(id.parse().unwrap()).names()
                    {
                        parts.push(part.replace(id, &format!("{}::{}", names.0, names.1)));
                    }else{
                        parts.push(part.to_string());
                    }
                } else if part.contains("\"server_class\"") {
                    let clsid = &part[part.find(':').unwrap() + 2..part.find(',').unwrap()];
                    if let Some(class) = pstate.server_classes.get(clsid.parse::<usize>().unwrap())
                    {
                        parts.push(part.replace(clsid, &class.name));
                    }else{
                        parts.push(part.to_string());
                    }
                } else {
                    parts.push(part.to_string());
                }
            }
            message = parts.join("\n");
        }
        self.msg_history.push(message);

        if self.msg_history.len() > AllMessages::MAX_MSGS_IN_MEMORY {
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
}
