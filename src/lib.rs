use std::collections::HashMap;
use egui::{CentralPanel, SidePanel};
use petgraph::stable_graph::{StableGraph, NodeIndex};
use common::slc_commands::{ClientCommand, ClientEvent, ServerCommand, ServerEvent};
use crossbeam_channel::{Receiver, Sender};
use eframe::{egui, CreationContext};
use egui_graphs::{DefaultGraphView, Graph, SettingsInteraction, SettingsStyle};
use wg_2024::{
    controller::{DroneCommand, DroneEvent},
    network::NodeId,
};

pub struct MyApp {
    network: egui_graphs::Graph,
    selected_node: Option<NodeIndex>,
}

impl MyApp {
    fn new(_: &CreationContext<'_>) -> Self {
        let g = generate_graph();
        MyApp {
            network: Graph::from(&g),
            selected_node: Option::default(),
        }
    }

    fn read_data(&mut self) {
        if !self.network.selected_nodes().is_empty() {
            let idx = self.network.selected_nodes().first().unwrap();
            self.selected_node = Some(*idx);
        }
    }

    fn render(&mut self, ctx: &egui::Context) {
        SidePanel::right("Panel").show(ctx, |ui| {
            ui.label("Selected node:");
            if let Some(idx) = self.selected_node {
                ui.label(format!("{:?}", idx));
            }
        });
        CentralPanel::default().show(ctx, |ui| {
            let graph_widget = &mut DefaultGraphView::new(&mut self.network)
                .with_interactions(
                    &SettingsInteraction::default()
                    .with_node_selection_enabled(true)   
                )
                .with_styles(&SettingsStyle::default().with_labels_always(true));
            ui.add(graph_widget);
        });
        
    }

}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        self.read_data();
        self.render(ctx);
    }
}

fn generate_graph() -> StableGraph<(), ()> {
    let mut g = StableGraph::new();

    let a = g.add_node(());
    let b = g.add_node(());
    let c = g.add_node(());

    g.add_edge(a, b, ());
    g.add_edge(b, c, ());
    g.add_edge(c, a, ());

    g
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
            Box::new(|cc| Ok(Box::new(MyApp::new(cc)))),
        )
        .expect("Failed to run simulation controller");
    }
}
