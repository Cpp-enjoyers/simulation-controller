#![warn(clippy::pedantic)]

use common::slc_commands::{ClientCommand, ClientEvent, ServerCommand, ServerEvent};
use crossbeam_channel::{Receiver, Sender};
use eframe::{egui, CreationContext};
use egui::{accesskit::Node, Button, CentralPanel, SidePanel, TopBottomPanel};
use egui_graphs::{
    Graph, GraphView, LayoutRandom, LayoutStateRandom, SettingsInteraction, SettingsNavigation,
    SettingsStyle,
};
use petgraph::{
    graph, stable_graph::{NodeIndex, StableGraph, StableUnGraph}, Undirected
};
use std::{cell::RefCell, collections::{HashMap, HashSet}, rc::Rc};
use wg_2024::{
    config::{Client, Drone, Server},
    controller::{DroneCommand, DroneEvent},
    network::NodeId,
    packet::Packet,
};
use widget::{ClientWidget, Drawable, DroneWidget, ServerWidget, WidgetType};
mod widget;

#[derive(Clone, Debug)]
pub enum Events {
    DroneEvent(DroneEvent),
    ClientEvent(ClientEvent),
    ServerEvent(ServerEvent),
}

pub fn run(id: NodeId,
    drones_channels: HashMap<
        NodeId,
        (
            Sender<DroneCommand>,
            Receiver<DroneEvent>,
            Sender<Packet>,
            Receiver<Packet>,
        ),
    >,
    clients_channels: HashMap<
        NodeId,
        (
            Sender<ClientCommand>,
            Receiver<ClientEvent>,
            Sender<Packet>,
            Receiver<Packet>,
        ),
    >,
    servers_channels: HashMap<
        NodeId,
        (
            Sender<ServerCommand>,
            Receiver<ServerEvent>,
            Sender<Packet>,
            Receiver<Packet>,
        ),
    >,
    drones: Vec<Drone>,
    clients: Vec<Client>,
    servers: Vec<Server>,) {
        let options = eframe::NativeOptions::default();
        eframe::run_native(
            "Simulation Controller",
            options,
            Box::new(|cc| Ok(Box::new(SimulationController::new(
                id,
                drones_channels,
                clients_channels,
                servers_channels,
                drones,
                clients,
                servers,
            ))))
        ).expect("Failed to run simulation controller");
}

type UWidget = Rc<RefCell<WidgetType>>;
pub fn generate_widgets(d: &HashMap<NodeId, (Sender<DroneCommand>, Receiver<DroneEvent>, Sender<Packet>, Receiver<Packet>),>,
                        c: &HashMap<NodeId, (Sender<ClientCommand>, Receiver<ClientEvent>, Sender<Packet>, Receiver<Packet>)>,
                        s: &HashMap<NodeId, (Sender<ServerCommand>, Receiver<ServerEvent>, Sender<Packet>, Receiver<Packet>)>)
                        -> HashMap<NodeId, UWidget>{
    let mut w: HashMap<NodeId, UWidget> = HashMap::new();
    for (id, channels) in d {
        w.insert(*id, Rc::new(RefCell::new(WidgetType::Drone(DroneWidget::new(
            *id,
            channels.0.clone(),
            channels.1.clone(),
        )))));
    }

    for (id, channels) in c {
        w.insert(*id, Rc::new(RefCell::new(WidgetType::Client(ClientWidget::new(
            *id,
            channels.0.clone(),
            channels.1.clone(),
        )))));
    }

    for (id, channels) in s {
        w.insert(*id, Rc::new(RefCell::new(WidgetType::Server(ServerWidget {
            id: *id,
            command_ch: channels.0.clone(),
            event_ch: channels.1.clone(),
        }))));
    }
    w
}

