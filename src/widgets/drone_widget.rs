use crossbeam_channel::Sender;
use egui::{Button, Color32, RichText, Ui, Widget};
use wg_2024::{controller::DroneCommand, network::NodeId, packet::Packet};

#[derive(Clone, Debug)]
pub struct DroneWidget {
    id: NodeId,
    command_ch: Sender<DroneCommand>,
    pdr_input: String,
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
        }
    }

    pub fn add_neighbor(&mut self, neighbor_id: u8, neighbor_ch: Sender<Packet>) {
        self.command_ch
            .send(DroneCommand::AddSender(neighbor_id, neighbor_ch)).expect("msg not sent");
    }

    pub fn remove_neighbor(&mut self, neighbor_id: u8) {
        self.command_ch
            .send(DroneCommand::RemoveSender(neighbor_id)).expect("msg not sent");
    }

    pub fn get_id(&self) -> NodeId {
        self.id
    }
}


impl Widget for &mut DroneWidget {
    fn ui(self, ui: &mut Ui) -> egui::Response {
        ui.vertical(|ui| {
            ui.label(format!("Drone {}", self.id));
            ui.label("Change PDR");
            ui.text_edit_singleline(&mut self.pdr_input);
            if ui.button("Send").clicked() {
                let cmd = DroneCommand::SetPacketDropRate(self.pdr_input.parse().unwrap());
                self.command_ch.send(cmd).expect("msg not sent");
            }

            ui.separator();
            ui.label("Crash the drone");
            let red_btn =
                ui.add(Button::new(RichText::new("Crash").color(Color32::BLACK)).fill(Color32::RED));
            if red_btn.clicked() {
                self.command_ch.send(DroneCommand::Crash).expect("msg not sent");
            }
        }).response
    }
}