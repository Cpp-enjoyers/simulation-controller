use std::{cell::RefCell, rc::Rc};

use crossbeam_channel::Sender;
use egui::{Color32, RichText, Ui, Widget};
use wg_2024::{controller::DroneCommand, network::NodeId, packet::Packet};

#[derive(Clone, Debug)]
/// Represents a drone widget
/// 
/// This struct stores the `NodeId` and the `Sender<DroneCommand>` of the
/// represented drone.
/// Furthermore, it stores the input for the packet drop rate (PDR) and a flag
/// to indicate if the input is invalid.
pub struct DroneWidget {
    /// The `NodeId` of the drone
    id: NodeId,
    /// The `Sender<DroneCommand>` channel to send commands to the drone
    command_ch: Sender<DroneCommand>,
    /// The input field for the packet drop rate (PDR)
    pdr_input: Rc<RefCell<String>>,
    /// Flag to indicate if the input for the PDR is invalid
    pdr_invalid: Rc<RefCell<String>>,
}

impl DroneWidget {
    /// Creates a new `DroneWidget` with the given `id` and `command_ch`
    #[must_use] pub fn new(
        id: NodeId,
        command_ch: Sender<DroneCommand>,
    ) -> Self {
        Self {
            id,
            command_ch,
            pdr_input: Rc::new(RefCell::new(String::default())),
            pdr_invalid: Rc::new(RefCell::new(String::default())),
        }
    }

    /// Utility function to send a `DroneCommand::AddSender` command to the drone
    /// Adds a new neighbor with `neighbor_id` to the drone's neighbor list
    /// Furthermore, a clone of the `Sender<Packet>` channel is stored in the drone
    /// 
    /// # Panics
    /// If the message is not sent
    pub fn add_neighbor(&mut self, neighbor_id: u8, neighbor_ch: Sender<Packet>) {
        self.command_ch
            .send(DroneCommand::AddSender(neighbor_id, neighbor_ch)).expect("msg not sent");
    }

    /// Utility function to send a `DroneCommand::RemoveSender` command to the drone
    /// Removes a the neighbor with `neighbor_id` from the drone's neighbor list
    /// 
    /// # Panics
    /// If the message is not sent
    pub fn remove_neighbor(&self, neighbor_id: u8) {
        self.command_ch
            .send(DroneCommand::RemoveSender(neighbor_id)).expect("msg not sent");
    }

    /// Utility function to get the `NodeId` of the drone
    #[must_use] pub fn get_id(&self) -> NodeId {
        self.id
    }

    /// Utility function to send a `DroneCommand::Crash` command to the drone
    /// 
    /// # Panics
    /// If the message is not sent
    pub fn send_crash_command(&self) {
        self.command_ch
            .send(DroneCommand::Crash).expect("msg not sent");
    }

    /// Function that validates the input for the PDR
    /// 
    /// The input is considered valid if it is not empty and can be parsed as a float
    /// between 0.0 and 1.0.
    /// 
    /// # Example
    /// ```no_run
    /// let pdr = "0.5".to_string();
    /// assert_eq!(validate_parse_pdr(&pdr), Some(0.5));
    /// 
    /// let pdr = "1.5".to_string();
    /// assert_eq!(validate_parse_pdr(&pdr), None);
    /// ```
    fn validate_parse_pdr(input_pdr: &str) -> Result<f32, String> {
        if input_pdr.is_empty() {
            return Err("Empty ID field".to_string());
        }
        let pdr = input_pdr.parse::<f32>();

        if pdr.is_err() {
            return Err("Wrong ID format".to_string());
        }
        let pdr = pdr.unwrap();
        // Bro....this is crazy
        if !(0.0..=1.0).contains(&pdr) {
            return Err("PDR must be between 0.0 and 1.0".to_string());
        }

        Ok(pdr)
    }
}

/// Implement the `egui::Widget` trait for `DroneWidget`
/// 
/// This allows the `DroneWidget` to be rendered as an egui widget
/// 
/// # Example
/// ```no_run
/// use egui::Ui;
/// ui.add(DroneWidget::new(1, command_ch));
/// ```
impl Widget for DroneWidget {
    fn ui(self, ui: &mut Ui) -> egui::Response {
        ui.vertical(|ui| {
            ui.label(format!("Drone {}", self.id));
            ui.label("Change PDR");
            ui.text_edit_singleline(&mut *self.pdr_input.borrow_mut());
            if ui.button("Send").clicked() {
                match DroneWidget::validate_parse_pdr(&self.pdr_input.borrow()) {
                    Ok(pdr) => {
                        self.pdr_invalid.borrow_mut().clear();
                        let cmd = DroneCommand::SetPacketDropRate(pdr);
                        self.command_ch.send(cmd).expect("msg not sent");
                    }
                    Err(error) => *self.pdr_invalid.borrow_mut() = error,

                }
            }

            if !self.pdr_invalid.borrow().is_empty() {
                ui.label(RichText::new(&*self.pdr_invalid.borrow()).color(Color32::RED));
            }
        }).response
    }
}