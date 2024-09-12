pub mod ticker;
mod algorithms {
    pub mod viewangles_180degrees;
    pub mod viewangles_to_csv;
    pub mod write_to_file;
}

use std::{borrow::BorrowMut, env, fs::{self}};
use anyhow::Error;
use serde_json::Value;
use serde::{Deserialize, Serialize};

use crate::ticker::perform_tick;
use algorithms::{
    viewangles_180degrees::ViewAngles180Degrees, 
    viewangles_to_csv::ViewAnglesToCSV,
    write_to_file::DemoAnalysisFileWriter
};

use tf_demo_parser::demo::{header::Header, parser::gamestateanalyser::GameStateAnalyser};
pub use tf_demo_parser::{Demo, DemoParser, Parse, ParseError, ParserState, Stream};

pub static SILENT: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
pub static COUNT: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

fn main() -> Result<(), Error> {
    let start = std::time::Instant::now();

    let mut silent = false;
    let mut path = String::new();
    let mut args = env::args().skip(1);

    // Algorithms that should run by default go here.
    // Any dev stuff and anything that modifies files should NOT go here.
    let mut algorithm_strings: Vec<String> = vec![
        "viewangles_180degrees".to_string()
    ];

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-i" => {
                path = args.next().expect("Expected input file path after -i");
            },
            "-q" => silent = true,
            "-a" => {
                algorithm_strings.clear();
                while let Some(arg) = args.next() {
                    if arg.starts_with('-') {
                        break;
                    } 
                    algorithm_strings.push(arg.to_string());
                }
                if algorithm_strings.len() == 0 {
                    panic!("No algorithms specified");
                }
            },
            "-c" => {
                COUNT.store(true, std::sync::atomic::Ordering::SeqCst);
            },
            _ => panic!("Unknown argument: {}", arg),
        }
    }
    if path.is_empty() {
        panic!("No input file path provided");
    }
    
    SILENT.store(silent, std::sync::atomic::Ordering::SeqCst);
    let file = fs::read(path)?;
    let demo: Demo = Demo::new(&file);
    let parser = DemoParser::new_with_analyser(demo.get_stream(), GameStateAnalyser::new());
    let ticker = parser.ticker();

    if ticker.is_err() {
        panic!("Error creating demo ticker: {}", ticker.err().unwrap());
    }

    let (header, mut ticker ) = ticker.unwrap();

    let mut event_instances: std::collections::HashMap<&str, Box<dyn DemoTickEvent>> = std::collections::HashMap::new();
    event_instances.insert("viewangles_180degrees", Box::new(ViewAngles180Degrees::new()));
    event_instances.insert("viewangles_to_csv", Box::new(ViewAnglesToCSV::new()));
    event_instances.insert("write_to_file", Box::new(DemoAnalysisFileWriter::new(&header)));

    for algorithm in algorithm_strings.iter() {
        if !event_instances.contains_key(algorithm.as_str()) {
            panic!("Algorithm '{}' is not a valid algorithm", algorithm);
        }
    }

    let events: Vec<Box<dyn DemoTickEvent>> = event_instances
        .into_iter()
        .filter_map(|(name, event)| {
            if algorithm_strings.contains(&name.to_string()) {
                Some(event)
            } else {
                None
            }
        })
        .collect();

    if events.is_empty() {
        panic!("No algorithms specified");
    }

    print_metadata(&header);

    let mut detections = Vec::new();
    detections.extend(perform_tick(&header, ticker.borrow_mut(), events));

    if start.elapsed().as_secs() >= 10 {
        print_metadata(&header);
    }

    let total_ticks = header.ticks;
    let total_time = start.elapsed().as_secs_f64();
    let total_tps = (total_ticks as f64) / total_time;
    dev_print!("Done! (Processed {} ticks in {:.2} seconds averaging {:.2} tps)", total_ticks, total_time, total_tps);

    if SILENT.load(std::sync::atomic::Ordering::Relaxed) {
        println!("{}", serde_json::to_string(&detections).unwrap());
    } else if COUNT.load(std::sync::atomic::Ordering::Relaxed) {
        println!("Detection count: {}", detections.len());
    } else {
        println!("{}", serde_json::to_string_pretty(&detections).unwrap());
    }

    Ok(())
}

pub trait DemoTickEvent {

    // Called before any other events
    // Use this instead of ::new() when performing any non-ephemeral actions e.g. modifying files
    fn init<'a>(&mut self) -> Result<Vec<Detection>, Error> {
        Ok(vec![])
    }

    // Called for each tick. Contains the json state for the tick
    // Try the write_to_file algorithm to see what those states look like (there is one state per line)
    fn on_tick<'a>(&mut self, _tick: Value) -> Result<Vec<Detection>, Error> {
        Ok(vec![])
    }

    // Called after all other events
    // Use for cleaning up or for aggregate analysis
    fn finish<'a>(&mut self) -> Result<Vec<Detection>, Error> {
        Ok(vec![])
    }
}

#[derive(Serialize, Deserialize)]
pub struct Detection {
    pub tick: u64,
    pub player: u64,
    pub data: Value
}

fn print_metadata(header: &Header) {
    dev_print!("Map: {}", header.map);
    let hours = (header.duration / 3600.0).floor();
    let minutes = ((header.duration % 3600.0) / 60.0).floor();
    let seconds = (header.duration % 60.0).floor();
    let milliseconds = ((header.duration % 1.0) * 100.0).floor();
    dev_print!("Duration: {:02}:{:02}:{:02}.{:03} ({} ticks)", hours, minutes, seconds, milliseconds, header.ticks);
    dev_print!("User: {}", header.nick);
    dev_print!("Server: {}", header.server);
}

#[macro_export]
macro_rules! dev_print {
    ($($arg:tt)*) => {
        if !crate::SILENT.load(std::sync::atomic::Ordering::SeqCst) {
            println!($($arg)*);
        }
    }
}
