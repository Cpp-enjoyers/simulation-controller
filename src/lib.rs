use std::collections::HashMap;

use common::slc_commands::{ClientCommand, ClientEvent, ServerCommand, ServerEvent};
use crossbeam_channel::{Receiver, Sender};
use eframe::egui;
use wg_2024::{
    controller::{DroneCommand, DroneEvent},
    network::NodeId,
};

pub struct MyApp {
    name: String,
}

impl Default for MyApp {
    fn default() -> Self {
        Self {
            name: "Hello, eframe!".to_owned(),
        }
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading(&self.name);
            if ui.button("Click me!").clicked() {
                self.name = "You clicked the button!".to_owned();
            }
        });
    }
}

#[derive(Debug)]
pub struct SimulationController {
    id: NodeId,
    drones_channels: HashMap<NodeId, (Sender<DroneCommand>, Receiver<DroneEvent>)>,
    clients_channels: HashMap<NodeId, (Sender<ClientCommand>, Receiver<ClientEvent>)>,
    servers_channels: HashMap<NodeId, (Sender<ServerCommand>, Receiver<ServerEvent>)>,
}

impl SimulationController {
    pub fn new(
        id: NodeId,
        drones_channels: HashMap<NodeId, (Sender<DroneCommand>, Receiver<DroneEvent>)>,
        clients_channels: HashMap<NodeId, (Sender<ClientCommand>, Receiver<ClientEvent>)>,
        servers_channels: HashMap<NodeId, (Sender<ServerCommand>, Receiver<ServerEvent>)>,
    ) -> Self {
        SimulationController {
            id,
            drones_channels,
            clients_channels,
            servers_channels,
        }
    }

    pub fn run(&mut self) {
        println!("Running simulation controller with drones: {}", self.drones_channels.len());
        let options = eframe::NativeOptions::default();
        eframe::run_native(
            "Simulation Controller",
            options,
            Box::new(|cc| Box::new(MyApp::default())),
        )
        .expect("Failed to run simulation controller");
    }
}