fn generate_graph(wid: &HashMap<NodeId, UWidget>, drones: &Vec<Drone>, clients: &Vec<Client>, servers: &Vec<Server>) -> Graph<UWidget, (), Undirected> {
    let mut g = StableUnGraph::default();
    let mut h: HashMap<u8, NodeIndex> = HashMap::new();
    let mut edges: HashSet<(u8, u8)> = HashSet::new();

    for widget in wid {
        let idx = g.add_node(widget.1.clone());
        h.insert(*widget.0, idx);
    }

    // Add edges
    for dr in drones {
        for n in &dr.connected_node_ids {
            if !edges.contains(&(dr.id, *n)) && !edges.contains(&(*n, dr.id)) {
                g.add_edge(h[&dr.id], h[n], ());
                edges.insert((dr.id, *n));
            }
        }
    }

    for cl in clients {
        for n in &cl.connected_drone_ids {
            if !edges.contains(&(cl.id, *n)) && !edges.contains(&(*n, cl.id)) {
                g.add_edge(h[&cl.id], h[n], ());
                edges.insert((cl.id, *n));
            }
        }
    }

    for srv in servers {
        for n in &srv.connected_drone_ids {
            if !edges.contains(&(srv.id, *n)) && !edges.contains(&(*n, srv.id)) {
                g.add_edge(h[&srv.id], h[n], ());
                edges.insert((srv.id, *n));
            }
        }
    }

    let mut eg_graph = Graph::from(&g);
    // Since graph library is beatiful, first iterate over the nodes to construct the labels for each node
    let temp: Vec<(NodeIndex, String)> = eg_graph
        .nodes_iter()
        .map(|(idx, node)| {
            let widget = node.payload().borrow();
            match &*widget {
                WidgetType::Drone(d) => (idx, format!("Drone {}", d.get_id())),
                WidgetType::Client(c) => (idx, format!("Client {}", c.get_id())),
                WidgetType::Server(s) => (idx, format!("Server {}", s.get_id())),
            }
        })
        .collect();
    // Then iterate over the nodes again to set the labels
    for (idx, label) in temp {
        eg_graph.node_mut(idx).unwrap().set_label(label);
    }

    eg_graph
}

// pub struct MyApp {
//     network: Graph<WidgetType, (), Undirected>,
//     selected_node: Option<NodeIndex>,
//     add_neighbor_input: String,
//     rm_neighbor_input: String,
//     drones_channels: HashMap<
//         NodeId,
//         (
//             Sender<DroneCommand>,
//             Receiver<DroneEvent>,
//             Sender<Packet>,
//             Receiver<Packet>,
//         ),
//     >,
//     clients_channels: HashMap<
//         NodeId,
//         (
//             Sender<ClientCommand>,
//             Receiver<ClientEvent>,
//             Sender<Packet>,
//             Receiver<Packet>,
//         ),
//     >,
//     servers_channels: HashMap<
//         NodeId,
//         (
//             Sender<ServerCommand>,
//             Receiver<ServerEvent>,
//             Sender<Packet>,
//             Receiver<Packet>,
//         ),
//     >,
// }

// impl MyApp {
//     fn new(
//         _: &CreationContext<'_>,
//         // drones: Vec<Drone>,
//         // clients: Vec<Client>,
//         // servers: Vec<Server>,
//         graph: StableGraph<WidgetType, (), Undirected>,
//         drones_channels: HashMap<
//             NodeId,
//             (
//                 Sender<DroneCommand>,
//                 Receiver<DroneEvent>,
//                 Sender<Packet>,
//                 Receiver<Packet>,
//             ),
//         >,
//         clients_channels: HashMap<
//             NodeId,
//             (
//                 Sender<ClientCommand>,
//                 Receiver<ClientEvent>,
//                 Sender<Packet>,
//                 Receiver<Packet>,
//             ),
//         >,
//         servers_channels: HashMap<
//             NodeId,
//             (
//                 Sender<ServerCommand>,
//                 Receiver<ServerEvent>,
//                 Sender<Packet>,
//                 Receiver<Packet>,
//             ),
//         >,
//     ) -> Self {
//         let mut graph = Graph::from(&graph);

