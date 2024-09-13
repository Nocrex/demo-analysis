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
    write_to_file::WriteToFile
};

use tf_demo_parser::demo::{header::Header, parser::gamestateanalyser::GameStateAnalyser};
pub use tf_demo_parser::{Demo, DemoParser, Parse, ParseError, ParserState, Stream};

pub static SILENT: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
pub static COUNT: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

fn main() -> Result<(), Error> {
    let start = std::time::Instant::now();

    let mut silent = false;
    let mut path = String::new();
    let args: Vec<String> = env::args().skip(1).collect();

    // List of all algorithms that can be executed.
    // Each algorithm can be individually invoked with -a <name>
    // If the associated bool is initialised to true, then the algorithm will run if no argument is passed to -a.
    // Any dev stuff and anything that modifies files should NOT run by default.
    let default_algorithm_strings: std::collections::HashMap<String, bool> = std::collections::HashMap::from([
        ("viewangles_180degrees".to_string(), true),
        ("viewangles_to_csv".to_string(), false),
        ("write_to_file".to_string(), false),
    ]);
    let mut algorithm_strings = default_algorithm_strings.clone();

    let mut i = 0;
    while i < args.len() {
        let arg = &args.clone()[i];
        match arg.as_str() {
            "-i" => {
                if i + 1 >= args.len() {
                    panic!("No path specified after -i");
                }
                path = args[i + 1].clone();
                i += 1;
                if path.starts_with('-') {
                    panic!("No path specified after -i");
                }
            },
            "-q" => silent = true,
            "-a" => {
                algorithm_strings.clear();
                let mut reached_a = false;
                for algorithm in args.clone() {
                    if !reached_a {
                        if algorithm.starts_with("-a") {
                            reached_a = true;
                        }
                        continue;
                    } else if algorithm.starts_with("-") {
                        break;
                    } else if !default_algorithm_strings.contains_key(&algorithm) {
                        panic!("Invalid algorithm specified: {}", algorithm);
                    }
                    algorithm_strings.insert(algorithm, true);
                    i += 1;
                }
                if algorithm_strings.values().all(|v| !*v) {
                    panic!("No algorithms specified");
                }
            },
            "-c" => {
                COUNT.store(true, std::sync::atomic::Ordering::SeqCst);
            },
            "-h" => {
                println!("-i <path> (required) - specify the demo file to analyze");
                println!("-q - silence all output except for the final JSON string");
                println!("-c - only print the number of detections");
                println!("-a [list of algorithms to run] - specify the algorithms to run. If not specified, the default algorithms are run.");
                println!("Default algorithms:");
                for (key, value) in default_algorithm_strings.iter() {
                    if *value {
                        println!("  {}", key);
                    }
                }
                println!("Other algorithms:");
                for (key, value) in default_algorithm_strings.iter() {
                    if !*value {
                        println!("  {}", key);
                    }
                }
                return Ok(());
            },
            _ => panic!("Unknown argument: {}", arg),
        }
        i += 1;
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
    event_instances.insert("write_to_file", Box::new(WriteToFile::new(&header)));

    let events: Vec<Box<dyn DemoTickEvent>> = event_instances
        .into_iter()
        .filter_map(|(name, event)| {
            if algorithm_strings.contains_key(&name.to_string()) {
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

pub trait DemoTickEvent<'a> {
    fn algorithm_name(&self) -> &str {
        panic!("algorithm_name() not implemented for {}", std::any::type_name::<Self>());
    }

    // Called before any other events
    // Use this instead of ::new() when performing any non-ephemeral actions e.g. modifying files
    fn init(&mut self) -> Result<Vec<Detection>, Error> {
        Ok(vec![])
    }

    // Called for each tick. Contains the json state for the tick
    // Try the write_to_file algorithm to see what those states look like (there is one state per line)
    // cargo run -- -i demo.dem -a write_to_file
    fn on_tick(&mut self, _tick: Value) -> Result<Vec<Detection>, Error> {
        Ok(vec![])
    }

    // Called after all other events
    // Use for cleaning up or for aggregate analysis
    fn finish(&mut self) -> Result<Vec<Detection>, Error> {
        Ok(vec![])
    }
}

#[derive(Serialize, Deserialize)]
pub struct Detection {
    pub tick: u64,
    pub algorithm: String,
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
