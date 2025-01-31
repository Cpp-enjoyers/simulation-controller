use common::slc_commands::{ClientCommand, ClientEvent, ServerCommand, ServerEvent, ServerType};
use crossbeam_channel::{Receiver, Sender};
use egui::{Button, Color32, Label, RichText, Sense, Ui};
use std::{collections::HashMap, fs::File, io::Write, path::Path};
use wg_2024::{
    controller::{DroneCommand, DroneEvent},
    network::NodeId,
    packet::Packet,
};

pub trait Drawable {
    fn draw(&mut self, ui: &mut Ui);
}

#[derive(Clone)]
pub enum WidgetType {
    Drone(DroneWidget),
    Client(ClientWidget),
    Server(ServerWidget),
}

#[derive(Clone)]
pub struct DroneWidget {
    id: NodeId,
    command_ch: Sender<DroneCommand>,
    event_ch: Receiver<DroneEvent>,
    send_ch: Sender<Packet>,
    recv_ch: Receiver<Packet>,
    pdr_input: String,
}

impl DroneWidget {
    pub fn new(
        id: NodeId,
        command_ch: Sender<DroneCommand>,
        event_ch: Receiver<DroneEvent>,
        send_ch: Sender<Packet>,
        recv_ch: Receiver<Packet>,
    ) -> Self {
        Self {
            id,
            command_ch,
            event_ch,
            send_ch,
            recv_ch,
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

impl Drawable for DroneWidget {
    fn draw(&mut self, ui: &mut Ui) {
        // Draw the drone widget
        ui.label(format!("Drone {}", self.id));

        // Send command to change the pdr
        ui.label("Change PDR");
        ui.text_edit_singleline(&mut self.pdr_input);
        if ui.button("Send").clicked() {
            let cmd = DroneCommand::SetPacketDropRate(self.pdr_input.parse().unwrap());
            self.command_ch.send(cmd);
        }

        ui.separator();
        // Make the current drone crash
        ui.label("Crash the drone");
        let red_btn =
            ui.add(Button::new(RichText::new("Crash").color(Color32::BLACK)).fill(Color32::RED));
        if red_btn.clicked() {
            self.command_ch.send(DroneCommand::Crash);
        }
    }
}

#[derive(Clone)]
pub struct ClientWidget {
    id: NodeId,
    command_ch: Sender<ClientCommand>,
    event_ch: Receiver<ClientEvent>,
    servers_types: HashMap<NodeId, ServerType>,
    id_input: String,
    list_of_files: HashMap<NodeId, Vec<String>>,
}

impl ClientWidget {
    pub fn new(
        id: NodeId,
        command_ch: Sender<ClientCommand>,
        event_ch: Receiver<ClientEvent>,
    ) -> Self {
        Self {
            id,
            command_ch,
            event_ch,
            servers_types: HashMap::default(),
            id_input: String::default(),
            list_of_files: HashMap::default(),
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

    pub fn get_id(&self) -> NodeId {
        self.id
    }

    pub fn handle_event(&mut self, event: ClientEvent) {
        println!("Client {} received event: {:?}", self.id, event);
        match event {
            ClientEvent::PacketSent(packet) => {},
            ClientEvent::Shortcut(packet) => {},
            ClientEvent::ClientsConnectedToChatServer(items) => {},
            ClientEvent::ListOfFiles(files, id) => {
                self.list_of_files.insert(id, files);
            },
            ClientEvent::FileFromClient(file_content, server_id) => {
                println!("Client {} received file from server {}: {:?}", self.id, server_id, file_content);
                // let folder = Path::new("tmp");

                // if !folder.exists() {
                //     std::fs::create_dir_all(folder).unwrap();
                // }

                // let file_path = folder.join("index.html");
                // let mut file = File::create(&file_path).unwrap();
                // file.write_all(file_content.as_bytes()).unwrap();

                // if webbrowser::open(file_path.to_str().unwrap()).is_err() {
                //     println!("Failed to open the file in the browser");
                // }
            },
            ClientEvent::ServersTypes(srv_types) => {
                self.servers_types = srv_types;
            },
            ClientEvent::WrongClientId => {},
            ClientEvent::UnsupportedRequest => {},
        }
    }
}

impl Drawable for ClientWidget {
    fn draw(&mut self, ui: &mut Ui) {

        if let Ok(event) = self.event_ch.try_recv() {
            self.handle_event(event);
        }

        // Draw the client widget
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
    }
}

#[derive(Clone)]
pub struct ServerWidget {
    pub id: NodeId,
    pub command_ch: Sender<ServerCommand>,
    pub event_ch: Receiver<ServerEvent>,
}

impl ServerWidget {
    pub fn new(
        id: NodeId,
        command_ch: Sender<ServerCommand>,
        event_ch: Receiver<ServerEvent>,
    ) -> Self {
        Self {
            id,
            command_ch,
            event_ch,
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

impl Drawable for ServerWidget {
    fn draw(&mut self, ui: &mut Ui) {
        // Draw the server widget
        ui.label(format!("Server {}", self.id));
    }
}
