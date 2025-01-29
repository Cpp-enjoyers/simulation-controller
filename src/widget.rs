use crossbeam_channel::{Receiver, Sender};
use wg_2024::{controller::{DroneCommand, DroneEvent}, network::NodeId};
use common::slc_commands::{ClientCommand, ClientEvent, ServerCommand, ServerEvent};
use egui::Ui;


pub trait Drawable {
    fn draw(&self, ui: &mut Ui);
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
    pub id: NodeId,
    pub command_ch: Sender<DroneCommand>,
    pub event_ch: Receiver<DroneEvent>,
}

impl Drawable for DroneWidget {
    fn draw(&self, ui: &mut Ui) {
        // Draw the drone widget
        ui.label(format!("Drone {}", self.id));
    }
}

#[derive(Clone)]
pub struct ClientWidget {
    pub id: NodeId,
    pub command_ch: Sender<ClientCommand>,
    pub event_ch: Receiver<ClientEvent>,
}

impl Drawable for ClientWidget {
    fn draw(&self, ui: &mut Ui) {
        // Draw the client widget
        ui.label(format!("Client {}", self.id));
    }
}

#[derive(Clone)]
pub struct ServerWidget {
    pub id: NodeId,
    pub command_ch: Sender<ServerCommand>,
    pub event_ch: Receiver<ServerEvent>,
}

impl Drawable for ServerWidget {
    fn draw(&self, ui: &mut Ui) {
        // Draw the server widget
        ui.label(format!("Server {}", self.id));
    }
}