//         // Since graph library is beatiful, first iterate over the nodes to construct the labels for each node
//         let temp: Vec<(NodeIndex, String)> = graph
//             .nodes_iter()
//             .map(|(idx, node)| match node.payload() {
//                 WidgetType::Drone(d) => (idx, format!("Drone {}", d.get_id())),
//                 WidgetType::Client(c) => (idx, format!("Client {}", c.get_id())),
//                 WidgetType::Server(s) => (idx, format!("Server {}", s.get_id())),
//             })
//             .collect();
//         // Then iterate over the nodes again to set the labels
//         for (idx, label) in temp {
//             graph.node_mut(idx).unwrap().set_label(label);
//         }

//         MyApp {
//             network: graph,
//             drones_channels,
//             clients_channels,
//             servers_channels,
//             selected_node: Option::default(),
//             add_neighbor_input: String::default(),
//             rm_neighbor_input: String::default(),
//         }
//     }

//     fn get_node_idx(&self, id: NodeId) -> NodeIndex {
//         for (node_idx, widget) in self.network.nodes_iter() {
//             match widget.payload() {
//                 WidgetType::Drone(drone_widget) => {
//                     if drone_widget.get_id() == id {
//                         return node_idx;
//                     }
//                 }
//                 WidgetType::Client(client_widget) => {
//                     if client_widget.get_id() == id {
//                         return node_idx;
//                     }
//                 }
//                 WidgetType::Server(server_widget) => {
//                     if server_widget.get_id() == id {
//                         return node_idx;
//                     }
//                 }
//             }
//         }
//         unreachable!("Se finisci qua rust ha la mamma puttana");
//     }

//     fn read_data(&mut self) {
//         if !self.network.selected_nodes().is_empty() {
//             let idx = self.network.selected_nodes().first().unwrap();
//             self.selected_node = Some(*idx);
//         }
//     }

//     fn render(&mut self, ctx: &egui::Context) {
//         SidePanel::right("Panel").show(ctx, |ui| {
//             ui.label("Selected node:");
//             if let Some(idx) = self.selected_node {
//                 let node = self.network.node_mut(idx).unwrap().payload_mut();
//                 match node {
//                     WidgetType::Drone(drone_widget) => drone_widget.draw(ui),
//                     WidgetType::Client(client_widget) => client_widget.draw(ui),
//                     WidgetType::Server(server_widget) => server_widget.draw(ui),
//                 }
//             }
//         });
//         CentralPanel::default().show(ctx, |ui| {
//             let graph_widget: &mut GraphView<
//                 '_,
//                 WidgetType,
//                 (),
//                 petgraph::Undirected,
//                 u32,
//                 egui_graphs::DefaultNodeShape,
//                 egui_graphs::DefaultEdgeShape,
//                 LayoutStateRandom,
//                 LayoutRandom,
//             > = &mut GraphView::new(&mut self.network)
//                 .with_interactions(
//                     &SettingsInteraction::default().with_node_selection_enabled(true),
//                 )
//                 .with_styles(&SettingsStyle::default().with_labels_always(true))
//                 .with_navigations(&SettingsNavigation::default().with_zoom_and_pan_enabled(true));
//             ui.add(graph_widget);
//         });
//         TopBottomPanel::bottom("Bottom_panel").show(ctx, |ui| {
//             if let Some(idx) = self.selected_node {
//                 ui.label(format!("Selected node: {:?}", idx));
//                 ui.horizontal(|ui| {
//                     // Buttons to add/remove sender
//                     ui.vertical(|ui| {
//                         ui.set_max_width(71.0); // Width of the add button
//                         // ui.add_sized([btn_size.x, btn_size.y], TextEdit::singleline(&mut self.add_neighbor_input));
//                         ui.text_edit_singleline(&mut self.add_neighbor_input);
//                         let add_btn = ui.add(Button::new("Add sender"));
//                         if add_btn.clicked() {
//                             let neighbor_id = self.add_neighbor_input.parse().unwrap();
//                             // get the NodeIndex of the neighbor and a clone of its Sender
//                             let neighbor_g_idx = self.get_node_idx(neighbor_id);
//                             let neighbor_send_ch =
//                             match self.network.node(neighbor_g_idx).unwrap().payload() {
//                                 WidgetType::Drone(drone_widget) => {
//                                     self.drones_channels[&neighbor_id].2.clone()
//                                 }
//                                 WidgetType::Client(client_widget) => {
//                                     self.clients_channels[&neighbor_id].2.clone()
//                                 }
//                                 WidgetType::Server(server_widget) => {
//                                     self.servers_channels[&neighbor_id].2.clone()
//                                 }
//                             };
                            
