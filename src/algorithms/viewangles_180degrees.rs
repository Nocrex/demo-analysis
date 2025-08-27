use anyhow::Error;
use serde_json::json;
use steamid_ng::SteamID;
use tf_demo_parser::ParserState;
use crate::{base::cheat_analyser_base::{CheatAnalyserState, PlayerState}, util::helpers::viewangle_delta, CheatAlgorithm, Detection};

// This example file looks for any examples of players rotating 180 degrees within a single server tick.

// To start, define a struct containing any information you want to store/share between events.
// Here we want to track the view angle and pitch angle of each player on the previous tick.
// Later we will compare the previous and current view angles to see if they are 180 degrees apart.
pub struct ViewAngles180Degrees {
    previous: Option<CheatAnalyserState>,
}

// Then implement a pub fn new for your struct.
// Use the new() function to initalize any variables specified in the struct.
// IMPORTANT: new() gets called even if the algorithm is not selected! Don't do any non-ephemeral operations here; use CheatAlgorithm::init() instead.
// Additional helper functions and consts also go here.

impl ViewAngles180Degrees {
    pub fn new() -> Self {
        let analyser: ViewAngles180Degrees = ViewAngles180Degrees { 
            previous: None,
        };
        analyser
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

    fn on_tick(&mut self, state: &CheatAnalyserState, _: &ParserState) -> Result<Vec<Detection>, Error> {
        let ticknum = u32::from(state.tick);
        let players = &state.players;

        let mut detections = Vec::new();

        // In the vast majority of cases you will only want to iterate over players that are:
        // - In PVS (data is being sent to the client)
        // - Alive (you can't cheat if you're dead)
        // - Not a tf_bot (you can't convict a tf_bot)
        for player in players.iter().filter(|p| {
            p.in_pvs && p.state == PlayerState::Alive && p.info.as_ref().is_some_and(|info| info.steam_id != "BOT")
        }) {
            let info = match &player.info {
                Some(info) => info,
                None => {continue}
            };

            let steam_id = &info.steam_id;
            let tick_delta = {
                if ticknum == 0 {
                    0
                } else {
                    ticknum - self.previous.as_ref().map_or(0, |pstate| pstate.tick.into())
                }
            };

            let (va_delta, pa_delta) = self.previous.as_ref()
                .map_or((f32::NAN, f32::NAN), |prev_state| {
                    match prev_state.players.iter().find(|p| {
                        p.in_pvs && p.state == PlayerState::Alive &&
                        p.info.as_ref().is_some_and(|i| i.steam_id == *steam_id)
                    }) {
                        Some(prev_player) => {
                            let prev_viewangle = prev_player.view_angle;
                            let prev_pitchangle = prev_player.pitch_angle;
                            viewangle_delta(player.view_angle, player.pitch_angle, prev_viewangle, prev_pitchangle, tick_delta)
                        },
                        None => (f32::NAN, f32::NAN)
                    }
                });
            // Creating the detection object
            // Avoid creating multiple detection objects for the same player and tick.
            // Nothing will break if you do, but it will overrepresent the data point.
            if va_delta.abs() >= 180.0 || pa_delta.abs() >= 180.0 {
                detections.push(Detection { 
                    tick: ticknum,
                    algorithm: self.algorithm_name().to_string(),
                    player: u64::from(SteamID::from_steam3(&steam_id).unwrap()),
                    data: json!({ "va_delta": va_delta, "pa_delta": pa_delta })
                });
            }
        }
        self.previous = Some(state.clone());
        // Any detections returned are official and final!
        // If you don't want to return any detections, just return an empty vector.
        // If your algorithm needs future ticks, you can store the detections within your algorithm's struct.
        // You can then return them in a later CheatAlgorithm::on_tick() or in CheatAlgorithm::finish().
        Ok(detections)
    }
}
