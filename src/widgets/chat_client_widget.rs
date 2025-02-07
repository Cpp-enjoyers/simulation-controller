use std::collections::HashMap;

use common::slc_commands::{ChatClientCommand, ServerType};
use crossbeam_channel::Sender;
use egui::{Label, Sense, Widget};
use wg_2024::{network::NodeId, packet::Packet};


#[derive(Debug, Clone)]
pub struct ChatClientWidget {
    id: NodeId,
    command_ch: Sender<ChatClientCommand>,
    servers_types: HashMap<NodeId, ServerType>,
}

impl ChatClientWidget {
    #[must_use] pub fn new(id: NodeId, command_ch: Sender<ChatClientCommand>) -> Self {
        Self { 
            id,
            command_ch,
            servers_types: HashMap::default(),
        }
    }

    /// Utility function to send a `ChatClientCommand::AddSender` command to the chat client
    /// Adds a new neighbor with `neighbor_id` to the chat client's neighbor list
    /// Furthermore, a clone of the `Sender<Packet>` channel is stored in the chat client
    /// 
    /// # Panics
    /// The function panics if the message is not sent
    pub fn add_neighbor(&mut self, neighbor_id: u8, neighbor_ch: Sender<Packet>) {
        self.command_ch
            .send(ChatClientCommand::AddSender(neighbor_id, neighbor_ch)).expect("msg not sent");
    }
    
    /// Utility function to send a `ChatClientCommand::RemoveSender` command to the chat client
    /// Removes a the neighbor with `neighbor_id` from the chat client's neighbor list
    /// 
    /// # Panics
    /// The function panics if the message is not sent
    pub fn remove_neighbor(&self, neighbor_id: u8) {
        self.command_ch
            .send(ChatClientCommand::RemoveSender(neighbor_id)).expect("msg not sent");
    }

    /**
     * Add the list of chat servers to the widget
     */
    pub fn add_server_type(&mut self, response: &HashMap<NodeId, ServerType>) {
        for (k, v) in response {
            if *v == ServerType::ChatServer {
                self.servers_types.insert(*k, *v);
            }
        }
    }

    #[must_use] pub fn get_id(&self) -> NodeId {
        self.id
    }
}

impl Widget for ChatClientWidget {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        ui.vertical(|ui| {
            ui.label(format!("Chat Client {}", self.id));

            // Send command to ask for servers types
            ui.label("Ask for Server types");
            if ui.button("Send").clicked() {
                let cmd = ChatClientCommand::AskServersTypes;
                self.command_ch.send(cmd).expect("msg not sent");
            }

            // Display the list of chat servers
            // Clicking on a server will connect to it
            ui.label("Chat servers:");
            for id in self.servers_types.keys() {
                if ui.add(Label::new(format!("Server {id}")).sense(Sense::click())).clicked() {
                    let cmd = ChatClientCommand::ConnectToChatServer(*id);
                    self.command_ch.send(cmd).expect("msg not sent");
                }
            }

            ui.separator();
        }).response
    }
}