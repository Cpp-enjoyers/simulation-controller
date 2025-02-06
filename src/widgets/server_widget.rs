use common::slc_commands::ServerCommand;
use crossbeam_channel::Sender;
use egui::{Ui, Widget};
use wg_2024::{network::NodeId, packet::Packet};

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
            .send(ServerCommand::AddSender(neighbor_id, neighbor_ch)).expect("msg not sent");
    }

    pub fn remove_neighbor(&self, neighbor_id: u8) {
        self.command_ch
            .send(ServerCommand::RemoveSender(neighbor_id)).expect("msg not sent");
    }

    pub fn get_id(&self) -> NodeId {
        self.id
    }
}

impl Widget for ServerWidget {
    fn ui(mut self, ui: &mut Ui) -> egui::Response {
        ui.vertical_centered(|ui| {
            ui.label(format!("Server {}", self.id));
            if ui.button("test").clicked() {
                println!("test");
            }
        }).response
    }
}