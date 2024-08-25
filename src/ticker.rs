use std::fs::{self, File};
use std::io::Write;
use serde_json::Value;

use tf_demo_parser::demo::parser::{gamestateanalyser::{GameState, GameStateAnalyser}, DemoTicker};
pub use tf_demo_parser::{Demo, DemoParser, Parse, ParseError, ParserState, Stream};

use crate::DemoTickEvent;

pub fn perform_tick<'a> (ticker: &mut DemoTicker<GameStateAnalyser>, events: Vec<Box<dyn DemoTickEvent + 'a>>) {
  
    let mut ticker_result: Result<bool, ParseError> = Ok(true);

    let mut ticknum: u32 = 0;

    while ticker_result.is_ok_and(|b| b) { 

        if ticknum > 0 && (ticknum & (ticknum - 1) == 0 || ticknum % 1024 == 0) {
            println!("Processing tick {}...", ticknum);
        }

        // Get the GameState from the parser

        let state: &GameState = ticker.state();
        let mut json = get_gamestate_json(state);
        json = modify_json(&mut json, ticknum);
        
        ticker_result = ticker.tick();
        ticknum += 1;
    }

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