//                             let current_node = self.network.node_mut(idx).unwrap().payload_mut();
//                             // get the id of the current and a clone of its Sender
//                             let (current_node_id, current_send_ch) = match current_node {
//                                 WidgetType::Drone(drone_widget) => (
//                                     drone_widget.get_id(),
//                                     self.drones_channels[&drone_widget.get_id()].2.clone(),
//                                 ),
//                                 WidgetType::Client(client_widget) => (
//                                     client_widget.get_id(),
//                                     self.clients_channels[&client_widget.get_id()].2.clone(),
//                                 ),
//                                 WidgetType::Server(server_widget) => (
//                                     server_widget.get_id(),
//                                     self.servers_channels[&server_widget.get_id()].2.clone(),
//                                 ),
//                             };
                            
//                             match current_node {
//                                 WidgetType::Drone(drone_widget) => {
//                                     drone_widget.add_neighbor(neighbor_id, neighbor_send_ch);
//                                 }
//                                 WidgetType::Client(client_widget) => {
//                                     client_widget.add_neighbor(neighbor_id, neighbor_send_ch);
//                                 }
//                                 WidgetType::Server(server_widget) => {
//                                     server_widget.add_neighbor(neighbor_id, neighbor_send_ch);
//                                 }
//                             }
                            
//                             let other_node =
//                             self.network.node_mut(neighbor_g_idx).unwrap().payload_mut();
//                             match other_node {
//                                 WidgetType::Drone(other_drone_widget) => {
//                                     println!("drones_channels: {:?}", self.drones_channels);
//                                     println!("current_node_id: {:?}", current_node_id);
//                                     other_drone_widget.add_neighbor(current_node_id, current_send_ch);
//                                 }
//                                 WidgetType::Client(other_client_widget) => {
//                                     other_client_widget.add_neighbor(current_node_id, current_send_ch);
//                                 }
//                                 WidgetType::Server(other_server_widget) => {
//                                     other_server_widget.add_neighbor(current_node_id, current_send_ch);
//                                 }
//                             }
//                             self.network.add_edge(idx, neighbor_g_idx, ());
//                         }
//                     });

//                     ui.add_space(15.0);

//                     // Remove sender button
//                     ui.vertical(|ui| {
//                         ui.set_max_width(95.0); // Width of the remove button
//                         ui.text_edit_singleline(&mut self.rm_neighbor_input);
//                         let remove_btn = ui.add(Button::new("Remove sender"));
                        
//                         if remove_btn.clicked() {
//                             let neighbor_id = self.rm_neighbor_input.parse().unwrap();
//                             let neighbor_g_idx = self.get_node_idx(neighbor_id);
//                             let current_node = self.network.node_mut(idx).unwrap().payload_mut();
//                             let current_node_id = match current_node {
//                                 WidgetType::Drone(drone_widget) => {
//                                     drone_widget.remove_neighbor(neighbor_id);
//                                     drone_widget.get_id()
//                                 }
//                                 WidgetType::Client(client_widget) => {
//                                     client_widget.remove_neighbor(neighbor_id);
//                                     client_widget.get_id()
//                                 }
//                                 WidgetType::Server(server_widget) => {
//                                     server_widget.remove_neighbor(neighbor_id);
//                                     server_widget.get_id()
//                                 }
//                             };
                            
//                             let other_node =
//                             self.network.node_mut(neighbor_g_idx).unwrap().payload_mut();
//                             match other_node {
//                                 WidgetType::Drone(other_drone_widget) => {
//                                     other_drone_widget.remove_neighbor(current_node_id);
//                                 }
//                                 WidgetType::Client(other_client_widget) => {
//                                     other_client_widget.remove_neighbor(current_node_id);
//                                 }
//                                 WidgetType::Server(other_server_widget) => {
//                                     other_server_widget.remove_neighbor(current_node_id);
//                                 }
//                             }
                            
