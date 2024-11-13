use anyhow::Error;
use serde_json::{json, Map, Value};
use steamid_ng::SteamID;
use crate::{CheatAlgorithm, Detection};

// This example file looks for any examples of players rotating 180 degrees within a single server tick.

// To start, define a struct containing any information you want to store/share between events.
// Here we want to track the view angle and pitch angle of each player on the previous tick.
// Later we will compare the previous and current view angles to see if they are 180 degrees apart.
pub struct ViewAngles180Degrees {
    previous: Map<String, Value>,
}

// Then implement a pub fn new for your struct.
// Use the new() function to initalize any variables specified in the struct.
// IMPORTANT: new() gets called even if the algorithm is not selected! Don't do any non-ephemeral operations here; use CheatAlgorithm::init() instead.
// Additional helper functions and consts also go here.

impl ViewAngles180Degrees {
    pub fn new() -> Self {
        let analyser: ViewAngles180Degrees = ViewAngles180Degrees { 
            previous: Map::new(),
        };
        analyser
    }

    // Compute the difference in viewangles. We have to account for the fact viewangles are in a circle.
    // E.g. If viewangle goes from 350 to 10 degrees, we want to return 20 degrees.
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

// Implement the CheatAlgorithm trait. This is where the bulk of your algorithm resides.
// Any interesting detections should be documented in a Detection object and returned within a vector.
// You can attach whatever json data you want to each detection via the "data" field.
// You don't have to implement every function in CheatAlgorithm; see its definition for a complete list of functions.

impl<'a> CheatAlgorithm<'a> for ViewAngles180Degrees {
    // REQUIRED: Should this algorithm run by default if -a isn't specified?
    // Generally should be true, unless you're doing dev-only stuff (writing to files, printing debug output, etc).
    fn default(&self) -> bool {
        true
    }

    // REQUIRED: Set your algorithm's name here. Best practice is to match the filename.
    fn algorithm_name(&self) -> &str {
        "viewangles_180degrees"
    }

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

            // Creating the detection object
            // Avoid creating multiple detection objects for the same player and tick.
            // Nothing will break if you do, but it will overrepresent the data point.
            if va_delta.abs() >= 180.0 || pa_delta.abs() >= 180.0 {
                detections.push(Detection { 
                    tick: ticknum,
                    algorithm: self.algorithm_name().to_string(),
                    player: u64::from(SteamID::from_steam3(steam_id).unwrap()),
                    data: json!({ "va_delta": va_delta, "pa_delta": pa_delta })
                });
            }
        }

        self.previous = tick.clone();

        // Any detections returned are official and final!
        // If you don't want to return any detections, just return an empty vector.
        // If your algorithm needs future ticks, you can store the detections within your algorithm's struct.
        // You can then return them in a later CheatAlgorithm::on_tick() or in CheatAlgorithm::finish().
        Ok(detections)
    }
}
