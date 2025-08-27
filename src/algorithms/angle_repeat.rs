// Written by Nocrex

use std::collections::HashMap;

use crate::{
    algorithms::util::jankguard::JankGuard,
    base::cheat_analyser_base::{CheatAnalyserState, Player, PlayerState},
    util, CheatAlgorithm, Detection,
};
use anyhow::Error;
use serde_json::json;
use steamid_ng::SteamID;
use tf_demo_parser::ParserState;

#[derive(Default)]
pub struct AngleRepeat {
    ticks: Vec<HashMap<u64, Player>>,

    jg: JankGuard,
    params: HashMap<&'static str, f32>,
}

impl AngleRepeat {
    pub fn new() -> Self {
        Self {
            params: HashMap::from([
                ("min_angle_diff_ratio", 20.0),
                ("max_first_third_angle_delta", 2.0),
                ("min_first_second_angle_delta", 5.0),
            ]),
            ..Default::default()
        }
    }
}

impl<'a> CheatAlgorithm<'a> for AngleRepeat {
    fn default(&self) -> bool {
        true
    }

    fn algorithm_name(&self) -> &str {
        "angle_repetition"
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
        self.ticks.truncate(3);

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

            let prev_player = self.ticks.get(1).and_then(|m| m.get(&steam_id)).cloned();
            let second_prev_player = self.ticks.get(2).and_then(|m| m.get(&steam_id)).cloned();

            if self.jg.teleported(&steam_id, ticknum) < 60
                || self.jg.spawned(&steam_id, ticknum) < 60
            {
                continue;
            }

            let third_angle = (player.view_angle, player.pitch_angle);
            self.ticks
                .get_mut(0)
                .unwrap()
                .insert(steam_id.clone(), player.clone()); // Store angle for this tick for next ticks

            if let (Some(second_data), Some(first_data)) = (prev_player, second_prev_player) {
                let first_angle = (first_data.view_angle, first_data.pitch_angle);
                let second_angle = (second_data.view_angle, second_data.pitch_angle);
                let first_second_delta = util::angle_delta(first_angle, second_angle);
                let first_third_delta = util::angle_delta(first_angle, third_angle);

                if first_second_delta < self.params["min_first_second_angle_delta"] {
                    // Ignore players with only a tiny adjustment in second angle
                    continue;
                }

                let ratio = first_second_delta / first_third_delta.max(1.0);

                if first_third_delta <= self.params["max_first_third_angle_delta"]
                    && ratio > self.params["min_angle_diff_ratio"]
                    && self.jg.fired(&steam_id, ticknum) < 3
                {
                    detections.push(Detection {
                        tick: ticknum,
                        algorithm: self.algorithm_name().to_string(),
                        player: steam_id,
                        data: json!({
                            "angle_1": first_angle,
                            "angle_2": second_angle,
                            "angle_3": third_angle,
                            "1_3_delta": first_third_delta,
                            "1_2_delta": first_second_delta,
                            "ratio": ratio,
                        }),
                    });
                }
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
