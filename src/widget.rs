use crossbeam_channel::{Receiver, Sender};
use wg_2024::{controller::{DroneCommand, DroneEvent}, network::NodeId};
use common::slc_commands::{ClientCommand, ClientEvent, ServerCommand, ServerEvent, ServerType};
use egui::{Button, Ui};
use std::collections::HashMap;


pub trait Drawable {
    fn draw(&mut self, ui: &mut Ui);
}

#[derive(Clone)]
pub struct Widget {
    id: NodeId,
    node_type: NodeType,
}

#[derive(Clone)]
pub enum NodeType {
    Drone { command_ch: Sender<DroneCommand>, event_ch: Receiver<DroneEvent> },
    Client { command_ch: Sender<ClientCommand>, event_ch: Receiver<ClientEvent> },
    Server { command_ch: Sender<ServerCommand>, event_ch: Receiver<ServerEvent> },
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
    pdr_input: String,
}

impl DroneWidget {
    pub fn new(id: NodeId, command_ch: Sender<DroneCommand>, event_ch: Receiver<DroneEvent>) -> Self {
        Self {
            id,
            command_ch,
            event_ch,
            pdr_input: String::default(),
        }
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
        let red_btn = ui.add(Button::new("Crash").fill(egui::Color32::RED));
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
    list_of_files: Vec<String>,
}

impl ClientWidget {
    pub fn new(id: NodeId, command_ch: Sender<ClientCommand>, event_ch: Receiver<ClientEvent>) -> Self {
        Self {
            id,
            command_ch,
            event_ch,
            servers_types: HashMap::default(),
            id_input: String::default(),
            list_of_files: Vec::default(),
        }
    }

    pub fn get_id(&self) -> NodeId {
        self.id
    }
}

impl Drawable for ClientWidget {
    fn draw(&mut self, ui: &mut Ui) {
        // Draw the client widget
        ui.label(format!("Client {}", self.id));

        // Send command to ask for servers types
        ui.label("Ask for Server types");
        if ui.button("Send").clicked() {
            let cmd = ClientCommand::AskServersTypes;
            self.command_ch.send(cmd);
        }

        while let Ok(event) = self.event_ch.try_recv() {
             match event {
                ClientEvent::ServersTypes(types) => {
                    self.servers_types = types;
                }
                _ => {}
            }
            
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
        while let Ok(event) = self.event_ch.try_recv() {
            match event {
                ClientEvent::ListOfFiles(files, id) => {
                    println!("Received files from server {}: {:?}", id, files);
                    self.list_of_files = files;
                }
                _ => {}
            }
        }

        for f in &self.list_of_files {
            ui.label(f);
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
    pub fn new(id: NodeId, command_ch: Sender<ServerCommand>, event_ch: Receiver<ServerEvent>) -> Self {
        Self {
            id,
            command_ch,
            event_ch,
        }
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
