use std::collections::{HashMap, HashSet};

use crate::{
    base::cheat_analyser_base::{CheatAnalyserState, PlayerState},
    CheatAlgorithm, Detection,
};
use anyhow::Error;
use serde_json::json;
use steamid_ng::SteamID;
use tf_demo_parser::ParserState;

pub struct OOBPitch {
    last_detections: HashSet<String>,
    
    params: HashMap<&'static str, f32>,
}

impl OOBPitch {
    pub fn new() -> Self {
        let analyser: OOBPitch = OOBPitch {
            last_detections: HashSet::new(),
            params: HashMap::from([
                ("min_pitch", -89.29412078857422),
                ("max_pitch", 89.29411315917969),
            ])
        };
        analyser
    }
}

impl<'a> CheatAlgorithm<'a> for OOBPitch {
    fn default(&self) -> bool {
        true
    }

    fn algorithm_name(&self) -> &str {
        "oob_pitch"
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
            let min_pitch = self.params["min_pitch"];
            let max_pitch = self.params["max_pitch"];

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
    
    fn params(&mut self) -> Option<&mut HashMap<&'static str, f32>> {
        Some(&mut self.params)    
    }
}
