pub mod ticker;
mod algorithms {
    pub mod viewangles_to_csv;
    pub mod write_to_file;
}

use std::{borrow::BorrowMut, env, fs::{self}};
use anyhow::Error;
use serde_json::Value;

use crate::ticker::perform_tick;
use algorithms::{viewangles_to_csv::ViewAnglesToCSV, write_to_file::DemoAnalysisFileWriter};

use tf_demo_parser::demo::parser::gamestateanalyser::GameStateAnalyser;
pub use tf_demo_parser::{Demo, DemoParser, Parse, ParseError, ParserState, Stream};

pub static SILENT: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

fn main() -> Result<(), Error> {
    let mut silent = false;
    let mut path = String::new();
    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-i" => {
                path = args.next().expect("Expected input file path after -i");
            },
            "-q" => silent = true,
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

    match ticker {
        Ok((header, mut ticker)) => {

            let mut events: Vec<Box<dyn DemoTickEvent>> = Vec::with_capacity(1);

            events.push(Box::new(DemoAnalysisFileWriter::new(&header)));
            events.push(Box::new(ViewAnglesToCSV::new()));

            perform_tick(&header, ticker.borrow_mut(), events);
        }
        Err(_) => todo!(),
    }

    Ok(())
}

pub trait DemoTickEvent {

    fn on_tick<'a>(&mut self, _tick: Value) -> Result<Option<Value>, Error> {
        Ok(None)
    }

    fn finish<'a>(&mut self) -> Result<Option<Value>, Error> {
        Ok(None)
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
