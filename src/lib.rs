// Import algorithm file here.
pub mod algorithms {
    pub mod all_messages;
    pub mod viewangles_180degrees;
    pub mod viewangles_to_csv;
    pub mod write_to_file;

    pub mod aim_snap;
    pub mod angle_repeat;
    pub mod oob_pitch;

    pub mod util {
        pub mod jankguard;
    }
}
use std::collections::HashMap;

//
// Import algorithm struct here.
pub use algorithms::{
    all_messages::AllMessages,
    viewangles_180degrees::ViewAngles180Degrees,
    viewangles_to_csv::ViewAnglesToCSV,
    write_to_file::WriteToFile,

    aim_snap::AimSnap, 
    angle_repeat::AngleRepeat, 
    oob_pitch::OOBPitch,
};

pub fn algorithms() -> Vec<Box<dyn CheatAlgorithm<'static>>> {
    vec![
        Box::new(AllMessages::new()),
        Box::new(ViewAngles180Degrees::new()),
        Box::new(ViewAnglesToCSV::new()),
        Box::new(WriteToFile::new()),
        Box::new(OOBPitch::new()),
        Box::new(AngleRepeat::new()),
        Box::new(AimSnap::new()),
    ]
}

pub mod base {
    pub mod cheat_analyser_base;
    pub mod demo_handler_base;
}

pub mod util;
use anyhow::Error;
use base::{cheat_analyser_base::CheatAnalyserState};
use bitbuffer::BitRead;
use serde_json::Value;
use serde::{Deserialize, Serialize};

use tf_demo_parser::{demo::{data::DemoTick, header::Header, message::Message, parser::RawPacketStream}, MessageType};

pub use tf_demo_parser::{Demo, DemoParser, Parse, ParseError, ParserState, Stream};

use crate::base::{cheat_analyser_base::CheatAnalyser, demo_handler_base::CheatDemoHandler};

pub fn analyse<'a>(demo: &Demo, algorithms: Vec<Box<dyn CheatAlgorithm<'a>>>) -> anyhow::Result<CheatAnalyser<'a>> {
    let mut stream = demo.get_stream();
    let header: Header = Header::read(&mut stream)?;
    let mut packets = RawPacketStream::new(stream);

    let analyser = CheatAnalyser::new(algorithms);
    let mut handler = CheatDemoHandler::with_analyser(analyser);

    handler.handle_header(&header);
    let _ = handler.analyser.init();
    loop {
        let packet = packets.next(&handler.state_handler);
        let packet = match packet {
            Ok(packet) => match packet {
                Some(packet) => packet,
                None => break,
            },
            Err(e) => {
                dev_print!("ParseError: {}", e);
                continue;
            }
        };
        let _ = handler.handle_packet(packet)?;
    }
    let _ = handler.analyser.finish()?;
    Ok(handler.analyser)
}

pub static SILENT: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

pub trait CheatAlgorithm<'a> {
    fn default(&self) -> bool {
        panic!("default() not set for {}", std::any::type_name::<Self>());
    }

    fn algorithm_name(&self) -> &str {
        panic!("algorithm_name() not implemented for {}", std::any::type_name::<Self>());
    }

    fn does_handle(&self, message_type: MessageType) -> bool {
        match self.handled_messages() {
            Ok(types) => types.contains(&message_type),
            Err(parse_all) => parse_all,
        }
    }

    // Called before any other events
    // Use this instead of ::new() when performing any non-ephemeral actions e.g. modifying files
    fn init(&mut self) -> Result<(), Error> {
        Ok(())
    }

    // Called for each tick. Passes the basic game state for the tick
    // Try the write_to_file algorithm to see what those states look like (there is one state per line)
    // cargo run -- -i demo.dem -a write_to_file
    fn on_tick(&mut self, _state: &CheatAnalyserState, _parser_state: &ParserState) -> Result<Vec<Detection>, Error> {
        Ok(vec![])
    }

    // If your algorithm needs to handle additional message types, return those types in a Vec.
    // You can return Err(true) to accept all messages, or Err(false) to reject all messages.
    fn handled_messages(&self) -> Result<Vec<MessageType>, bool> {
        Err(false)
    }

    // Called for each message received by the parser.
    // Only called for types specified in handled_messages.
    fn on_message(&mut self, _message: &Message, _state: &CheatAnalyserState, _parser_state: &ParserState, _tick: DemoTick) -> Result<Vec<Detection>, Error> {
        Ok(vec![])
    }

    // Called after all other events
    // Use for cleaning up or for aggregate analysis
    fn finish(&mut self) -> Result<Vec<Detection>, Error> {
        Ok(vec![])
    }
    
    fn params(&mut self) -> Option<&mut HashMap<&'static str, f32>>{
        None
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Detection {
    pub tick: u32,
    pub algorithm: String,
    pub player: u64,
    pub data: Value
}

#[macro_export]
macro_rules! dev_print {
    ($($arg:tt)*) => {
        if !crate::SILENT.load(std::sync::atomic::Ordering::Relaxed) {
            println!($($arg)*);
        }
    }
}
