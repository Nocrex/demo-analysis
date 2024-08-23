use std::{borrow::BorrowMut, env, fs};
use anyhow::Error;
use std::io::Write;
use serde_json::Value;

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

    let mut state_history: Vec<serde_json::Value> = Vec::new();

    let mut file = match fs::File::create("./test/test.wip") {
        Ok(file) => file,
        Err(err) => {
            if err.kind() != std::io::ErrorKind::AlreadyExists {
                panic!("Error creating file: {}", err);
            }
            fs::remove_file("./test/test.wip").unwrap();
            fs::File::create("./test/test.wip").unwrap()
        }
    };

    let mut ticknum: u32 = 0;

    let max_states_in_memory = 8 * 1024;

    writeln!(file, "[").unwrap();

    while ticker_result.is_ok_and(|b| b) { 

        if ticknum > 0 && (ticknum & (ticknum - 1) == 0 || ticknum % 1024 == 0) {
            println!("Processing tick {}...", ticknum);
        }

        // Get the GameState from the parser

        let state: &GameState = ticker.state();
        let mut json_value: Value = serde_json::to_value(state).unwrap();
        let json_object = json_value.as_object_mut().unwrap();
        json_object.insert("tick".to_string(), serde_json::Value::String(ticknum.to_string()));
        json_object.remove("kills");
        state_history.push(json_value);

        if state_history.len() > max_states_in_memory {
            let mut states_remaining = state_history.len();
            for json_object in &state_history {
                if states_remaining > 0 && (states_remaining & (states_remaining - 1) == 0 || states_remaining % 1024 == 0) {
                    println!("Flushing to disk ({} states remaining)", states_remaining);
                }
                write!(file, "{}", serde_json::to_string(&json_object).unwrap()).unwrap();
                writeln!(file, ",").unwrap();
                states_remaining -= 1;
            }
            state_history.clear();
        }

        // TODO: the fun stuff (implement cheat detection algorithm events)
        
        ticker_result = ticker.tick();
        ticknum += 1;
    }

    if state_history.len() > 0 {
        let mut states_remaining = state_history.len();
        for json_object in state_history {
            if states_remaining > 0 && (states_remaining & (states_remaining - 1) == 0 || states_remaining % 1024 == 0) {
                println!("Flushing to disk ({} states remaining)", states_remaining);
            }
            write!(file, "{}", serde_json::to_string(&json_object).unwrap()).unwrap();
            writeln!(file, ",").unwrap();
            states_remaining -= 1;
        }
    }

    writeln!(file, "]").unwrap();

    let _ = file.flush();

    println!("Done!");
}