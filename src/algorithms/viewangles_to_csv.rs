use std::fs::{self, File};
use std::io::Write;

use anyhow::Error;
use serde_json::{Map, Value};
use crate::{DemoTickEvent, Detection};

pub struct ViewAnglesToCSV {
    file: Option<File>,
    previous: Map<String, Value>,
}

impl ViewAnglesToCSV {
    pub fn new() -> Self {
        let writer: ViewAnglesToCSV = ViewAnglesToCSV { 
            file: None,
            previous: Map::new(),
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

impl DemoTickEvent for ViewAnglesToCSV {

    fn init<'a>(&mut self) -> Result<Vec<Detection>, Error> {
        self.init_file("./test/viewangles_to_csv.csv");
        writeln!(self.file.as_mut().unwrap(), "tick,steam_id,origin_x,origin_y,origin_z,viewangle,pitchangle,va_delta,pa_delta").unwrap();
        Ok(vec![])
    }

    fn on_tick(&mut self, tick: Value) -> Result<Vec<Detection>, Error> {
        let tick = tick.as_object().unwrap();
        let ticknum = tick["tick"].as_u64().unwrap();
        let players = tick["players"].as_array().unwrap();

        for player in players {
            let e = player.as_object().unwrap();

            let steam_id = e["info"]["steamId"].as_str().unwrap();
            let origin_x = e["position"]["x"].as_f64().unwrap();
            let origin_y = e["position"]["y"].as_f64().unwrap();
            let origin_z = e["position"]["z"].as_f64().unwrap();
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

            writeln!(self.file.as_mut().unwrap(), "{},{},{},{},{},{},{},{},{}", ticknum, steam_id, origin_x, origin_y, origin_z, viewangle, pitchangle, va_delta, pa_delta).unwrap();
        }

        self.previous = tick.clone();

        Ok(vec![])
    }
}
