use std::collections::{BTreeMap, HashMap};

use tf_demo_parser::{
    MessageType, ParserState, ReadResult, Stream,
    demo::{
        gamevent::GameEvent,
        message::{EntityId, GameEventMessage, Message, PacketEntity},
        packet::{
            datatable::{ParseSendTable, ServerClass, ServerClassName},
            stringtable::StringTableEntry,
        },
        parser::{MessageHandler, analyser::UserId},
        sendprop::SendProp,
    },
};

use crate::data::Class;

fn parse_integer_prop<F>(
    packet: &PacketEntity,
    table: &str,
    name: &str,
    parser_state: &ParserState,
    handler: F,
) where
    F: FnOnce(u32),
{
    use tf_demo_parser::demo::sendprop::SendPropValue;

    if let Some(SendProp {
        value: SendPropValue::Integer(val),
        ..
    }) = packet.get_prop_by_name(table, name, parser_state)
    {
        handler(val as u32);
    }
}

#[derive(Default, Debug)]
pub struct DemoAnalyzer {
    state: DemoState,
    class_names: Vec<ServerClassName>,
    entity_to_user: HashMap<EntityId, UserId>,

    current_tick: u32,
}

impl DemoAnalyzer {
    fn player(&mut self, user_id: UserId) -> &mut PlayerInfo {
        self.state.players.entry(user_id).or_default()
    }

    fn player_by_entity(&mut self, entity_id: EntityId) -> Option<&mut PlayerInfo> {
        if let Some(user_id) = self.entity_to_user.get(&entity_id) {
            Some(self.player(*user_id))
        } else {
            None
        }
    }

    pub fn handle_entity(&mut self, entity: &PacketEntity, parser_state: &ParserState) {
        // const OUTER: SendPropIdentifier =
        //     SendPropIdentifier::new("DT_AttributeContainer", "m_hOuter");

        let Some(class_name) = self.class_names.get(usize::from(entity.server_class)) else {
            return;
        };

        // for prop in &entity.props {
        //     if prop.identifier == OUTER {
        //         let outer = i64::try_from(&prop.value).unwrap_or_default();
        //         self.state
        //             .outer_map
        //             .insert(Handle(outer), entity.entity_index);
        //     }
        // }

        // println!("{class_name}");
        match class_name.as_str() {
            "CTFPlayer" => self.handle_player_entity(entity, parser_state),
            "CTFPlayerResource" => self.handle_player_resource(entity, parser_state),
            // "CWorld" => self.handle_world_entity(entity, parser_state),
            // "CObjectSentrygun" => self.handle_sentry_entity(entity, parser_state),
            // "CObjectDispenser" => self.handle_dispenser_entity(entity, parser_state),
            // "CObjectTeleporter" => self.handle_teleporter_entity(entity, parser_state),
            // "CFuncTrackTrain" => self.handle_train_entity(entity, parser_state),
            // _ if class_name.starts_with("CTFProjectile_")
            //     || class_name.as_str() == "CTFGrenadePipebombProjectile" =>
            // {
            //     self.handle_projectile_entity(entity, parser_state)
            // }
            _ => {}
        }
    }

    pub fn handle_player_entity(&mut self, packet: &PacketEntity, parser_state: &ParserState) {
        let Some(player) = self.player_by_entity(packet.entity_index) else {
            return;
        };

        parse_integer_prop(
            packet,
            "DT_TFPlayerScoringDataExclusive",
            "m_iKills",
            parser_state,
            |kills| {
                player.kills = player.kills.max(kills);
            },
        );
        parse_integer_prop(
            packet,
            "DT_TFPlayerScoringDataExclusive",
            "m_iDeaths",
            parser_state,
            |deaths| {
                player.deaths = player.deaths.max(deaths);
            },
        );
    }

    pub fn handle_player_resource(&mut self, entity: &PacketEntity, parser_state: &ParserState) {
        for prop in entity.props(parser_state) {
            if let Some((table_name, prop_name)) = prop.identifier.names() {
                if let Ok(player_id) = u32::from_str_radix(prop_name.as_str(), 10) {
                    let entity_id = EntityId::from(player_id);
                    if let Some(player) = self.player_by_entity(entity_id) {
                        match table_name.as_str() {
                            // "m_iTeam" => {
                            //     player.team =
                            //         Team::new(i64::try_from(&prop.value).unwrap_or_default())
                            // }
                            // "m_iMaxHealth" => {
                            //     player.max_health =
                            //         i64::try_from(&prop.value).unwrap_or_default() as u16
                            // }
                            "m_iPlayerClass" => {
                                player.class = Class::from_int(
                                    i64::try_from(&prop.value).unwrap_or_default() as u16,
                                );
                            }
                            // "m_iChargeLevel" => {
                            //     player.charge = i64::try_from(&prop.value).unwrap_or_default() as u8
                            // }
                            // "m_iPing" => {
                            //     player.ping = i64::try_from(&prop.value).unwrap_or_default() as u16
                            // }
                            _ => {}
                        }
                    }
                }
            }
        }
    }

