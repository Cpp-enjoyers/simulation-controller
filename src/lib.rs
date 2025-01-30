#![warn(clippy::pedantic)]

use common::slc_commands::{ClientCommand, ClientEvent, ServerCommand, ServerEvent};
use crossbeam_channel::{Receiver, Sender};
use eframe::{egui, CreationContext};
use egui::{CentralPanel, SidePanel, TopBottomPanel};
use egui_graphs::{
    Graph, GraphView, LayoutRandom, LayoutStateRandom, SettingsInteraction, SettingsStyle,
};
use petgraph::{
    graph,
    stable_graph::{NodeIndex, StableGraph, StableUnGraph},
    visit::NodeRef,
    Undirected,
};
use std::collections::{HashMap, HashSet};
use wg_2024::{
    config::{Client, Drone, Server},
    controller::{DroneCommand, DroneEvent},
    network::NodeId,
};
use widget::{ClientWidget, Drawable, DroneWidget, ServerWidget, Widget, WidgetType};
mod widget;

#[derive(Clone)]
pub struct GraphNode {
    pub id: u8,
    pub node_type: widget::NodeType,
}

pub struct MyApp {
    network: Graph<WidgetType, (), Undirected>,
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
        graph: StableGraph<WidgetType, (), Undirected>,
    ) -> Self {
        let mut graph = Graph::from(&graph);

        // Since graph library is beatiful, first iterate over the nodes to construct the labels for each node
        let temp: Vec<(NodeIndex, String)> = graph
            .nodes_iter()
            .map(|(idx, node)| match node.payload() {
                WidgetType::Drone(d) => (idx, format!("Drone {}", d.get_id())),
                WidgetType::Client(c) => (idx, format!("Client {}", c.get_id())),
                WidgetType::Server(s) => (idx, format!("Server {}", s.get_id())),
            })
            .collect();
        // Then iterate over the nodes again to set the labels
        for (idx, label) in temp {
            graph.node_mut(idx).unwrap().set_label(label);
        }

        MyApp {
            network: graph,
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
                let node = self.network.node_mut(idx).unwrap().payload_mut();
                match node {
                    WidgetType::Drone(drone_widget) => drone_widget.draw(ui),
                    WidgetType::Client(client_widget) => client_widget.draw(ui),
                    WidgetType::Server(server_widget) => server_widget.draw(ui),
                }
            }
        });
        CentralPanel::default().show(ctx, |ui| {
            let graph_widget: &mut GraphView<
                '_,
                WidgetType,
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
        TopBottomPanel::bottom("Bottom_panel").show(ctx, |ui| {
            if let Some(idx) = self.selected_node {
                ui.label(format!("Selected node: {:?}", idx));
            }
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

    fn generate_graph(&self) -> StableGraph<WidgetType, (), Undirected> {
        let mut g = StableUnGraph::default();
        let mut h: HashMap<u8, NodeIndex> = HashMap::new();
        let mut edges: HashSet<(u8, u8)> = HashSet::new();

        for dr in &self.drones {
            let idx = g.add_node(WidgetType::Drone(DroneWidget::new(
                dr.id,
                self.drones_channels[&dr.id].0.clone(),
                self.drones_channels[&dr.id].1.clone(),
            )));
            h.insert(dr.id, idx);
        }

        for cl in &self.clients {
            let idx = g.add_node(WidgetType::Client(ClientWidget::new(
                cl.id,
                self.clients_channels[&cl.id].0.clone(),
                self.clients_channels[&cl.id].1.clone(),
            )));
            h.insert(cl.id, idx);
        }

        for srv in &self.servers {
            let idx = g.add_node(WidgetType::Server(ServerWidget {
                id: srv.id,
                command_ch: self.servers_channels[&srv.id].0.clone(),
                event_ch: self.servers_channels[&srv.id].1.clone(),
            }));
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
