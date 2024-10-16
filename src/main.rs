pub mod ticker;
mod algorithms {
    pub mod viewangles_180degrees;
    pub mod viewangles_to_csv;
    pub mod write_to_file;
}

use std::{borrow::BorrowMut, collections::HashMap, env, fs::{self}};
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

    let unknown_algorithms: Vec<String> = specified_algorithms
        .into_iter()
        .filter(|a| algorithms.iter().all(|b| b.algorithm_name() != *a))
        .collect();
    if !unknown_algorithms.is_empty() {
        panic!("Unknown algorithms specified: {}", unknown_algorithms.join(", "));
    } else if algorithms.is_empty() {
        panic!("No algorithms specified");
    }

    print_metadata(&header, header.ticks);

    let (detections, actual_ticks) = perform_tick(&header, ticker.borrow_mut(), algorithms);

    if start.elapsed().as_secs() >= 10 {
        print_metadata(&header, actual_ticks);
    }

    let total_ticks = header.ticks;
    let total_time = start.elapsed().as_secs_f64();
    let total_tps = (total_ticks as f64) / total_time;
    dev_print!("Done! (Processed {} ticks in {:.2} seconds averaging {:.2} tps)", total_ticks, total_time, total_tps);

    if SILENT.load(std::sync::atomic::Ordering::Relaxed) {
        print_detection_json(&header, &detections, actual_ticks, pretty);
    } else if matches.opt_present("c") {
        print_detection_count(&detections);
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

fn print_metadata(header: &Header, ticks: u32) {
    dev_print!("Map: {}", header.map);
    let hours = (header.duration / 3600.0).floor();
    let minutes = ((header.duration % 3600.0) / 60.0).floor();
    let seconds = (header.duration % 60.0).floor();
    let milliseconds = ((header.duration % 1.0) * 100.0).floor();
    dev_print!("Duration: {:02}:{:02}:{:02}.{:03} ({} ticks)", hours, minutes, seconds, milliseconds, ticks);
    dev_print!("User: {}", header.nick);
    dev_print!("Server: {}", header.server);
}

fn print_detection_json(header: &Header, detections: &Vec<Detection>, ticks: u32, pretty: bool) {
    let analysis = serde_json::json!({
        "server_ip": header.server.clone(),
        "duration": ticks,
        "author": header.nick.clone(),
        "map": header.map.clone(),
        "detections": detections
    });
    let json = if pretty {
        serde_json::to_string_pretty(&analysis).unwrap()
    } else {
        serde_json::to_string(&analysis).unwrap()
    };
    println!("{}", json);
}

fn print_detection_count(detections: &Vec<Detection>) {
    let mut algorithm_counts: HashMap<String, HashMap<u64, usize>> = HashMap::new();
    for detection in detections {
        let algorithm = detection.algorithm.clone();
        let steamid = detection.player;
        *algorithm_counts.entry(algorithm).or_insert(HashMap::new()).entry(steamid).or_insert(0) += 1;
    }

    dev_print!("Total detections: {}", detections.len());
    if detections.is_empty() {
        return;
    }
    dev_print!("Detections by Algorithm:");
    for (algorithm, steamid_counts) in algorithm_counts {
        dev_print!("  {}: {} players, {} detections", algorithm, steamid_counts.len(), steamid_counts.values().sum::<usize>());
        let mut steamid_counts_vec: Vec<_> = steamid_counts.into_iter().collect();
        steamid_counts_vec.sort_by(|a, b| b.1.cmp(&a.1));
        for (steamid, count) in steamid_counts_vec {
            dev_print!("    {}: {}", steamid, count);
        }
    }

}

#[macro_export]
macro_rules! dev_print {
    ($($arg:tt)*) => {
        if !crate::SILENT.load(std::sync::atomic::Ordering::SeqCst) {
            println!($($arg)*);
        }
    }
}