    fn parse_user_info(
        &mut self,
        index: usize,
        text: Option<&str>,
        data: Option<Stream>,
    ) -> ReadResult<()> {
        if let Some(user_info) =
            tf_demo_parser::demo::data::UserInfo::parse_from_string_table(index as u16, text, data)?
        {
            self.entity_to_user
                .insert(user_info.entity_id, user_info.player_info.user_id);
            let player = self.player(user_info.player_info.user_id);
            player.entity = user_info.entity_id;
            player.name = user_info.player_info.name;
            player.user_id = user_info.player_info.user_id;
            player.is_bot = user_info.player_info.is_fake_player != 0;
        }

        Ok(())
    }
}

impl MessageHandler for DemoAnalyzer {
    type Output = DemoState;

    fn does_handle(message_type: tf_demo_parser::MessageType) -> bool {
        matches!(
            message_type,
            MessageType::PacketEntities | MessageType::GameEvent | MessageType::ServerInfo
        )
    }

    fn handle_message(
        &mut self,
        message: &Message,
        tick: tf_demo_parser::demo::data::DemoTick,
        parser_state: &tf_demo_parser::ParserState,
    ) {
        if tick > self.current_tick {
            let delta = u32::from(tick - self.current_tick);
            self.current_tick = tick.into();
            for player in self.state.players.values_mut() {
                player.add_playtime(player.class, delta as usize);
            }
        }

        match message {
            Message::PacketEntities(message) => {
                for entity in &message.entities {
                    self.handle_entity(entity, parser_state);
                }
            }
            // Message::ServerInfo(message) => {
            //     self.state.interval_per_tick = message.interval_per_tick
            // }
            Message::GameEvent(GameEventMessage { event, .. }) => match event {
                GameEvent::PlayerChangeClass(change_class) => {
                    let player = self.player(UserId::from(change_class.user_id));
                    player.class = Class::from_int(change_class.class);
                }
                _ => {}
            },
            _ => {}
        }
    }

    fn handle_data_tables(
        &mut self,
        _parse_tables: &[ParseSendTable],
        server_classes: &[ServerClass],
        _parser_state: &ParserState,
    ) {
        self.class_names = server_classes
            .iter()
            .map(|class| &class.name)
            .cloned()
            .collect();
    }

    fn handle_string_entry(
        &mut self,
        table: &str,
        index: usize,
        entry: &StringTableEntry,
        _parser_state: &ParserState,
    ) {
        if table == "userinfo" {
            let _ = self.parse_user_info(
                index,
                entry.text.as_ref().map(|s| s.as_ref()),
                entry.extra_data.as_ref().map(|data| data.data.clone()),
            );
        }
    }

    fn into_output(self, _state: &tf_demo_parser::ParserState) -> Self::Output {
        self.state
    }
}

#[derive(Default, Debug)]
pub struct DemoState {
    pub players: BTreeMap<UserId, PlayerInfo>,
}

#[derive(Debug)]
pub struct PlayerInfo {
    pub entity: EntityId,
    pub name: String,
    pub playtime_per_class: HashMap<Class, usize>,
    pub class: Class,
    pub is_bot: bool,

    pub user_id: UserId,

    pub deaths: u32,
    pub kills: u32,
}

impl PlayerInfo {
    pub fn add_playtime(&mut self, class: Class, time: usize) {
        *self.playtime_per_class.entry(class).or_default() += time;
    }

    pub fn most_played_class(&self) -> Class {
        self.playtime_per_class
            .iter()
            .filter(|&(class, _)| *class != Class::Other)
            .max_by_key(|&(_, &time)| time)
            .map(|(&class, _)| class)
            .unwrap_or(self.class)
    }
}

impl Default for PlayerInfo {
    fn default() -> Self {
        Self {
            entity: EntityId::from(0usize),
            user_id: UserId::default(),
            name: "<unknown>".to_string(),
            playtime_per_class: HashMap::new(),
            is_bot: true,
            class: Class::Other,
            kills: 0,
            deaths: 0,
        }
    }
}
