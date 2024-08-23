use std::{borrow::BorrowMut, env, fs};
use anyhow::Error;

use tf_demo_parser::demo::parser::{gamestateanalyser::{GameState, GameStateAnalyser}, DemoTicker};
pub use tf_demo_parser::{Demo, DemoParser, Parse, ParseError, ParserState, Stream};

fn main() -> Result<(), Error> {
       let args: Vec<_> = env::args().collect();
    if args.len() < 2 {
        println!("1 argument required");
        return Ok(());
    }
    let path = args[1].clone();
    let file = fs::read(path)?;
    let demo = Demo::new(&file);
    let parser = DemoParser::new_with_analyser(demo.get_stream(), GameStateAnalyser::new());
    let ticker = parser.ticker();

    match ticker {
        Ok((_, mut ticker)) => {
            perform_tick(ticker.borrow_mut());
        }
        Err(_) => todo!(),
    }

    Ok(())
}

fn perform_tick(ticker: &mut DemoTicker<GameStateAnalyser>) {
  
    let mut ticker_result: Result<bool, ParseError> = Ok(true);

    while ticker_result.is_ok_and(|b| b) { 

        // Get the GameState from the parser

        let state: &GameState = ticker.state();
        let state_string = serde_json::to_string_pretty(state);

        if state_string.is_ok() {
            println!("{}", state_string.unwrap());
        } else {
            break;
        }

        // TODO: the fun stuff (implement cheat detection algorithm events)
        
        ticker_result = ticker.tick();
    }
}