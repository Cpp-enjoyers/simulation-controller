use common::slc_commands::ChatClientCommand;
use crossbeam_channel::Sender;
use egui::Widget;
use wg_2024::{network::NodeId, packet::Packet};


#[derive(Debug, Clone)]
pub struct ChatClientWidget {
    id: NodeId,
    command_ch: Sender<ChatClientCommand>,
}

impl ChatClientWidget {
    pub fn new(id: NodeId, command_ch: Sender<ChatClientCommand>) -> Self {
        Self { id, command_ch }
    }

    pub fn add_neighbor(&mut self, neighbor_id: u8, neighbor_ch: Sender<Packet>) {
        self.command_ch
            .send(ChatClientCommand::AddSender(neighbor_id, neighbor_ch)).expect("msg not sent");
    }

    pub fn remove_neighbor(&mut self, neighbor_id: u8) {
        self.command_ch
            .send(ChatClientCommand::RemoveSender(neighbor_id)).expect("msg not sent");
    }

    pub fn get_id(&self) -> NodeId {
        self.id
    }
}

impl Widget for &mut ChatClientWidget {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        ui.vertical_centered(|ui| {
            ui.label(format!("Chat Client {}", self.id));
        }).response
    }
}