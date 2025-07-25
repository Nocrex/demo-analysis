use std::collections::HashMap;

use crate::{
    base::cheat_analyser_base::{CheatAnalyserState, Player, PlayerState},
    util, CheatAlgorithm, Detection,
};
use anyhow::Error;
use serde_json::json;
use steamid_ng::SteamID;
use tf_demo_parser::ParserState;

use super::jankguard::JankGuard;

#[derive(Default)]
pub struct AimSnap {
    ticks: Vec<HashMap<u64, Player>>,

    jg: JankGuard,
    params: HashMap<&'static str, f32>,
}

impl AimSnap {
    pub fn new() -> Self {
        Self {
            params: HashMap::from([
                ("noise_max", 0.5),
                ("noise_min", 0.001),
                ("snap_threshold", 10.0),
            ]),
            ..Default::default()
        }
    }
}

impl<'a> CheatAlgorithm<'a> for AimSnap {
    fn default(&self) -> bool {
        false
    }

    fn algorithm_name(&self) -> &str {
        "aimsnap"
    }

    fn on_tick(
        &mut self,
        state: &CheatAnalyserState,
        _: &ParserState,
    ) -> Result<Vec<Detection>, Error> {
        self.jg.on_tick(state);
        let ticknum = u32::from(state.tick);
        let players = &state.players;

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
                deltas.push(util::angle_delta(*a, *b));
            }
            let noise_range = self.params["noise_min"]..self.params["noise_max"];

            if noise_range.contains(deltas.first().unwrap())
                && noise_range.contains(deltas.last().unwrap())
                && deltas.iter().filter(|&d| noise_range.contains(d)).count() == deltas.len() - 1
                && deltas
                    .iter()
                    .filter(|&&d| d > self.params["snap_threshold"])
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

    fn params(&mut self) -> Option<&mut HashMap<&'static str, f32>> {
        Some(&mut self.params)
    }
}
