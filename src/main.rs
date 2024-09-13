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

use getopts::Options;

fn main() -> Result<(), Error> {
    let start = std::time::Instant::now();

    let mut opts = Options::new();
    opts.optopt("i", "input", "set input file path", "PATH");
    opts.optflag("q", "quiet", "silence all output except for the final JSON string");
    opts.optopt("a", "algorithms", "specify the algorithms to run. If not specified, the default algorithms are run.", "ALGORITHMS");
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
    let silent = matches.opt_present("q");
    SILENT.store(silent, std::sync::atomic::Ordering::SeqCst);

    let file = fs::read(path)?;
    let demo: Demo = Demo::new(&file);
    let parser = DemoParser::new_with_analyser(demo.get_stream(), GameStateAnalyser::new());
    let ticker = parser.ticker();

    if ticker.is_err() {
        panic!("Error creating demo ticker: {}", ticker.err().unwrap());
    }

    let (header, mut ticker ) = ticker.unwrap();

    let mut algorithms: Vec<Box<dyn DemoTickEvent>> = vec![
        Box::new(ViewAngles180Degrees::new()),
        Box::new(ViewAnglesToCSV::new()),
        Box::new(WriteToFile::new(&header)),
    ];

    let specified_algorithms = matches.opt_strs("a");
    if specified_algorithms.is_empty() && !matches.opt_present("a") {
        algorithms.retain(|a| a.default());
    } else {
        algorithms.retain(|a| specified_algorithms.contains(&a.algorithm_name().to_string()));
    }

    if algorithms.is_empty() {
        panic!("No algorithms specified");
    }

    let unknown_algorithms: Vec<String> = specified_algorithms
        .into_iter()
        .filter(|a| algorithms.iter().all(|b| b.algorithm_name() != *a))
        .collect();
    if !unknown_algorithms.is_empty() {
        panic!("Unknown algorithms specified: {}", unknown_algorithms.join(", "));
    }

    print_metadata(&header);

    let mut detections = Vec::new();
    detections.extend(perform_tick(&header, ticker.borrow_mut(), algorithms));

    if start.elapsed().as_secs() >= 10 {
        print_metadata(&header);
    }

    let total_ticks = header.ticks;
    let total_time = start.elapsed().as_secs_f64();
    let total_tps = (total_ticks as f64) / total_time;
    dev_print!("Done! (Processed {} ticks in {:.2} seconds averaging {:.2} tps)", total_ticks, total_time, total_tps);

    if SILENT.load(std::sync::atomic::Ordering::Relaxed) {
        println!("{}", serde_json::to_string(&detections).unwrap());
    } else if matches.opt_present("c") {
        println!("Detection count: {}", detections.len());
    } else {
        println!("{}", serde_json::to_string_pretty(&detections).unwrap());
    }

    Ok(())
}

pub trait DemoTickEvent<'a> {
    fn default(&self) -> bool {
        panic!("default() not set for {}", std::any::type_name::<Self>());
    }

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
