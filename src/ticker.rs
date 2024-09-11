use serde_json::Value;

use tf_demo_parser::demo::header::Header;
use tf_demo_parser::demo::parser::{gamestateanalyser::{GameState, GameStateAnalyser}, DemoTicker};
pub use tf_demo_parser::{Demo, DemoParser, Parse, ParseError, ParserState, Stream};

use crate::{dev_print, DemoTickEvent};

pub fn perform_tick<'a> (header: &Header, ticker: &mut DemoTicker<GameStateAnalyser>, mut events: Vec<Box<dyn DemoTickEvent + 'a>>) {
  
    let mut ticker_result: Result<bool, ParseError> = Ok(true);
    let mut last_update = std::time::Instant::now();
    let mut prior_tick: u32 = 1;
    let start = std::time::Instant::now();

    dev_print!("Starting analysis...");

    print_metadata(header);

    while ticker_result.is_ok_and(|b| b) { 

        // Get the GameState from the parser

        let state: &GameState = ticker.state();

        if state.tick == prior_tick {
            ticker_result = ticker.tick();
            continue;
        }

        if last_update.elapsed().as_secs() >= 1 {
            let tps: u32 = u32::from(state.tick) / start.elapsed().as_secs() as u32;
            dev_print!("Processing tick {} ({} remaining, {} tps)", state.tick, header.ticks - u32::from(state.tick), tps);
            last_update = std::time::Instant::now();
        }

        let mut json = get_gamestate_json(state);
        json = modify_json(&mut json);

        for event in events.iter_mut() {
            event.on_tick(json.clone()).unwrap();
        }
        
        prior_tick = u32::from(state.tick);

        ticker_result = ticker.tick();
    }

    for event in events.iter_mut() {
        let _ = event.finish(); // Fire the end event.
    }

    print_metadata(header);

    dev_print!("Done! (Processed {} ticks in {} seconds)", header.ticks, start.elapsed().as_secs());
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

fn get_gamestate_json(state: &GameState) -> Value {
    serde_json::to_value(state).unwrap()
}

fn modify_json(state_json: &mut Value) -> Value {
    let json_object = state_json.as_object_mut().unwrap();

    // Remove kills as it is cumulative (only need latest value)
    // TODO: remove this once the parser is updated to not cumulate kill events
    json_object.remove("kills");

    json_object.entry("players".to_string()).and_modify(|v| {
        let players = v.as_array_mut().unwrap();
        *players = players.iter().filter(|p| {
            p["in_pvs"].as_bool().unwrap() &&
            p["state"].as_str().unwrap() == "Alive"
        } ).cloned().collect();
    });

    return serde_json::to_value(json_object).unwrap();
}