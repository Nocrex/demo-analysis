// Written by Nocrex

use std::collections::{HashMap, HashSet};

use crate::{
    base::cheat_analyser_base::{CheatAnalyserState, PlayerState}
};
use anyhow::Error;
use serde_json::json;
use steamid_ng::SteamID;
use tf_demo_parser::ParserState;

use crate::lib::algorithm::{CheatAlgorithm, Detection};
use crate::lib::parameters::{Parameter, Parameters, get_parameter_value};

pub struct OOBPitch {
    last_detections: HashSet<String>,
    
    params: Parameters,
}

impl OOBPitch {
    pub fn new() -> Self {
        let analyser: OOBPitch = OOBPitch {
            last_detections: HashSet::new(),
            params: HashMap::from([
                ("min_pitch".to_string(), Parameter::Float(-89.999)),
                ("max_pitch".to_string(), Parameter::Float(89.999)),
            ]),
        };
        analyser
    }
}

impl<'a> CheatAlgorithm<'a> for OOBPitch {
    fn default(&self) -> bool {
        true
    }

    fn algorithm_name(&self) -> &str {
        "nocrex/oob_pitch"
    }

    fn on_tick(
        &mut self,
        state: &CheatAnalyserState,
        _: &ParserState,
    ) -> Result<Vec<Detection>, Error> {
        let ticknum = u32::from(state.tick);
        let players = &state.players;

        let mut submitted_detections = Vec::new();

        let mut detections = HashSet::new();

        let min_pitch: f32 = get_parameter_value(&self.params, "min_pitch");
        let max_pitch: f32 = get_parameter_value(&self.params, "max_pitch");

        for player in players.iter().filter(|p| {
            p.in_pvs
                && p.state == PlayerState::Alive
                && p.info.as_ref().is_some_and(|info| info.steam_id != "BOT")
        }) {
            let info = match &player.info {
                Some(info) => info,
                None => continue,
            };

            let steam_id = &info.steam_id;

            if !(min_pitch..=max_pitch).contains(&player.pitch_angle) {
                detections.insert(steam_id.clone());
                if !self.last_detections.contains(steam_id){
                    submitted_detections.push(Detection {
                        tick: ticknum,
                        algorithm: self.algorithm_name().to_string(),
                        player: u64::from(SteamID::from_steam3(&steam_id).unwrap()),
                        data: json!({ "pitch": player.pitch_angle }),
                    });
                }
            }
        }
        self.last_detections = detections;
        Ok(submitted_detections)
    }
    
    fn params(&mut self) -> Option<&mut Parameters> {
        Some(&mut self.params)
    }
}
