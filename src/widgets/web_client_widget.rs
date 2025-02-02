use std::collections::HashMap;

use common::slc_commands::{ClientCommand, ServerType};
use crossbeam_channel::Sender;
use egui::{Label, Sense, Ui, Widget};
use wg_2024::{network::NodeId, packet::Packet};

#[derive(Clone, Debug)]
pub struct ClientWidget {
    id: NodeId,
    command_ch: Sender<ClientCommand>,
    servers_types: HashMap<NodeId, ServerType>,
    id_input: String,
    list_of_files: HashMap<NodeId, Vec<String>>,
    chat_server_id: String,
    list_of_connected_users: Vec<NodeId>,
}

impl ClientWidget {
    pub fn new(
        id: NodeId,
        command_ch: Sender<ClientCommand>,
    ) -> Self {
        Self {
            id,
            command_ch,
            servers_types: HashMap::default(),
            id_input: String::default(),
            list_of_files: HashMap::default(),
            chat_server_id: String::default(),
            list_of_connected_users: Vec::default(),
        }
    }

    pub fn add_neighbor(&mut self, neighbor_id: u8, neighbor_ch: Sender<Packet>) {
        self.command_ch
            .send(ClientCommand::AddSender(neighbor_id, neighbor_ch)).expect("msg not sent");
    }

    pub fn remove_neighbor(&mut self, neighbor_id: u8) {
        self.command_ch
            .send(ClientCommand::RemoveSender(neighbor_id)).expect("msg not sent");
    }

    pub fn add_list_of_files(&mut self, server_id: NodeId, files: Vec<String>) {
        self.list_of_files.insert(server_id, files);
    }

    pub fn add_server_type(&mut self, server_types: HashMap<NodeId, ServerType>) {
        self.servers_types = server_types;
    }

    pub fn add_connected_users(&mut self, users_id: Vec<NodeId>) {
        self.list_of_connected_users = users_id;
    }

    pub fn get_id(&self) -> NodeId {
        self.id
    }
}


impl Widget for &mut ClientWidget {
    fn ui(self, ui: &mut Ui) -> egui::Response {
        ui.vertical(|ui| {
            ui.label(format!("Client {}", self.id));

            // Send command to ask for servers types
            ui.label("Ask for Server types");
            if ui.button("Send").clicked() {
                let cmd = ClientCommand::AskServersTypes;
                self.command_ch.send(cmd).expect("msg not sent");
            }

            ui.label("Servers types:");
            for (id, srv_type) in &self.servers_types {
                ui.label(format!("Server {}: {:?}", id, srv_type));
            }

            ui.separator();

            // Send command to ask for files
            ui.label("Ask for Server files");
            ui.text_edit_singleline(&mut self.id_input);
            if ui.button("Send").clicked() {
                let cmd = ClientCommand::AskListOfFiles(self.id_input.parse().unwrap());
                self.command_ch.send(cmd).expect("msg not sent");
            }

            ui.separator();
            ui.label("Received files:");
            for (server_id, server_files) in &self.list_of_files {
                ui.label(format!("Server {}: ", server_id));
                for file in server_files {
                    let file_name = file.split("/").last().unwrap().to_string();
                    if ui.add(Label::new(file_name).sense(Sense::click())).clicked() {
                        let cmd = ClientCommand::RequestFile(file.to_string(), *server_id);
                        self.command_ch.send(cmd).expect("msg not sent");
                    }

                }
            }

            // Button to connect to chat server
            ui.separator();
            ui.label("Connect to chat server");
            ui.text_edit_singleline(&mut self.chat_server_id);
            // TODO: Add validation for the input (also for other inputs)
            if ui.button("Connect").clicked() {
                let cmd = ClientCommand::ConnectToChatServer(self.chat_server_id.parse().unwrap());
                self.command_ch.send(cmd).expect("msg not sent");
            }

        }).response
    }
}