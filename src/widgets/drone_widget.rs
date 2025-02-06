use crossbeam_channel::Sender;
use egui::{Button, Color32, RichText, Ui, Widget};
use wg_2024::{controller::DroneCommand, network::NodeId, packet::Packet};

#[derive(Clone, Debug)]
pub struct DroneWidget {
    id: NodeId,
    command_ch: Sender<DroneCommand>,
    pdr_input: String,
    is_pdr_invalid: bool,
}

impl DroneWidget {
    pub fn new(
        id: NodeId,
        command_ch: Sender<DroneCommand>,
    ) -> Self {
        Self {
            id,
            command_ch,
            pdr_input: String::default(),
            is_pdr_invalid: false,
        }
    }

    pub fn add_neighbor(&mut self, neighbor_id: u8, neighbor_ch: Sender<Packet>) {
        self.command_ch
            .send(DroneCommand::AddSender(neighbor_id, neighbor_ch)).expect("msg not sent");
    }

    pub fn remove_neighbor(&self, neighbor_id: u8) {
        self.command_ch
            .send(DroneCommand::RemoveSender(neighbor_id)).expect("msg not sent");
    }

    pub fn get_id(&self) -> NodeId {
        self.id
    }

    pub fn send_crash_command(&self) {
        self.command_ch
            .send(DroneCommand::Crash).expect("msg not sent");
    }

    fn validate_parse_pdr(&self, input_pdr: &String) -> Option<f32> {
        if input_pdr.is_empty() {
            return None;
        }

        let pdr = input_pdr.parse::<f32>().unwrap();
        if pdr < 0.0 || pdr > 1.0 {
            return None;
        }

        Some(pdr)
    }
}


impl Widget for DroneWidget {
    fn ui(mut self, ui: &mut Ui) -> egui::Response {
        ui.vertical(|ui| {
            ui.label(format!("Drone {}", self.id));
            ui.label("Change PDR");
            ui.text_edit_singleline(&mut self.pdr_input);
            if ui.button("Send").clicked() {
                match self.validate_parse_pdr(&self.pdr_input) {
                    Some(pdr) => {
                        self.is_pdr_invalid = false;
                        let cmd = DroneCommand::SetPacketDropRate(pdr);
                        self.command_ch.send(cmd).expect("msg not sent");
                    }
                    None => self.is_pdr_invalid = true,

                }
            }

            if self.is_pdr_invalid {
                ui.label(RichText::new("Invalid or empty PDR field!").color(Color32::RED));
            }
        }).response
    }
}