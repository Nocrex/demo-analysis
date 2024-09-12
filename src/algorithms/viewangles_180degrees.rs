use anyhow::Error;
use serde_json::{json, Map, Value};
use crate::{DemoTickEvent, Detection};

// This example file looks for any examples of players rotating 180 degrees within a single server tick.

// To start, define a struct containing any information you want to store/share between events.
pub struct ViewAngles180Degrees {
    previous: Map<String, Value>,
}

// Then implement a pub fn new for your struct.
// Use the new() function to initalize any variables specified in the struct.
// IMPORTANT: new() gets called even if the algorithm is not selected! Don't do any non-ephemeral operations here; use DemoTickEvent::init() instead.
// Additional helper functions and consts also go here.

impl ViewAngles180Degrees {
    pub fn new() -> Self {
        let analyser: ViewAngles180Degrees = ViewAngles180Degrees { 
            previous: Map::new(),
        };
        analyser
    }

    fn calculate_delta(&self, curr_viewangle: f64, curr_pitchangle: f64, prev_viewangle: f64, prev_pitchangle: f64, tick_delta: u64) -> (f64, f64) {
        let va_delta = {
            let diff = (curr_viewangle - prev_viewangle).rem_euclid(360.0);
            if diff > 180.0 {
                diff - 360.0
            } else {
                diff
            }
        } / tick_delta as f64;
        let pa_delta = (curr_pitchangle - prev_pitchangle) / tick_delta as f64;
        (va_delta, pa_delta)
    }

}

// Implement the DemoTickEvent trait. This is where the bulk of your algorithm resides.
// Any interesting detections should be documented in a Detection instance and returned within a vector.
// You can attach whatever json data you want to each detection via the "data" field.

impl DemoTickEvent for ViewAngles180Degrees {
    fn on_tick(&mut self, tick: Value) -> Result<Vec<Detection>, Error> {
        let tick = tick.as_object().unwrap();
        let ticknum = tick["tick"].as_u64().unwrap();
        let players = tick["players"].as_array().unwrap();

        let mut detections = Vec::new();

        for player in players {
            let e = player.as_object().unwrap();

            let steam_id = e["info"]["steamId"].as_str().unwrap();
            let viewangle = e["view_angle"].as_f64().unwrap();
            let pitchangle = e["pitch_angle"].as_f64().unwrap();

            let tick_delta = {
                if ticknum == 0 {
                    0
                } else {
                    ticknum - self.previous.get("tick")
                        .and_then(|tick| tick.as_u64())
                        .unwrap_or(0)
                }
            };

            let (va_delta, pa_delta) = self.previous
                .get("players")
                .and_then(|players| players.as_array())
                .and_then(|players| players.iter().find(|p| p["info"]["steamId"].as_str().unwrap() == steam_id))
                .map(|prev_player| self.calculate_delta(
                    viewangle,
                    pitchangle,
                    prev_player["view_angle"].as_f64().unwrap(),
                    prev_player["pitch_angle"].as_f64().unwrap(),
                    tick_delta
                ))
                .unwrap_or((f64::NAN, f64::NAN));

            if va_delta.abs() >= 180.0 || pa_delta.abs() >= 180.0 {
                detections.push(Detection { 
                    tick: ticknum,
                    player: player.get("info").unwrap()
                        .get("userId")
                        .unwrap_or(&json!(0))
                        .as_u64().unwrap(),
                    data: json!({ "va_delta": va_delta, "pa_delta": pa_delta })
                });
            }
        }

        self.previous = tick.clone();

        Ok(detections)
    }
}
