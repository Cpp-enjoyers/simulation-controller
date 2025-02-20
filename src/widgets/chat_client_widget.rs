use std::{cell::RefCell, collections::HashMap, rc::Rc};

use common::slc_commands::{ChatClientCommand, ServerType};
use crossbeam_channel::Sender;
use egui::{Align, Label, Layout, Sense, Widget};
use wg_2024::{network::NodeId, packet::Packet};

#[derive(Debug, Clone)]
pub struct ChatClientWidget {
    id: NodeId,
    command_ch: Sender<ChatClientCommand>,
    servers_types: HashMap<NodeId, ServerType>,
    list_connected_clients: HashMap<NodeId, Vec<u8>>,
    open_chat: Rc<RefCell<bool>>,
    chat_input: Rc<RefCell<String>>,
    chat_messages: Rc<RefCell<Vec<(bool, String)>>>,
}

impl ChatClientWidget {
    #[must_use]
    pub fn new(id: NodeId, command_ch: Sender<ChatClientCommand>) -> Self {
        Self {
            id,
            command_ch,
            servers_types: HashMap::default(),
            list_connected_clients: HashMap::default(),
            open_chat: Rc::new(RefCell::new(false)),
            chat_input: Rc::new(RefCell::new(String::new())),
            chat_messages: Rc::new(RefCell::new(Vec::new())),
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
            .send(ChatClientCommand::AddSender(neighbor_id, neighbor_ch))
            .expect("msg not sent");
    }

    /// Utility function to send a `ChatClientCommand::RemoveSender` command to the chat client
    /// Removes a the neighbor with `neighbor_id` from the chat client's neighbor list
    ///
    /// # Panics
    /// The function panics if the message is not sent
    pub fn remove_neighbor(&self, neighbor_id: u8) {
        self.command_ch
            .send(ChatClientCommand::RemoveSender(neighbor_id))
            .expect("msg not sent");
    }

    /// Function to add the server types to the chat client
    /// The server type is associated with the `server_id`
    /// The response is received from the mimicked chat client through the `ChatClientEvent::ServersTypes` event
    pub fn add_server_type(&mut self, response: &HashMap<NodeId, ServerType>) {
        for (k, v) in response {
            if *v == ServerType::ChatServer {
                self.servers_types.insert(*k, *v);
            }
        }
    }

    pub fn update_chat(&mut self, msg: String) {
        self.chat_messages.borrow_mut().push((false, msg));
    }

    /// Function to update the list of connected clients to a specific chat server
    /// The list of connected clients is associated with the `server_id`
    pub fn update_connected_client(&mut self, server_id: NodeId, connected_clients: Vec<u8>) {
        self.list_connected_clients
            .insert(server_id, connected_clients);
    }

    #[must_use]
    pub fn get_id(&self) -> NodeId {
        self.id
    }
}

/// Implementation of the `egui::Widget` trait for the `ChatClientWidget`
///
/// This allows the `ChatClientWidget` to be rendered as an egui widget
///
/// # Example
/// ```no_run
/// use egui::Ui;
/// ui.add(ChatClientWidget::new(1, command_ch));
/// ```
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
            // Clicking on a server will open a new window with the chat
            ui.label("Chat servers:");
            for id in self.servers_types.keys() {
                if ui
                    .add(Label::new(format!("Server {id}")).sense(Sense::click()))
                    .clicked()
                {
                    *self.open_chat.borrow_mut() = true;
                }

                egui::Window::new(format!("Chat Server {id}"))
                    .open(&mut self.open_chat.borrow_mut())
                    .resizable(false)
                    .scroll(true)
                    .show(ui.ctx(), |ui| {
                        ui.vertical(|ui| {
                            egui::ScrollArea::vertical()
                                .max_height(ui.available_height() - 45.0) // this is clearly a bad idea but oh
                                .stick_to_bottom(true)
                                .show(ui, |ui| {
                                    ui.label("Chat messages:");
                                    for (is_sender, msg) in self.chat_messages.borrow().iter() {
                                        if *is_sender {
                                            ui.with_layout(
                                                Layout::right_to_left(Align::TOP),
                                                |ui| {
                                                    ui.add(Label::new(format!("Me: {msg}")).wrap());
                                                },
                                            );
                                        } else {
                                            ui.with_layout(
                                                Layout::left_to_right(Align::TOP),
                                                |ui| {
                                                    // ui.label(format!("Other: {}", msg));
                                                    ui.add(Label::new(msg).wrap());
                                                },
                                            );
                                        }
                                    }
                                });
                        });
                        ui.with_layout(Layout::bottom_up(egui::Align::Center), |ui| {
                            ui.add_space(10.0);
                            ui.horizontal(|ui| {
                                ui.text_edit_singleline(&mut *self.chat_input.borrow_mut());
                                if ui.button("Send").clicked()
                                    && !self.chat_input.borrow().is_empty()
                                {
                                    self.chat_messages
                                        .borrow_mut()
                                        .push((true, self.chat_input.borrow().clone()));
                                    let cmd = ChatClientCommand::SendMessage(
                                        self.chat_input.borrow().clone(),
                                    );
                                    self.command_ch.send(cmd).expect("msg not sent");
                                    self.chat_input.borrow_mut().clear();
                                }
                            });
                        });
                    });
            }
            ui.separator();
        })
        .response
    }
}
