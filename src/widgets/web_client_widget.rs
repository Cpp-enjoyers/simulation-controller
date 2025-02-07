use std::{cell::RefCell, collections::HashMap, rc::Rc};

use common::slc_commands::{ServerType, WebClientCommand};
use crossbeam_channel::Sender;
use egui::{Label, RichText, Sense, Ui, Widget};
use wg_2024::{network::NodeId, packet::Packet};

#[derive(Clone, Debug)]
/// Represents a web client widget
/// 
/// This struct stores the `NodeId` and the `Sender<WebClientCommand>` of the
/// represented web client.
/// Furthermore, it stores the input for the server id and a flag to indicate if
/// the input is invalid.
/// It also stores the discovered servers with their types and the list of files
/// they have.
pub struct WebClientWidget {
    /// The `NodeId` of the web client
    id: NodeId,
    /// The `Sender<WebClientCommand>` channel to send commands to the web client
    command_ch: Sender<WebClientCommand>,
    /// The discovered servers with their types
    servers_types: HashMap<NodeId, ServerType>,
    /// The input field for the server id
    id_input: Rc<RefCell<String>>,
    /// Flag to indicate if the input for the server id is invalid
    id_input_error: Rc<RefCell<String>>,
    /// The list of files contained on the servers
    list_of_files: HashMap<NodeId, Vec<String>>,
}

impl WebClientWidget {
    /// Creates a new `WebClientWidget` with the given `id` and `command_ch`
    #[must_use] pub fn new(
        id: NodeId,
        command_ch: Sender<WebClientCommand>,
    ) -> Self {
        Self {
            id,
            command_ch,
            servers_types: HashMap::default(),
            id_input: Rc::new(RefCell::new(String::default())),
            id_input_error: Rc::new(RefCell::new(String::default())),
            list_of_files: HashMap::default(),
        }
    }

    /// Utility function to send a `WebClientCommand::AddSender` command to the web client
    /// Adds a new neighbor with `neighbor_id` to the web client's neighbor list
    /// Furthermore, a clone of the `Sender<Packet>` channel is stored in the web client
    /// 
    /// # Panics
    /// The function panics if the message is not sent
    pub fn add_neighbor(&mut self, neighbor_id: u8, neighbor_ch: Sender<Packet>) {
        self.command_ch
            .send(WebClientCommand::AddSender(neighbor_id, neighbor_ch)).expect("msg not sent");
    }

    /// Utility function to send a `WebClientCommand::RemoveSender` command to the web client
    /// Removes a the neighbor with `neighbor_id` from the web client's neighbor list
    /// 
    /// # Panics
    /// The function panics if the message is not sent
    pub fn remove_neighbor(&self, neighbor_id: u8) {
        self.command_ch
            .send(WebClientCommand::RemoveSender(neighbor_id)).expect("msg not sent");
    }

    /// Function to add a list of files to the web client
    /// The list of files is associated with the server with the given `server_id`
    /// The response is received from the mimicked client through the `WebClientEvent::ListOfFiles` event
    pub fn add_list_of_files(&mut self, server_id: NodeId, files: Vec<String>) {
        self.list_of_files.insert(server_id, files);
    }

    /// Function to add a servers type to the web client
    /// The server type is associated with the server with the given `server_id`
    /// The response is received from the mimicked client through the `WebClientEvent::ServersTypes` event
    pub fn add_server_type(&mut self, server_types: HashMap<NodeId, ServerType>) {
        self.servers_types = server_types;
    }

    /// Utility function to get the `NodeId` of the web client
    #[must_use] pub fn get_id(&self) -> NodeId {
        self.id
    }

    /// Function that validates the input for the server id
    /// 
    /// The function checks if the input is empty, if the input can be parsed to a `NodeId`
    /// and if the parsed `NodeId` is a valid server id.
    /// 
    /// # Example
    /// ```no_run
    /// let input_id = "1".to_string();
    /// assert_eq!(validate_parse_id(&input_id), Ok(1));
    /// 
    /// let input_id = "a".to_string();
    /// assert_eq!(validate_parse_id(&input_id), Err("Wrong ID format".to_string()));
    /// ```
    fn validate_parse_id(&self, input_id: &str) -> Result<NodeId, String> {
        if input_id.is_empty() {
            return Err("Empty ID field".to_string());
        }

        let id = input_id.parse::<NodeId>();

        if id.is_err() {
            return Err("Wrong ID format".to_string());
        }

        let id = id.unwrap();
        if self.servers_types.contains_key(&id) {
            Ok(id)
        } else {
            Err("Server ID not found".to_string())
        }
    }
}

/// Implementation of the `egui::Widget` trait for the `WebClientWidget`
/// 
/// This allows the `WebClientWidget` to be rendered as an egui widget
/// 
/// # Example
/// ```no_run
/// use egui::Ui;
/// ui.add(WebClientWidget::new(1, command_ch));
/// ```
impl Widget for WebClientWidget {
    fn ui(self, ui: &mut Ui) -> egui::Response {
        ui.vertical(|ui| {
            ui.label(format!("Web Client {}", self.id));

            // Send command to ask for servers types
            ui.label("Ask for Server types");
            if ui.button("Send").clicked() {
                let cmd = WebClientCommand::AskServersTypes;
                self.command_ch.send(cmd).expect("msg not sent");
            }

            ui.label("Servers types:");
            for (id, srv_type) in &self.servers_types {
                ui.label(format!("Server {id}: {srv_type:?}"));
            }

            ui.separator();

            // Send command to ask for files
            ui.label("Ask for Server files");
            ui.text_edit_singleline(&mut *self.id_input.borrow_mut());
            if ui.button("Send").clicked() {
                match self.validate_parse_id(&self.id_input.borrow()) {
                    Ok(id) => {
                        self.id_input_error.borrow_mut().clear();
                        let cmd = WebClientCommand::AskListOfFiles(id);
                        self.command_ch.send(cmd).expect("msg not sent");
                    },
                    Err(error) => *self.id_input_error.borrow_mut() = error,
                }
            }

            if !self.id_input_error.borrow().is_empty() {
                ui.label(RichText::new(&*self.id_input_error.borrow()).color(egui::Color32::RED));
            }

            ui.separator();
            ui.label("Received files:");
            for (server_id, server_files) in &self.list_of_files {
                ui.label(format!("Server {server_id}: "));
                for file in server_files {
                    let file_name = file.split('/').last().unwrap().to_string();
                    if ui.add(Label::new(file_name).sense(Sense::click())).clicked() {
                        let cmd = WebClientCommand::RequestFile(file.to_string(), *server_id);
                        self.command_ch.send(cmd).expect("msg not sent");
                    }

                }
            }
        }).response
    }
}