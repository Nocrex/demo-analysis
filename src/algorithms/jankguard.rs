use std::collections::HashMap;

use crate::base::cheat_analyser_base::{CheatAnalyserState, Player, PlayerState};
use steamid_ng::SteamID;

const TELEPORT_DIST: f32 = 256.0;

#[derive(Default)]
pub struct JankGuard {
    last_spawns: HashMap<u64, u32>,
    last_teleport: HashMap<u64, u32>,
    last_fire: HashMap<u64, u32>,

    last_tick: HashMap<u64, Player>,
}

impl JankGuard {
    pub fn teleported(&self, player: &u64, tick: u32) -> u32 {
        tick - self.last_teleport.get(player).cloned().unwrap_or_default()
    }

    pub fn spawned(&self, player: &u64, tick: u32) -> u32 {
        tick - self.last_spawns.get(player).cloned().unwrap_or_default()
    }

    pub fn fired(&self, player: &u64, tick: u32) -> u32 {
        tick - self.last_fire.get(player).cloned().unwrap_or_default()
    }

    pub fn handled_messages(&self) -> Result<Vec<tf_demo_parser::MessageType>, bool> {
        Ok(vec![
            tf_demo_parser::MessageType::GameEvent,
            tf_demo_parser::MessageType::TempEntities,
        ])
    }

    pub fn on_message(
        &mut self,
        message: &tf_demo_parser::demo::message::Message,
        state: &CheatAnalyserState,
        _parser_state: &tf_demo_parser::ParserState,
        tick: tf_demo_parser::demo::data::DemoTick,
    ) {
        match message {
            tf_demo_parser::demo::message::Message::GameEvent(
                tf_demo_parser::demo::message::GameEventMessage { event, .. },
            ) => match event {
                tf_demo_parser::demo::gamevent::GameEvent::PlayerSpawn(spawn) => {
                    if let Some(id) = state.get_id64_from_userid(spawn.user_id.into()) {
                        self.last_spawns.insert(id, tick.into());
                    }
                }
                tf_demo_parser::demo::gamevent::GameEvent::PostInventoryApplication(app) => {
                    if let Some(id) = state.get_id64_from_userid(app.user_id.into()) {
                        self.last_spawns.insert(id, tick.into());
                    }
                }
                tf_demo_parser::demo::gamevent::GameEvent::PlayerTeleported(tele) => {
                    if let Some(id) = state.get_id64_from_userid(tele.user_id.into()) {
                        self.last_teleport.insert(id, tick.into());
                    }
                }
                _ => (),
            },
            // Try to find firing events through tracers and player animations, thx to megascatterbomb for this snippet
            tf_demo_parser::demo::message::Message::TempEntities(msg) => {
                for event in &msg.events {
                    match u16::from(event.class_id) {
                        //Bullet tracer
                        152 => {
                            for prop in &event.props {
                                match prop.identifier.names() {
                                    Some((table_name, prop_name)) => {
                                        if table_name == "DT_TEFireBullets"
                                            && prop_name == "m_iPlayer"
                                        {
                                            let ent_id: i64 = match prop.value {
                                        tf_demo_parser::demo::sendprop::SendPropValue::Integer(x) => x.try_into().unwrap_or_default(),
                                        _ => {continue}
                                    };
                                            let entity_id =
                                                tf_demo_parser::demo::message::EntityId::from(
                                                    ent_id as u32,
                                                );
                                            match state
                                                .entid_to_userid
                                                .get(&entity_id)
                                                .and_then(|userid| state.userid_to_id64.get(userid))
                                            {
                                                Some(id64) => {
                                                    self.last_fire.insert(*id64, tick.into());
                                                }
                                                None => continue,
                                            };
                                        }
                                    }
                                    None => {}
                                }
                            }
                        }
                        // Animation
                        165 => {
                            for prop in &event.props {
                                match prop.identifier.names() {
                                    Some((table_name, prop_name)) => {
                                        if table_name == "DT_TEPlayerAnimEvent"
                                            && prop_name == "m_hPlayer"
                                        {
                                            let handle_id: u32 = match prop.value {
                                        tf_demo_parser::demo::sendprop::SendPropValue::Integer(x) => x.try_into().unwrap_or_default(),
                                        _ => {continue}
                                    };
                                            let entity_id = crate::util::handle_to_entid(handle_id);
                                            match state
                                                .entid_to_userid
                                                .get(&entity_id)
                                                .and_then(|userid| state.userid_to_id64.get(userid))
                                            {
                                                Some(id64) => {
                                                    self.last_fire.insert(*id64, tick.into());
                                                }
                                                None => continue,
                                            };
                                        }
                                    }
                                    None => {}
                                }
                            }
                        }
                        _ => continue,
                    }
                }
            }
            _ => (),
        }
    }

    pub fn on_tick(&mut self, state: &CheatAnalyserState) {
        for player in state.players.iter().filter(|p| {
            p.in_pvs
                && p.state == PlayerState::Alive
                && p.info.as_ref().is_some_and(|info| info.steam_id != "BOT")
        }) {
            let info = match &player.info {
                Some(info) => info,
                None => continue,
            };

            let steam_id: u64 = u64::from(SteamID::from_steam3(&info.steam_id).unwrap());

            let prev_player = self.last_tick.get(&steam_id);

            if prev_player.as_ref().is_some_and(|p| {
                // Ignore players that just moved more than 256 HUs in a single tick (teleport)
                let diff = p.position - player.position;
                let sq_len = diff.x.powi(2) + diff.y.powi(2) + diff.z.powi(2);
                sq_len > TELEPORT_DIST.powi(2)
            }) {
                continue;
            }
        }
    }
}
