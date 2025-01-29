#![warn(clippy::pedantic)]

use common::slc_commands::{ClientCommand, ClientEvent, ServerCommand, ServerEvent};
use crossbeam_channel::{Receiver, Sender};
use eframe::{egui, CreationContext};
use egui::{CentralPanel, SidePanel};
use egui_graphs::{
    Graph, GraphView, LayoutRandom, LayoutStateRandom, SettingsInteraction,
    SettingsStyle,
};
use petgraph::{graph, stable_graph::{NodeIndex, StableGraph, StableUnGraph}, Undirected};
use widget::Widget;
use std::collections::{HashMap, HashSet};
use wg_2024::{
    config::{Client, Drone, Server},
    controller::{DroneCommand, DroneEvent},
    network::NodeId,
};
mod widget;

#[derive(Clone)]
pub struct GraphNode {
    pub id: u8,
    pub node_type: widget::NodeType,
}

pub struct MyApp {
    network: Graph<GraphNode, (), Undirected>,
    selected_node: Option<NodeIndex>,
    input: String,
    result: Vec<String>,
}

impl MyApp {
    fn new(
        _: &CreationContext<'_>,
        // drones: Vec<Drone>,
        // clients: Vec<Client>,
        // servers: Vec<Server>,
        graph: StableGraph<GraphNode, (), Undirected>,
    ) -> Self {
        // let graph = generate_graph(drones, clients, servers);
        MyApp {
            network: Graph::from(&graph),
            selected_node: Option::default(),
            input: String::default(),
            result: Vec::default(),
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
                let node = self.network.node(idx).unwrap().payload();
                match &node.node_type {
                    widget::NodeType::Drone { command_ch, event_ch } => todo!(),
                    widget::NodeType::Client { command_ch, event_ch } => {
                        ui.label("Client: {node.id}");
                        ui.label("Ask for Server files");
                        ui.text_edit_singleline(&mut self.input);
                        if ui.button("Send").clicked() {
                            let cmd = ClientCommand::AskListOfFiles(self.input.parse().unwrap());
                            // let fake_cmd = ClientCommand::AskListOfFiles(3);
                            command_ch.send(cmd);
                        }

                        ui.separator();
                        ui.label("Received files:");
                        while let Ok(event) = event_ch.try_recv() {
                            match event {
                                ClientEvent::ListOfFiles(files, id) => {
                                    self.result = files;
                                }
                                _ => {}
                            }
                        }

                        for f in &self.result {
                            ui.label(f);
                        }
                    },
                    widget::NodeType::Server { command_ch, event_ch } => todo!(),
                }
                // match node.node_type {
                //     widget::NodeType::Drone => {
                //         ui.label("Drone");
                //     }
                //     widget::NodeType::Client => {
                //         ui.label("Client");
                //         ui.label("Ask for Server files");
                //         ui.text_edit_singleline(&mut "".to_string());
                //         if ui.button("Send").clicked() {
                //             // Send message to server
                //         }
                //     }
                //     widget::NodeType::Server => {
                //         ui.label("Server");
                //     }
                // }
                // let node_label = self.network.node(idx).unwrap().payload().label.clone();
                // ui.label(format!("Label: {}", node_label));
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

// fn generate_graph(v: Vec<Drone>, cls: Vec<Client>, srvs: Vec<Server>) -> StableGraph<GraphNode, (), Undirected> {
//     let mut g = StableUnGraph::default();
//     let mut h: HashMap<u8, NodeIndex> = HashMap::new();
//     let mut edges: HashSet<(u8, u8)> = HashSet::new();

//     for d in &v {
//         let idx = g.add_node(GraphNode {
//             id: d.id,
//             node_type: widget::NodeType::Drone {
//                 command_ch: d.command_ch.clone(),
//                 event_ch: d.event_ch.clone(),
//             },
//         });
//         h.insert(d.id, idx);
//     }

//     for c in &cls {
//         let idx = g.add_node(GraphNode {
//             id: c.id,
//             node_type: widget::NodeType::Client,
//         });
//         h.insert(c.id, idx);
//     }

//     for s in &srvs {
//         let idx = g.add_node(GraphNode {
//             id: s.id,
//             node_type: widget::NodeType::Server,
//         });
//         h.insert(s.id, idx);
//     }

//     // Add edges
//     for d in &v {
//         for n in &d.connected_node_ids {
//             if !edges.contains(&(d.id, *n)) && !edges.contains(&(*n, d.id)) {
//                 g.add_edge(h[&d.id], h[n], ());
//                 edges.insert((d.id, *n));
//             }
//         }
//     }

//     for c in &cls {
//         for n in &c.connected_drone_ids {
//             if !edges.contains(&(c.id, *n)) && !edges.contains(&(*n, c.id)) {
//                 g.add_edge(h[&c.id], h[n], ());
//                 edges.insert((c.id, *n));
//             }
//         }
//     }

//     for s in &srvs {
//         for n in &s.connected_drone_ids {
//             if !edges.contains(&(s.id, *n)) && !edges.contains(&(*n, s.id)) {
//                 g.add_edge(h[&s.id], h[n], ());
//                 edges.insert((s.id, *n));
//             }
//         }
//     }

//     g
// }

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

    fn generate_graph(&self) -> StableGraph<GraphNode, (), Undirected> {
        let mut g = StableUnGraph::default();
        let mut h: HashMap<u8, NodeIndex> = HashMap::new();
        let mut edges: HashSet<(u8, u8)> = HashSet::new();

        for dr in &self.drones {
            let idx = g.add_node(GraphNode {
                id: dr.id,
                node_type: widget::NodeType::Drone {
                    command_ch: self.drones_channels[&dr.id].0.clone(),
                    event_ch: self.drones_channels[&dr.id].1.clone(),
                },
            });
            h.insert(dr.id, idx);
        }

        for cl in &self.clients {
            let idx = g.add_node(GraphNode {
                id: cl.id,
                node_type: widget::NodeType::Client {
                    command_ch: self.clients_channels[&cl.id].0.clone(),
                    event_ch: self.clients_channels[&cl.id].1.clone(),
                },
            });
            h.insert(cl.id, idx);
        }

        for srv in &self.servers {
            let idx = g.add_node(GraphNode {
                id: srv.id,
                node_type: widget::NodeType::Server {
                    command_ch: self.servers_channels[&srv.id].0.clone(),
                    event_ch: self.servers_channels[&srv.id].1.clone(),
                },
            });
            h.insert(srv.id, idx);
        }

        // Add edges
        for dr in &self.drones {
            for n in &dr.connected_node_ids {
                if !edges.contains(&(dr.id, *n)) && !edges.contains(&(*n, dr.id)) {
                    g.add_edge(h[&dr.id], h[n], ());
                    edges.insert((dr.id, *n));
                }
            }
        }

        for cl in &self.clients {
            for n in &cl.connected_drone_ids {
                if !edges.contains(&(cl.id, *n)) && !edges.contains(&(*n, cl.id)) {
                    g.add_edge(h[&cl.id], h[n], ());
                    edges.insert((cl.id, *n));
                }
            }
        }

        for srv in &self.servers {
            for n in &srv.connected_drone_ids {
                if !edges.contains(&(srv.id, *n)) && !edges.contains(&(*n, srv.id)) {
                    g.add_edge(h[&srv.id], h[n], ());
                    edges.insert((srv.id, *n));
                }
            }
        }

        g
    }

    pub fn run(&mut self) {
        let graph = self.generate_graph();
        let options = eframe::NativeOptions::default();
        eframe::run_native(
            "Simulation Controller",
            options,
            Box::new(|cc| {
                Ok(Box::new(MyApp::new(
                    cc,
                    // self.drones.clone(),
                    // self.clients.clone(),
                    // self.servers.clone(),
                    graph,
                )))
            }),
        )
        .expect("Failed to run simulation controller");
    }
}
