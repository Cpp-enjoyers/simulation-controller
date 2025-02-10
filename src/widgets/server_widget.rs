use common::slc_commands::ServerCommand;
use crossbeam_channel::Sender;
use egui::{Ui, Widget};
use wg_2024::{network::NodeId, packet::Packet};

#[derive(Clone, Debug)]
/// Represents a server widget
/// 
/// This struct stores the `NodeId` and the `Sender<ServerCommand>` of the 
/// represented server. 
pub struct ServerWidget {
    /// The `NodeId` of the server
    pub id: NodeId,
    /// The `Sender<ServerCommand>` channel to send commands to the server
    pub command_ch: Sender<ServerCommand>,
}

impl ServerWidget {
    /// Creates a new `ServerWidget` with the given `id` and `command_ch`
    #[must_use] pub fn new(
        id: NodeId,
        command_ch: Sender<ServerCommand>,
    ) -> Self {
        Self {
            id,
            command_ch,
        }
    }

    /// Utility function to send a `ServerCommand::AddSender` command to the server
    /// Adds a new neighbor with `neighbor_id` to the server's neighbor list
    /// Furthermore, a clone of the `Sender<Packet>` channel is stored in the server
    /// 
    /// # Panics
    /// The function panics if the message is not sent
    pub fn add_neighbor(&mut self, neighbor_id: u8, neighbor_ch: Sender<Packet>) {
        self.command_ch
            .send(ServerCommand::AddSender(neighbor_id, neighbor_ch)).expect("msg not sent");
    }

    /// Utility function to send a `ServerCommand::RemoveSender` command to the server
    /// Removes a the neighbor with `neighbor_id` from the server's neighbor list
    /// 
    /// # Panics
    /// The function panics if the message is not sent
    pub fn remove_neighbor(&self, neighbor_id: u8) {
        self.command_ch
            .send(ServerCommand::RemoveSender(neighbor_id)).expect("msg not sent");
    }

    /// Utility function to get the `NodeId` of the server
    #[must_use] pub fn get_id(&self) -> NodeId {
        self.id
    }
}

/// Implement the `egui::Widget` trait for `ServerWidget`
/// 
/// This allows the `ServerWidget` to be rendered as an egui widget
/// 
/// # Example
/// ```no_run
/// use egui::Ui;
/// ui.add(ServerWidget::new(1, command_ch));
/// ```
impl Widget for ServerWidget {
    fn ui(self, ui: &mut Ui) -> egui::Response {
        ui.vertical_centered(|ui| {
            ui.label(format!("Server {}", self.id));
        }).response
    }
}