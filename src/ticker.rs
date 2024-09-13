use serde_json::Value;

use tf_demo_parser::demo::header::Header;
use tf_demo_parser::demo::parser::{gamestateanalyser::{GameState, GameStateAnalyser}, DemoTicker};
pub use tf_demo_parser::{Demo, DemoParser, Parse, ParseError, ParserState, Stream};

use crate::{dev_print, DemoTickEvent, Detection};

pub fn perform_tick<'a> (header: &Header, ticker: &mut DemoTicker<GameStateAnalyser>, mut events: Vec<Box<dyn DemoTickEvent + 'a>>) -> Vec<Detection> {

    let mut ticker_result: Result<bool, ParseError> = Ok(true);
    let mut prior_tick: u32 = 1;
    let start = std::time::Instant::now();

    let mut last_update = std::time::Instant::now();
    let mut tps_start_window = start;
    let mut tps_start_window_tick: Vec<u32> = vec![0];
    
    let mut detections: Vec<Detection> = Vec::new();

    dev_print!("Starting analysis...");

    // DemoTickEvent::init()
    for event in events.iter_mut() {
        let _ = event.init();
    }

    while ticker_result.is_ok_and(|b| b) { 

        // Get the GameState from the parser

        let state: &GameState = ticker.state();

        if state.tick == prior_tick {
            ticker_result = ticker.tick();
            continue;
        }

        if !crate::SILENT.load(std::sync::atomic::Ordering::Relaxed) && last_update.elapsed().as_secs() >= 1 {
            let tps: u32 = u32::from(state.tick - tps_start_window_tick[0]) / tps_start_window.elapsed().as_secs() as u32;
            dev_print!("Processing tick {} ({} remaining, {} tps)", state.tick, header.ticks - u32::from(state.tick), tps);
            last_update = std::time::Instant::now();
            tps_start_window = std::cmp::max(start, last_update - std::time::Duration::from_secs(10));
            tps_start_window_tick.push(u32::from(state.tick));
            if tps_start_window_tick.len() > 10 {
                tps_start_window_tick.remove(0);
            }
        }

        let mut json = get_gamestate_json(state);
        json = modify_json(&mut json);

        // DemoTickEvent::on_tick()
        for event in events.iter_mut() {
            match event.on_tick(json.clone()) {
                Ok(d) => {
                    detections.extend(d);       
                },
                Err(e) => {
                    dev_print!("Error: {}", e);
                }
            }
        }
        
        prior_tick = u32::from(state.tick);

        ticker_result = ticker.tick();
    }

    // DemoTickEvent::finish()
    for event in events.iter_mut() {
        let _ = event.finish();
    }

    return detections;
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