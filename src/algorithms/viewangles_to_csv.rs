use std::fs::{self, File};
use std::io::Write;

use anyhow::Error;
use tf_demo_parser::ParserState;
use crate::base::cheat_analyser_base::{CheatAnalyserState, PlayerState};
use crate::util::viewangle_delta;
use crate::{CheatAlgorithm, Detection};

pub struct ViewAnglesToCSV {
    file: Option<File>,
    previous: Option<CheatAnalyserState>,
}

impl ViewAnglesToCSV {
    pub fn new() -> Self {
        let writer: ViewAnglesToCSV = ViewAnglesToCSV { 
            file: None,
            previous: None,
        };
        writer
    }

    fn init_file(&mut self, file_path: &str) {
        self.file = Some(match File::create(file_path) {
            Ok(file) => file,
            Err(err) => {
                if err.kind() != std::io::ErrorKind::AlreadyExists {
                    panic!("Error creating file: {}", err);
                }
                fs::remove_file(file_path).unwrap();
                File::create(file_path).unwrap()
            }
        });
    }

    fn calculate_delta(&self, curr_viewangle: f64, curr_pitchangle: f64, prev_viewangle: f64, prev_pitchangle: f64, tick_delta: u64) -> (f64, f64) {
        let tick_delta = if tick_delta < 1 { 1 } else { tick_delta };
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

    fn escape_csv_string(&self, input: &str) -> String {
        let mut output = String::new();
        output.push('"');
    
        for c in input.chars() {
            if c == '"' {
                output.push_str("\"\"");
            } else {
                output.push(c);
            }
        }
    
        output.push('"');
        output
    }

}

impl<'a> CheatAlgorithm<'a> for ViewAnglesToCSV {
    fn default(&self) -> bool {
        false
    }

    fn algorithm_name(&self) -> &str {
        "viewangles_to_csv"
    }

    fn init(&mut self) -> Result<(), Error> {
        self.init_file("./test/viewangles_to_csv.csv");
        writeln!(self.file.as_mut().unwrap(), "tick,name,steam_id,origin_x,origin_y,origin_z,viewangle,pitchangle,va_delta,pa_delta").unwrap();
        Ok(())
    }

    fn on_tick(&mut self, state: &CheatAnalyserState, _: &ParserState) -> Result<Vec<Detection>, Error> {
        let ticknum = u32::from(state.tick);
        let players = &state.players;

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

            let name = &info.name;
            let origin_x = player.position.x;
            let origin_y = player.position.y;
            let origin_z = player.position.z;
            let viewangle = player.view_angle;
            let pitchangle = player.pitch_angle;
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
                
            writeln!(
                self.file.as_mut().unwrap(),
                "{},{},{},{},{},{},{},{},{},{}",
                ticknum,
                name,
                steam_id,
                origin_x,
                origin_y,
                origin_z,
                viewangle,
                pitchangle,
                va_delta,
                pa_delta
            )
            .unwrap();
        }
        self.previous = Some(state.clone());

        Ok(vec![])
    }
}
