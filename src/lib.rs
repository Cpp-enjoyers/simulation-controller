#![warn(clippy::pedantic)]

use common::slc_commands::{ClientCommand, ClientEvent, ServerCommand, ServerEvent};
use crossbeam_channel::{Receiver, Sender};
use eframe::{egui, CreationContext};
use egui::{CentralPanel, SidePanel};
use egui_graphs::{
    DefaultGraphView, Edge, Graph, GraphView, LayoutRandom, LayoutStateRandom, SettingsInteraction,
    SettingsStyle,
};
use petgraph::{
    stable_graph::{NodeIndex, StableGraph, StableUnGraph},
    Undirected,
};
use std::collections::{HashMap, HashSet};
use wg_2024::{
    config::{Client, Drone, Server},
    controller::{DroneCommand, DroneEvent},
    network::NodeId,
};

#[derive(Clone, Default, Hash, Eq, PartialEq)]
pub struct GraphNode {
    pub id: u8,
    pub label: String,
}

pub struct MyApp {
    network: Graph<GraphNode, (), Undirected>,
    selected_node: Option<NodeIndex>,
}

impl MyApp {
    fn new(
        _: &CreationContext<'_>,
        drones: Vec<Drone>,
        clients: Vec<Client>,
        servers: Vec<Server>,
    ) -> Self {
        let g = generate_graph(drones, clients, servers);
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
                let node_label = self.network.node(idx).unwrap().payload().label.clone();
                ui.label(format!("Label: {}", node_label));
            }
        });
        CentralPanel::default().show(ctx, |ui| {
            let graph_widget: &mut GraphView<
                '_,
                GraphNode,
                (),
                petgraph::Undirected,
                u32,
                egui_graphs::DefaultNodeShape,
                egui_graphs::DefaultEdgeShape,
                LayoutStateRandom,
                LayoutRandom,
            > = &mut GraphView::new(&mut self.network)
                .with_interactions(
                    &SettingsInteraction::default().with_node_selection_enabled(true),
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

fn generate_graph(v: Vec<Drone>, cls: Vec<Client>, srvs: Vec<Server>) -> StableGraph<GraphNode, (), Undirected> {
    let mut g = StableUnGraph::default();
    let mut h: HashMap<u8, NodeIndex> = HashMap::new();
    let mut edges: HashSet<(u8, u8)> = HashSet::new();

    for d in &v {
        let idx = g.add_node(GraphNode {
            id: d.id,
            label: format!("Drone {}", d.id),
        });
        h.insert(d.id, idx);
    }

    for c in &cls {
        let idx = g.add_node(GraphNode {
            id: c.id,
            label: format!("Client {}", c.id),
        });
        h.insert(c.id, idx);
    }

    for s in &srvs {
        let idx = g.add_node(GraphNode {
            id: s.id,
            label: format!("Server {}", s.id),
        });
        h.insert(s.id, idx);
    }

    // Add edges
    for d in &v {
        for n in &d.connected_node_ids {
            if !edges.contains(&(d.id, *n)) && !edges.contains(&(*n, d.id)) {
                g.add_edge(h[&d.id], h[n], ());
                edges.insert((d.id, *n));
            }
        }
    }

    for c in &cls {
        for n in &c.connected_drone_ids {
            if !edges.contains(&(c.id, *n)) && !edges.contains(&(*n, c.id)) {
                g.add_edge(h[&c.id], h[n], ());
                edges.insert((c.id, *n));
            }
        }
    }

    for s in &srvs {
        for n in &s.connected_drone_ids {
            if !edges.contains(&(s.id, *n)) && !edges.contains(&(*n, s.id)) {
                g.add_edge(h[&s.id], h[n], ());
                edges.insert((s.id, *n));
            }
        }
    }

    g
}

#[derive(Debug)]
pub struct SimulationController {
    id: NodeId,
    drones_channels: HashMap<NodeId, (Sender<DroneCommand>, Receiver<DroneEvent>)>,
    clients_channels: HashMap<NodeId, (Sender<ClientCommand>, Receiver<ClientEvent>)>,
    servers_channels: HashMap<NodeId, (Sender<ServerCommand>, Receiver<ServerEvent>)>,
    drones: Vec<Drone>,
    clients: Vec<Client>,
    servers: Vec<Server>,
}

impl SimulationController {
    pub fn new(
        id: NodeId,
        drones_channels: HashMap<NodeId, (Sender<DroneCommand>, Receiver<DroneEvent>)>,
        clients_channels: HashMap<NodeId, (Sender<ClientCommand>, Receiver<ClientEvent>)>,
        servers_channels: HashMap<NodeId, (Sender<ServerCommand>, Receiver<ServerEvent>)>,
        drones: Vec<Drone>,
        clients: Vec<Client>,
        servers: Vec<Server>,
    ) -> Self {
        SimulationController {
            id,
            drones_channels,
            clients_channels,
            servers_channels,
            drones,
            clients,
            servers,
        }
    }

    pub fn run(&mut self) {
        println!(
            "Running simulation controller with drones: {}",
            self.drones_channels.len()
        );
        let options = eframe::NativeOptions::default();
        eframe::run_native(
            "Simulation Controller",
            options,
            Box::new(|cc| {
                Ok(Box::new(MyApp::new(
                    cc,
                    self.drones.clone(),
                    self.clients.clone(),
                    self.servers.clone(),
                )))
            }),
        )
        .expect("Failed to run simulation controller");
    }
}
