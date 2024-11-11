use serde_json::Value;

use tf_demo_parser::demo::header::Header;
use tf_demo_parser::demo::parser::DemoTicker;
pub use tf_demo_parser::ParseError;

use crate::analysers::cheat_analyser_base::{CheatAnalyser, CheatAnalyserState};
use crate::{dev_print, DemoTickEvent, Detection};

// We return the total number of ticks iterated through in case the header is corrupted (e.g. game crash).
pub fn perform_tick<'a> (header: &Header, ticker: &mut DemoTicker<CheatAnalyser>, mut events: Vec<Box<dyn DemoTickEvent + 'a>>) -> (Vec<Detection>, u32) {

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

        // Get the CheatAnalyserState from the parser

        let state: &CheatAnalyserState = ticker.state();

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

    return (detections, ticker.state().tick.into());
}

fn get_gamestate_json(state: &CheatAnalyserState) -> Value {
    serde_json::to_value(state).unwrap()
}

fn modify_json(state_json: &mut Value) -> Value {
    let json_object = state_json.as_object_mut().unwrap();

    json_object.entry("players".to_string()).and_modify(|v| {
        let players = v.as_array_mut().unwrap();
        *players = players.iter().filter(|p| {
            p["in_pvs"].as_bool().unwrap() &&
            p["state"].as_str().unwrap() == "Alive" &&
            p["info"]["steamId"].as_str().unwrap() != "BOT"
        } ).cloned().collect();
    });

    return serde_json::to_value(json_object).unwrap();
}