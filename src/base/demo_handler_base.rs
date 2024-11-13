// This file is a modified version of DemoHandler.
// https://github.com/demostf/parser/blob/master/src/demo/parser/gamestateanalyser.rs
// This version makes CheatDemoHandler::analyser public (that's literally it)

use tf_demo_parser::demo::message::Message;
use tf_demo_parser::demo::packet::datatable::{ParseSendTable, ServerClass};
use tf_demo_parser::demo::packet::stringtable::{StringTable, StringTableEntry};
use tf_demo_parser::demo::packet::Packet;
use tf_demo_parser::demo::parser::handler::BorrowMessageHandler;
use tf_demo_parser::demo::parser::{MessageHandler, NullHandler};
use tf_demo_parser::{ParserState, Result};

use tf_demo_parser::demo::data::{DemoTick, ServerTick};
use tf_demo_parser::demo::header::Header;
use std::borrow::Cow;

#[derive(Clone)]
pub struct CheatDemoHandler<'a, T: MessageHandler> {
    pub server_tick: ServerTick,
    pub demo_tick: DemoTick,
    pub string_table_names: Vec<Cow<'a, str>>,
    pub analyser: T,
    pub state_handler: ParserState,
}

impl<'a> CheatDemoHandler<'a, NullHandler> {
    pub fn new() -> Self {
        Self::parse_all_with_analyser(NullHandler)
    }
}

impl<'a> Default for CheatDemoHandler<'a, NullHandler> {
    fn default() -> Self {
        CheatDemoHandler::new()
    }
}

impl<'a, T: MessageHandler> CheatDemoHandler<'a, T> {
    pub fn with_analyser(analyser: T) -> Self {
        let state_handler = ParserState::new(24, T::does_handle, false);

        CheatDemoHandler {
            server_tick: ServerTick::default(),
            demo_tick: DemoTick::default(),
            string_table_names: Vec::new(),
            analyser,
            state_handler,
        }
    }
    pub fn parse_all_with_analyser(analyser: T) -> Self {
        let state_handler = ParserState::new(24, T::does_handle, true);

        CheatDemoHandler {
            server_tick: ServerTick::default(),
            demo_tick: DemoTick::default(),
            string_table_names: Vec::new(),
            analyser,
            state_handler,
        }
    }

    pub fn handle_header(&mut self, header: &Header) {
        self.state_handler.protocol_version = header.protocol;
        self.analyser.handle_header(header);
    }

    pub fn handle_packet(&mut self, packet: Packet<'a>) -> Result<()> {
        match packet {
            Packet::DataTables(packet) => {
                self.handle_data_table(packet.tables, packet.server_classes)?;
            }
            Packet::StringTables(packet) => {
                for table in packet.tables.into_iter() {
                    self.handle_string_table(table)
                }
            }
            Packet::Message(packet) | Packet::Signon(packet) => {
                self.analyser
                    .handle_packet_meta(packet.tick, &packet.meta, &self.state_handler);
                for message in packet.messages {
                    match message {
                        Message::NetTick(message) => {
                            self.server_tick = message.tick;
                            self.handle_message(Message::NetTick(message), packet.tick)
                        }
                        Message::CreateStringTable(message) => {
                            self.handle_string_table(message.table)
                        }
                        Message::UpdateStringTable(message) => {
                            self.handle_table_update(message.table_id, message.entries)
                        }
                        Message::PacketEntities(msg) => {
                            self.handle_message(Message::PacketEntities(msg), packet.tick)
                        }
                        message => self.handle_message(message, packet.tick),
                    }
                }
            }
            _ => {}
        };
        Ok(())
    }

    fn handle_string_table(&mut self, table: StringTable<'a>) {
        self.state_handler
            .handle_string_table_meta(table.get_table_meta());
        for (entry_index, entry) in table.entries.into_iter() {
            let entry_index = entry_index as usize;
            self.state_handler
                .handle_string_entry(&table.name, entry_index, &entry);
            self.analyser.handle_string_entry(
                &table.name,
                entry_index,
                &entry,
                &self.state_handler,
            );
        }

        self.string_table_names.push(table.name);
    }

    fn handle_table_update(&mut self, table_id: u8, entries: Vec<(u16, StringTableEntry<'a>)>) {
        if let Some(table_name) = self.string_table_names.get(table_id as usize) {
            for (index, entry) in entries {
                let index = index as usize;
                self.state_handler
                    .handle_string_entry(table_name, index, &entry);
                self.analyser
                    .handle_string_entry(table_name, index, &entry, &self.state_handler);
            }
        }
    }

    fn handle_data_table(
        &mut self,
        send_tables: Vec<ParseSendTable>,
        server_classes: Vec<ServerClass>,
    ) -> Result<()> {
        self.analyser
            .handle_data_tables(&send_tables, &server_classes, &self.state_handler);
        self.state_handler
            .handle_data_table(&send_tables, server_classes)
    }

    pub fn handle_message(&mut self, message: Message<'a>, tick: DemoTick) {
        let message_type = message.get_message_type();
        if T::does_handle(message_type) {
            self.analyser
                .handle_message(&message, tick, &self.state_handler);
        }
        self.state_handler.handle_message(message, tick);
    }

    pub fn into_output(self) -> T::Output {
        self.analyser.into_output(&self.state_handler)
    }

    pub fn get_parser_state(&self) -> &ParserState {
        &self.state_handler
    }
}

impl<T: MessageHandler + BorrowMessageHandler> CheatDemoHandler<'_, T> {
    pub fn borrow_output(&self) -> &T::Output {
        self.analyser.borrow_output(&self.state_handler)
    }
}
