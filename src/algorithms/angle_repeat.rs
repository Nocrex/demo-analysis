use std::collections::HashMap;

use crate::{
    base::cheat_analyser_base::{CheatAnalyserState, Player, PlayerState},
    util, CheatAlgorithm, Detection,
};
use anyhow::Error;
use serde_json::json;
use steamid_ng::SteamID;
use tf_demo_parser::ParserState;

const MIN_ANGLE_DIFF_RATIO: f32 = 20.0;
const MAX_FIRST_THIRD_ANGLE_DELTA: f32 = 2.0;
const TELEPORT_DIST: f32 = 256.0;

#[derive(Default)]
pub struct AngleRepeat {
    ticks: Vec<HashMap<u64, Player>>,

    last_spawns: HashMap<u64, u32>,
}

impl AngleRepeat {
    pub fn new() -> Self {
        Default::default()
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
        let ticknum = u32::from(state.tick);
        let players = &state.players;

        let mut detections = Vec::new();

        let mut data = HashMap::new();

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

            let prev_player = self.ticks.get(1).and_then(|m|m.get(&steam_id));
            let second_prev_player = self.ticks.get(2).and_then(|m|m.get(&steam_id));

            if prev_player.is_some_and(|p| {
                // Ignore players that just moved more than 256 HUs in a single tick (teleport)
                let diff = p.position - player.position;
                let sq_len = diff.x.powi(2) + diff.y.powi(2) + diff.z.powi(2);
                sq_len > TELEPORT_DIST.powi(2)
            }) {
                continue;
            }
            
            // Ignore players that just spawned
            if (state.tick
                - self
                    .last_spawns
                    .get(&steam_id)
                    .map(|v| *v)
                    .unwrap_or_default())
                < 60
            {
                continue;
            }

            let third_angle = (player.view_angle, player.pitch_angle);
            data.insert(steam_id.clone(), player.clone()); // Store angle for this tick for next ticks

            if let (Some(second_data), Some(first_data)) = (prev_player, second_prev_player) {
                let first_angle = (first_data.view_angle, first_data.pitch_angle);
                let second_angle = (second_data.view_angle, second_data.pitch_angle);
                let first_second_delta = util::angle_delta(first_angle, second_angle);
                let first_third_delta = util::angle_delta(first_angle, third_angle);

                if first_second_delta < 1.0 {
                    // Ignore players with only a tiny adjustment in second angle
                    continue;
                }

                let ratio = first_second_delta / first_third_delta.max(f32::EPSILON);

                if first_third_delta <= MAX_FIRST_THIRD_ANGLE_DELTA && ratio > MIN_ANGLE_DIFF_RATIO
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
        self.ticks.insert(0, data);
        self.ticks.truncate(3);
        Ok(detections)
    }

    fn handled_messages(&self) -> Result<Vec<tf_demo_parser::MessageType>, bool> {
        Ok(vec![tf_demo_parser::MessageType::GameEvent])
    }

    fn on_message(
        &mut self,
        message: &tf_demo_parser::demo::message::Message,
        state: &CheatAnalyserState,
        _parser_state: &ParserState,
        tick: tf_demo_parser::demo::data::DemoTick,
    ) -> Result<Vec<Detection>, Error> {
        match message {
            tf_demo_parser::demo::message::Message::GameEvent(
                tf_demo_parser::demo::message::GameEventMessage { event, .. },
            ) => match event {
                tf_demo_parser::demo::gamevent::GameEvent::PlayerSpawn(spawn) => {
                    if let Some(id) = state.get_id64_from_userid(spawn.user_id.into()){
                        self.last_spawns.insert(id, tick.into());
                    }
                }
                _ => (),
            },
            _ => (),
        }
        Ok(vec![])
    }
}