//                             self.network.remove_edges_between(idx, neighbor_g_idx);
//                         }
//                     });
//                 });
//             }
//         });
//     }
// }

// impl eframe::App for MyApp {
//     fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
//         self.read_data();
//         self.render(ctx);
//     }
// }

#[derive(Debug)]
pub struct SimulationController {
    id: NodeId,
    drones_channels: HashMap<
        NodeId,
        (
            Sender<DroneCommand>,
            Receiver<DroneEvent>,
            Sender<Packet>,
            Receiver<Packet>,
        ),
    >,
    clients_channels: HashMap<
        NodeId,
        (
            Sender<ClientCommand>,
            Receiver<ClientEvent>,
            Sender<Packet>,
            Receiver<Packet>,
        ),
    >,
    servers_channels: HashMap<
        NodeId,
        (
            Sender<ServerCommand>,
            Receiver<ServerEvent>,
            Sender<Packet>,
            Receiver<Packet>,
        ),
    >,
    drones: Vec<Drone>,
    clients: Vec<Client>,
    servers: Vec<Server>,
    widgets: HashMap<NodeId, UWidget>,
    graph: Graph<UWidget, (), Undirected>,
    selected_node: Option<NodeIndex>,
}

impl SimulationController {
    pub fn new(
        id: NodeId,
        drones_channels: HashMap<
            NodeId,
            (
                Sender<DroneCommand>,
                Receiver<DroneEvent>,
                Sender<Packet>,
                Receiver<Packet>,
            ),
        >,
        clients_channels: HashMap<
            NodeId,
            (
                Sender<ClientCommand>,
                Receiver<ClientEvent>,
                Sender<Packet>,
                Receiver<Packet>,
            ),
        >,
        servers_channels: HashMap<
            NodeId,
            (
                Sender<ServerCommand>,
                Receiver<ServerEvent>,
                Sender<Packet>,
                Receiver<Packet>,
            ),
        >,
        drones: Vec<Drone>,
        clients: Vec<Client>,
        servers: Vec<Server>,
    ) -> Self {
        let widgets = generate_widgets(&drones_channels, &clients_channels, &servers_channels);
        let graph = generate_graph(&widgets, &drones, &clients, &servers);
        SimulationController {
            id,
            drones_channels,
            clients_channels,
            servers_channels,
            drones,
            clients,
            servers,
            widgets,
            graph,
            selected_node: Option::default(),
        }
    }

    fn get_node_idx(&self, id: NodeId) -> NodeIndex {
        for (node_idx, widget) in self.graph.nodes_iter() {
            match &*widget.payload().borrow() {
                WidgetType::Drone(drone_widget) => {
                    if drone_widget.get_id() == id {
                        return node_idx;
                    }
                }
                WidgetType::Client(client_widget) => {
                    if client_widget.get_id() == id {
                        return node_idx;
                    }
                }
                WidgetType::Server(server_widget) => {
                    if server_widget.get_id() == id {
                        return node_idx;
                    }
                }
            }
        }
        unreachable!("Se finisci qua rust ha la mamma puttana");
    }

    fn handle_shortcut(&self, id: NodeId, packet: Packet) {
        if let Some(ch) = self.drones_channels.get(&id) {
            ch.2.send(packet).unwrap();
        } else if let Some(ch) = self.clients_channels.get(&id) {
            ch.2.send(packet).unwrap();
        } else if let Some(ch) = self.servers_channels.get(&id) {
            ch.2.send(packet).unwrap();
        }
    }

