use std::{borrow::BorrowMut, env, fs::{self, File}};
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

    let mut file = match fs::File::create("./test/test.json") {
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

    let max_states_in_memory = 1024;

    write!(file, "[").unwrap();

    while ticker_result.is_ok_and(|b| b) { 

        if ticknum > 0 && (ticknum & (ticknum - 1) == 0 || ticknum % 1024 == 0) {
            println!("Processing tick {}...", ticknum);
        }

        // Get the GameState from the parser

        let state: &GameState = ticker.state();
        let mut json = get_gamestate_json(state);
        json = modify_json(&mut json, ticknum);

        state_history.push(json);

        if state_history.len() > max_states_in_memory {
            write_states_to_file(&mut file, &state_history);

            state_history.clear();
        }

        // TODO: the fun stuff (implement cheat detection algorithm events)
        
        ticker_result = ticker.tick();
        ticknum += 1;
    }

    if state_history.len() > 0 {
        write_states_to_file(&mut file,&state_history);

        state_history.clear();
    }

    writeln!(file, "]").unwrap();

    let _ = file.flush();

    println!("Done!");
}

fn get_gamestate_json(state: &GameState) -> Value {
    serde_json::to_value(state).unwrap()
}

fn modify_json(state_json: &mut Value, ticknum: u32) -> Value {
    let json_object = state_json.as_object_mut().unwrap();

    // Insert tick number
    json_object.insert("tick".to_string(), serde_json::Value::String(ticknum.to_string()));

    // Remove kills as it is cumulative (only need latest value)
    json_object.remove("kills");

    json_object.entry("players".to_string()).and_modify(|v| {
        let players = v.as_array_mut().unwrap();
        *players = players.iter().filter(|p| p["in_pvs"].as_bool().unwrap()).cloned().collect();
    });

    return serde_json::to_value(json_object).unwrap();

}

fn write_states_to_file(file: &mut File, states: &Vec<Value>) {
    let out = states.iter()
                .map(|j| serde_json::to_string(&j).unwrap())
                .collect::<Vec<String>>().join(",\n") + ",\n";

    writeln!(file, "{}", out).unwrap();
}