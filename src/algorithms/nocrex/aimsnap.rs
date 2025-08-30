// Written by Nocrex

use std::{collections::HashMap, ops::Range};

use crate::{
    base::cheat_analyser_base::{CheatAnalyserState, Player, PlayerState}, dev_print, util::{helpers::angle_delta, nocrex::jankguard::JankGuard}
};
use anyhow::Error;
use serde_json::json;
use steamid_ng::SteamID;
use tf_demo_parser::ParserState;

use crate::lib::algorithm::{CheatAlgorithm, Detection};
use crate::lib::parameters::{Parameter, Parameters};

#[derive(Default)]
pub struct AimSnap {
    ticks: Vec<HashMap<u64, Player>>,
    jg: JankGuard,
    params: Parameters,
}

impl AimSnap {
    pub fn new() -> Self {
        Self {
            params: HashMap::from([
                ("noise_min".to_string(), Parameter::Float(0.028)),
                ("noise_max".to_string(), Parameter::Float(0.99)),
                ("snap_threshold".to_string(), Parameter::Float(10.0)),
            ]),
            ..Default::default()
        }
    }
}

impl<'a> CheatAlgorithm<'a> for AimSnap {
    fn default(&self) -> bool {
        true
    }

    fn algorithm_name(&self) -> &str {
        "nocrex/aimsnap"
    }

    fn on_tick(
        &mut self,
        state: &CheatAnalyserState,
        _: &ParserState,
    ) -> Result<Vec<Detection>, Error> {
        self.jg.on_tick(state);
        let ticknum = u32::from(state.tick);
        let players = &state.players;

        let noise_range: Range<f32> = match (
            self.params.get("noise_min"),
            self.params.get("noise_max"),
        ) {
            (Some(Parameter::Float(min)), Some(Parameter::Float(max))) => *min..*max,
            _ => {
                dev_print!("Warning: Invalid noise bounds for {}. Using default of 0.028..0.99", self.algorithm_name());
                0.028..0.99
            },
        };

        let snap_threshold = match self.params.get("snap_threshold") {
            Some(Parameter::Float(thresh)) => *thresh,
            _ => {
                dev_print!("Warning: Invalid snap_threshold for {}. Using default of 10.0", self.algorithm_name());
                10.0
            },
        };

        let mut detections = Vec::new();

        self.ticks.insert(0, HashMap::new());
        self.ticks.truncate(5);

        for player in players.iter().filter(|p| {
            p.in_pvs
                && p.state == PlayerState::Alive
                && p.info.as_ref().is_some_and(|info| info.steam_id != "BOT")
        }) {
            let info = match &player.info {
                Some(info) => info,
                None => continue,
            };

            let steam_id: u64 = u64::from(SteamID::from_steam3(&info.steam_id).unwrap());

            if self.jg.teleported(&steam_id, ticknum) < 60
                || self.jg.spawned(&steam_id, ticknum) < 60
            {
                continue;
            }

            self.ticks
                .get_mut(0)
                .unwrap()
                .insert(steam_id.clone(), player.clone()); // Store angle for this tick for next ticks

            let mut angles: Vec<_> = self
                .ticks
                .iter()
                .map(|m| m.get(&steam_id).map(|p| (p.view_angle, p.pitch_angle)))
                .rev()
                .collect();

            if angles.iter().any(|o| o.is_none()) {
                continue;
            }

            let angles: Vec<(f32, f32)> = angles.drain(..).map(|o| o.unwrap()).collect();
            let mut deltas = Vec::new();
            for (a, b) in angles.iter().zip(angles.iter().skip(1)) {
                deltas.push(angle_delta(*a, *b));
            }

            if noise_range.contains(deltas.first().unwrap())
                && noise_range.contains(deltas.last().unwrap())
                && deltas.iter().filter(|&d| noise_range.contains(d)).count() == deltas.len() - 1
                && deltas
                    .iter()
                    .filter(|&&d| d > snap_threshold)
                    .count()
                    == 1
                && self.jg.fired(&steam_id, ticknum) < 5
            {
                detections.push(Detection {
                    tick: ticknum - 2,
                    algorithm: self.algorithm_name().to_string(),
                    player: steam_id,
                    data: json!({
                        "deltas": deltas
                    }),
                });
            }
        }
        Ok(detections)
    }

    fn handled_messages(&self) -> Result<Vec<tf_demo_parser::MessageType>, bool> {
        self.jg.handled_messages()
    }

    fn on_message(
        &mut self,
        message: &tf_demo_parser::demo::message::Message,
        state: &CheatAnalyserState,
        parser_state: &ParserState,
        tick: tf_demo_parser::demo::data::DemoTick,
    ) -> Result<Vec<Detection>, Error> {
        self.jg.on_message(message, state, parser_state, tick);
        Ok(vec![])
    }

    fn params(&mut self) -> Option<&mut Parameters> {
        Some(&mut self.params)
    }
}