    fn handle_event(&mut self) {
        let mut event_queue: Vec<(NodeId, Events)> = Vec::new();
        for (drone_id, drone_ch) in &self.drones_channels {
            if let Ok(event) = drone_ch.1.try_recv() {
                event_queue.push((*drone_id, Events::DroneEvent(event)));
            }
        }

        for (client_id, client_ch) in &self.clients_channels {
            if let Ok(event) = client_ch.1.try_recv() {
                event_queue.push((*client_id, Events::ClientEvent(event)));
            }
        }

        for (server_id, server_ch) in &self.servers_channels {
            if let Ok(event) = server_ch.1.try_recv() {
                event_queue.push((*server_id, Events::ServerEvent(event)));
            }
        }

        for (id, event) in event_queue {
            match event {
                Events::DroneEvent(event) => self.handle_drone_event(&id, event),
                Events::ClientEvent(event) => self.handle_client_event(&id, event),
                Events::ServerEvent(event) => self.handle_server_event(&id, event),
            }
        }

    }

    fn handle_drone_event(&self, drone_id: &NodeId, event: DroneEvent) {}
    fn handle_client_event(&mut self, client_id: &NodeId, event: ClientEvent) {
        match event {
            ClientEvent::PacketSent(packet) => {},
            ClientEvent::Shortcut(packet) => {
                let destination_id = packet.routing_header.destination();
                match destination_id {
                    Some(id) => self.handle_shortcut(id, packet),
                    None => unreachable!("Is it possible????"),
                }
            },
            ClientEvent::ClientsConnectedToChatServer(items) => {},
            ClientEvent::ListOfFiles(files, server_id) => {
                println!("Client {} received list of files from server {}: {:?}", client_id, server_id, files);
                // TODO: dont modify the widget directly, modify the graph and then update the widget
                let mut client = self.widgets.get(client_id).unwrap().borrow_mut();
                match *client {
                    WidgetType::Client(ref mut client_widget) => {
                        client_widget.add_list_of_files(server_id, files);
                    }
                    _ => {}
                }
            },
            ClientEvent::FileFromClient(items, _) => {},
            ClientEvent::ServersTypes(types) => {
                // let client_idx = self.get_node_idx(*client_id);
                // let client = self.graph.node_mut(client_idx).unwrap().payload_mut();
                let mut client = self.widgets.get(client_id).unwrap().borrow_mut();
                match *client {
                    WidgetType::Client(ref mut client_widget) => {
                        client_widget.add_server_type(types);
                    }
                    _ => {}
                }
            },
            ClientEvent::WrongClientId => {},
            ClientEvent::UnsupportedRequest => {},
        }
    }
    fn handle_server_event(&self, server_id: &NodeId, event: ServerEvent) {}

    fn read_data(&mut self) {
        if !self.graph.selected_nodes().is_empty() {
            let idx = self.graph.selected_nodes().first().unwrap();
            self.selected_node = Some(*idx);
        }
    }

}

impl eframe::App for SimulationController {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        self.handle_event();
        self.read_data();
        SidePanel::right("Panel").show(ctx, |ui| {
            ui.label("Selected node:");
            if let Some(idx) = self.selected_node {
                let node = self.graph.node_mut(idx).unwrap().payload_mut();
                let mut nw = node.borrow_mut();
                match *nw {
                    WidgetType::Drone(ref mut drone_widget) => drone_widget.draw(ui),
                    WidgetType::Client(ref mut client_widget) => client_widget.draw(ui),
                    WidgetType::Server(ref mut server_widget) => server_widget.draw(ui),
                }
            }
        });
        egui::CentralPanel::default().show(ctx, |ui| {
            let graph_widget: &mut GraphView<
                '_,
                UWidget,
                (),
                petgraph::Undirected,
                u32,
                egui_graphs::DefaultNodeShape,
                egui_graphs::DefaultEdgeShape,
                LayoutStateRandom,
                LayoutRandom,
            > = &mut GraphView::new(&mut self.graph)
                .with_interactions(
                    &SettingsInteraction::default().with_node_selection_enabled(true),
                )
                .with_styles(&SettingsStyle::default().with_labels_always(true))
                .with_navigations(&SettingsNavigation::default().with_zoom_and_pan_enabled(true));
            ui.add(graph_widget);       

        });
    }
}
