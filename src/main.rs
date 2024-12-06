// Import algorithm file here.
mod algorithms {
    pub mod all_messages;
    pub mod viewangles_180degrees;
    pub mod viewangles_to_csv;
    pub mod write_to_file;
}

mod base {
    pub mod cheat_analyser_base;
    pub mod demo_handler_base;
}

mod util;

use std::{env, fs::{self}};
use anyhow::Error;
use base::{cheat_analyser_base::CheatAnalyserState, demo_handler_base::CheatDemoHandler};
use bitbuffer::BitRead;
use serde_json::Value;
use serde::{Deserialize, Serialize};

// Import algorithm struct here.
use algorithms::{
    all_messages::AllMessages,
    viewangles_180degrees::ViewAngles180Degrees,
    viewangles_to_csv::ViewAnglesToCSV,
    write_to_file::WriteToFile
};
use tf_demo_parser::{demo::{data::DemoTick, header::Header, message::Message, parser::RawPacketStream}, MessageType};

use crate::base::cheat_analyser_base::CheatAnalyser;
pub use tf_demo_parser::{Demo, DemoParser, Parse, ParseError, ParserState, Stream};

pub static SILENT: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

use getopts::Options;

fn main() -> Result<(), Error> {
    let start = std::time::Instant::now();

    let mut opts = Options::new();
    opts.optopt("i", "input", "set input file path", "PATH");
    opts.optflag("q", "quiet", "silence all output except for the final JSON string");
    opts.optflag("p", "pretty", "same as -q, but with more human-readable json");
    opts.optmulti("a", "algorithm", "specify the algorithm to run. Include multiple -a flags to run multiple algorithms. If not specified, the default algorithms are run.", "ALGORITHM [-a ALGORITHM]...");
    opts.optflag("c", "count", "only print the number of detections");
    opts.optflag("h", "help", "print this help menu");

    fn print_help(opts: &getopts::Options) {
        println!("{}", opts.usage("Usage: analysis-template [options]"));
    }

    let matches = match opts.parse(env::args().skip(1)) {
        Ok(m) => m,
        Err(_) => {
            print_help(&opts);
            return Ok(());
        }
    };

    if matches.opt_present("h") {
        print_help(&opts);
        return Ok(());
    }

    let path = matches.opt_str("i").expect("No input file path provided");
    let silent = matches.opt_present("q") || matches.opt_present("p");
    let pretty = matches.opt_present("p");
    SILENT.store(silent, std::sync::atomic::Ordering::SeqCst);

    // To add your algorithm, call new() on it and store inside a Box.
    // You will need to import it at the top of the file.
    let mut algorithms: Vec<Box<dyn CheatAlgorithm>> = vec![
        Box::new(AllMessages::new()),
        Box::new(ViewAngles180Degrees::new()),
        Box::new(ViewAnglesToCSV::new()),
        Box::new(WriteToFile::new()),
    ];

    let specified_algorithms = matches.opt_strs("a");
    if specified_algorithms.is_empty() && !matches.opt_present("a") {
        algorithms.retain(|a| a.default());
    } else {
        algorithms.retain(|a| specified_algorithms.contains(&a.algorithm_name().to_string()));
    }

    let unknown_algorithms: Vec<String> = specified_algorithms
        .into_iter()
        .filter(|a| algorithms.iter().all(|b| b.algorithm_name() != *a))
        .collect();
    if !unknown_algorithms.is_empty() {
        panic!("Unknown algorithms specified: {}", unknown_algorithms.join(", "));
    } else if algorithms.is_empty() {
        panic!("No algorithms specified");
    }

    let file = fs::read(path)?;
    let demo: Demo = Demo::new(&file);
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

    if start.elapsed().as_secs() >= 10 {
        handler.analyser.print_metadata();
    }

    if SILENT.load(std::sync::atomic::Ordering::Relaxed) {
        handler.analyser.print_detection_json(pretty);
    } else if matches.opt_present("c") {
        handler.analyser.print_detection_summary();
    } else {
        handler.analyser.print_detection_json(true);
    }

    let total_ticks = handler.analyser.get_tick_count_u32();
    let total_time = start.elapsed().as_secs_f64();
    let total_tps = (total_ticks as f64) / total_time;
    dev_print!("Done! (Processed {} ticks in {:.2} seconds averaging {:.2} tps)", total_ticks, total_time, total_tps);

    Ok(())
}


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
        if !crate::SILENT.load(std::sync::atomic::Ordering::SeqCst) {
            println!($($arg)*);
        }
    }
}
