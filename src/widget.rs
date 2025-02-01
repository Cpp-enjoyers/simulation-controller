use common::slc_commands::{ClientCommand, ServerCommand, ServerType};
use crossbeam_channel::Sender;
use egui::{Button, Color32, Label, RichText, Sense, Ui, Widget};
use std::collections::HashMap;
use wg_2024::{
    controller::DroneCommand,
    network::NodeId,
    packet::Packet,
};


#[derive(Clone, Debug)]
pub enum WidgetType {
    Drone(DroneWidget),
    Client(ClientWidget),
    Server(ServerWidget),
}

#[derive(Clone, Debug)]
pub struct DroneWidget {
    id: NodeId,
    command_ch: Sender<DroneCommand>,
    pdr_input: String,
}

impl DroneWidget {
    pub fn new(
        id: NodeId,
        command_ch: Sender<DroneCommand>,
    ) -> Self {
        Self {
            id,
            command_ch,
            pdr_input: String::default(),
        }
    }

    pub fn add_neighbor(&mut self, neighbor_id: u8, neighbor_ch: Sender<Packet>) {
        self.command_ch
            .send(DroneCommand::AddSender(neighbor_id, neighbor_ch));
    }

    pub fn remove_neighbor(&mut self, neighbor_id: u8) {
        self.command_ch
            .send(DroneCommand::RemoveSender(neighbor_id));
    }

    pub fn get_id(&self) -> NodeId {
        self.id
    }
}


impl Widget for &mut DroneWidget {
    fn ui(self, ui: &mut Ui) -> egui::Response {
        ui.vertical(|ui| {
            ui.label(format!("Drone {}", self.id));
            ui.label("Change PDR");
            ui.text_edit_singleline(&mut self.pdr_input);
            if ui.button("Send").clicked() {
                let cmd = DroneCommand::SetPacketDropRate(self.pdr_input.parse().unwrap());
                self.command_ch.send(cmd);
            }

            ui.separator();
            ui.label("Crash the drone");
            let red_btn =
                ui.add(Button::new(RichText::new("Crash").color(Color32::BLACK)).fill(Color32::RED));
            if red_btn.clicked() {
                self.command_ch.send(DroneCommand::Crash);
            }
        }).response
    }
}

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
            .send(ClientCommand::AddSender(neighbor_id, neighbor_ch));
    }

    pub fn remove_neighbor(&mut self, neighbor_id: u8) {
        self.command_ch
            .send(ClientCommand::RemoveSender(neighbor_id));
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
                self.command_ch.send(cmd);
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
                self.command_ch.send(cmd);
            }

            ui.separator();
            ui.label("Received files:");
            for (server_id, server_files) in &self.list_of_files {
                ui.label(format!("Server {}: ", server_id));
                for file in server_files {
                    let file_name = file.split("/").last().unwrap().to_string();
                    if ui.add(Label::new(file_name).sense(Sense::click())).clicked() {
                        let cmd = ClientCommand::RequestFile(file.to_string(), *server_id);
                        self.command_ch.send(cmd);
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
                self.command_ch.send(cmd);
            }

        }).response
    }
}

#[derive(Clone, Debug)]
pub struct ServerWidget {
    pub id: NodeId,
    pub command_ch: Sender<ServerCommand>,
}

impl ServerWidget {
    pub fn new(
        id: NodeId,
        command_ch: Sender<ServerCommand>,
    ) -> Self {
        Self {
            id,
            command_ch,
        }
    }

    pub fn add_neighbor(&mut self, neighbor_id: u8, neighbor_ch: Sender<Packet>) {
        self.command_ch
            .send(ServerCommand::AddSender(neighbor_id, neighbor_ch));
    }

    pub fn remove_neighbor(&mut self, neighbor_id: u8) {
        self.command_ch
            .send(ServerCommand::RemoveSender(neighbor_id));
    }

    pub fn get_id(&self) -> NodeId {
        self.id
    }
}

impl Widget for &mut ServerWidget {
    fn ui(self, ui: &mut Ui) -> egui::Response {
        ui.vertical_centered(|ui| {
            ui.label(format!("Server {}", self.id));
            if ui.button("test").clicked() {
                println!("test");
            }
        }).response
    }
}
