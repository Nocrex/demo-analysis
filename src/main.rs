pub mod ticker;
mod algorithms {
    pub mod write_to_file;
}

use std::{borrow::BorrowMut, env, fs::{self}};
use anyhow::Error;
use serde_json::Value;

use crate::ticker::perform_tick;
use algorithms::write_to_file::DemoAnalysisFileWriter;

use tf_demo_parser::demo::parser::gamestateanalyser::GameStateAnalyser;
pub use tf_demo_parser::{Demo, DemoParser, Parse, ParseError, ParserState, Stream};

fn main() -> Result<(), Error> {
       let args: Vec<_> = env::args().collect();
    if args.len() < 2 {
        println!("1 argument required");
        return Ok(());
    }
    let path = args[1].clone();
    let file = fs::read(path)?;
    let demo: Demo = Demo::new(&file);
    let parser = DemoParser::new_with_analyser(demo.get_stream(), GameStateAnalyser::new());
    let ticker = parser.ticker();

    match ticker {
        Ok((header, mut ticker)) => {

            let mut events: Vec<Box<dyn DemoTickEvent>> = Vec::with_capacity(1);

            events.push(Box::new(DemoAnalysisFileWriter::new(&header)));

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