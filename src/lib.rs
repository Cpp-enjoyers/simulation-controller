#![warn(clippy::pedantic)]

use common::slc_commands::{ClientCommand, ClientEvent, ServerCommand, ServerEvent};
use crossbeam_channel::{Receiver, Sender};
use eframe::{egui, CreationContext};
use egui::{Button, CentralPanel, SidePanel, TopBottomPanel};
use egui_graphs::{
    Graph, GraphView, LayoutRandom, LayoutStateRandom, SettingsInteraction, SettingsNavigation, SettingsStyle
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
    network::NodeId, packet::Packet,
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
    add_remove_input: String,
    drones_channels: HashMap<NodeId, (Sender<DroneCommand>, Receiver<DroneEvent>, Sender<Packet>, Receiver<Packet>)>,
    clients_channels: HashMap<NodeId, (Sender<ClientCommand>, Receiver<ClientEvent>, Sender<Packet>, Receiver<Packet>)>,
    servers_channels: HashMap<NodeId, (Sender<ServerCommand>, Receiver<ServerEvent>, Sender<Packet>, Receiver<Packet>)>,
}

impl MyApp {
    fn new(
        _: &CreationContext<'_>,
        // drones: Vec<Drone>,
        // clients: Vec<Client>,
        // servers: Vec<Server>,
        graph: StableGraph<WidgetType, (), Undirected>,
        drones_channels: HashMap<NodeId, (Sender<DroneCommand>, Receiver<DroneEvent>, Sender<Packet>, Receiver<Packet>)>,
        clients_channels: HashMap<NodeId, (Sender<ClientCommand>, Receiver<ClientEvent>, Sender<Packet>, Receiver<Packet>)>,
        servers_channels: HashMap<NodeId, (Sender<ServerCommand>, Receiver<ServerEvent>, Sender<Packet>, Receiver<Packet>)>,
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
            drones_channels,
            clients_channels,
            servers_channels,
            selected_node: Option::default(),
            add_remove_input: String::default(),
        }
    }

    fn get_node_idx(&self, id: NodeId) -> NodeIndex {
        for (node_idx, widget) in self.network.nodes_iter() {
            match widget.payload() {
                WidgetType::Drone(drone_widget) => {
                    if drone_widget.get_id() == id {
                        return node_idx;
                    }
                },
                WidgetType::Client(client_widget) => {
                    if client_widget.get_id() == id {
                        return node_idx;
                    }
                },
                WidgetType::Server(server_widget) => {
                    if server_widget.get_id() == id {
                        return node_idx;
                    }
                },
            }
        }
        unreachable!("Se finisci qua rust ha la mamma puttana");
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
                .with_styles(&SettingsStyle::default().with_labels_always(true))
                .with_navigations(&SettingsNavigation::default().with_zoom_and_pan_enabled(true));
            ui.add(graph_widget);
        });
        TopBottomPanel::bottom("Bottom_panel").show(ctx, |ui| {
            if let Some(idx) = self.selected_node {
                ui.label(format!("Selected node: {:?}", idx));
                ui.horizontal(|ui| {

                    // // Buttons to add/remove sender
                    ui.text_edit_singleline(&mut self.add_remove_input);
                    let add_btn = ui.add(Button::new("Add sender"));
                    if add_btn.clicked() {
                        
                        let neighbor_id = self.add_remove_input.parse().unwrap();
                        // get the NodeIndex of the neighbor and a clone of its Sender
                        let neighbor_g_idx = self.get_node_idx(neighbor_id);
                        let neighbor_send_ch = match self.network.node(neighbor_g_idx).unwrap().payload() {
                            WidgetType::Drone(drone_widget) => self.drones_channels[&neighbor_id].2.clone(),
                            WidgetType::Client(client_widget) => self.clients_channels[&neighbor_id].2.clone(),
                            WidgetType::Server(server_widget) => self.servers_channels[&neighbor_id].2.clone(),
                        };
                        
                        let current_node = self.network.node_mut(idx).unwrap().payload_mut();
                        // get the id of the current and a clone of its Sender
                        let (current_node_id, current_send_ch) = match current_node {
                            WidgetType::Drone(drone_widget) => (drone_widget.get_id(), self.drones_channels[&drone_widget.get_id()].2.clone()),
                            WidgetType::Client(client_widget) => (client_widget.get_id(), self.clients_channels[&client_widget.get_id()].2.clone()),
                            WidgetType::Server(server_widget) => (server_widget.get_id(), self.servers_channels[&server_widget.get_id()].2.clone()),
                        };
                        
                        match current_node {
                            WidgetType::Drone(drone_widget) => {
                                drone_widget.add_neighbor(neighbor_id, neighbor_send_ch);
                            },
                            WidgetType::Client(client_widget) => {
                                client_widget.add_neighbor(neighbor_id, neighbor_send_ch);
                            },
                            WidgetType::Server(server_widget) => {
                                server_widget.add_neighbor(neighbor_id, neighbor_send_ch);
                            },
                        }
                        
                        let other_node = self.network.node_mut(neighbor_g_idx).unwrap().payload_mut();
                        match other_node {
                            WidgetType::Drone(other_drone_widget) => {
                                println!("drones_channels: {:?}", self.drones_channels);
                                println!("current_node_id: {:?}", current_node_id);
                                other_drone_widget.add_neighbor(current_node_id, current_send_ch);
                            },
                            WidgetType::Client(other_client_widget) => {
                                other_client_widget.add_neighbor(current_node_id, current_send_ch);
                            },
                            WidgetType::Server(other_server_widget) => {
                                other_server_widget.add_neighbor(current_node_id, current_send_ch);
                            },
                        }
                        self.network.add_edge(idx, neighbor_g_idx, ());
                        
                    }
                    
                    // Remove sender button
                    ui.text_edit_singleline(&mut self.add_remove_input);
                    let remove_btn = ui.add(Button::new("Remove sender"));
                    if remove_btn.clicked() {
                        let neighbor_id = self.add_remove_input.parse().unwrap();
                        let neighbor_g_idx = self.get_node_idx(neighbor_id);
                        let current_node = self.network.node_mut(idx).unwrap().payload_mut();
                        let current_node_id = match current_node {
                            WidgetType::Drone(drone_widget) =>  {
                                drone_widget.remove_neighbor(neighbor_id);
                                drone_widget.get_id()
                            },
                            WidgetType::Client(client_widget) => {
                                client_widget.remove_neighbor(neighbor_id);
                                client_widget.get_id()
                            },
                            WidgetType::Server(server_widget) => {
                                server_widget.remove_neighbor(neighbor_id);
                                server_widget.get_id()
                            },
                            
                        };
                        
                        let other_node = self.network.node_mut(neighbor_g_idx).unwrap().payload_mut();
                        match other_node {
                            WidgetType::Drone(other_drone_widget) => {
                                other_drone_widget.remove_neighbor(current_node_id);
                            },
                            WidgetType::Client(other_client_widget) => {
                                other_client_widget.remove_neighbor(current_node_id);
                            },
                            WidgetType::Server(other_server_widget) => {
                                other_server_widget.remove_neighbor(current_node_id);
                            },
                        }
                        
                        self.network.remove_edges_between(idx, neighbor_g_idx);
                    }
                });
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


#[derive(Debug)]
pub struct SimulationController {
    id: NodeId,
    drones_channels: HashMap<NodeId, (Sender<DroneCommand>, Receiver<DroneEvent>, Sender<Packet>, Receiver<Packet>)>,
    clients_channels: HashMap<NodeId, (Sender<ClientCommand>, Receiver<ClientEvent>, Sender<Packet>, Receiver<Packet>)>,
    servers_channels: HashMap<NodeId, (Sender<ServerCommand>, Receiver<ServerEvent>, Sender<Packet>, Receiver<Packet>)>,
    drones: Vec<Drone>,
    clients: Vec<Client>,
    servers: Vec<Server>,
}

impl SimulationController {
    pub fn new(
        id: NodeId,
        drones_channels: HashMap<NodeId, (Sender<DroneCommand>, Receiver<DroneEvent>, Sender<Packet>, Receiver<Packet>)>,
        clients_channels: HashMap<NodeId, (Sender<ClientCommand>, Receiver<ClientEvent>, Sender<Packet>, Receiver<Packet>)>,
        servers_channels: HashMap<NodeId, (Sender<ServerCommand>, Receiver<ServerEvent>, Sender<Packet>, Receiver<Packet>)>,
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
                self.drones_channels[&dr.id].2.clone(),
                self.drones_channels[&dr.id].3.clone(),
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
                    graph,
                    self.drones_channels.clone(),
                    self.clients_channels.clone(),
                    self.servers_channels.clone(),
                )))
            }),
        )
        .expect("Failed to run simulation controller");
    }
}
