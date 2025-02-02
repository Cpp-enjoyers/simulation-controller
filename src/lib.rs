#![warn(clippy::pedantic)]

use common::slc_commands::{ChatClientCommand, ChatClientEvent, ServerCommand, ServerEvent, WebClientCommand, WebClientEvent};
use crossbeam_channel::{Receiver, Sender};
use eframe::egui;
use egui::{Button, CentralPanel, SidePanel, TopBottomPanel};
use egui_graphs::{
    Graph, GraphView, LayoutRandom, LayoutStateRandom, SettingsInteraction, SettingsNavigation,
    SettingsStyle,
};
use petgraph::{
    stable_graph::{NodeIndex, StableUnGraph}, Undirected
};
use std::{collections::{HashMap, HashSet}, fs::File, io::Write, path::Path};
use wg_2024::{
    config::{Client, Drone, Server},
    controller::{DroneCommand, DroneEvent},
    network::NodeId,
    packet::Packet,
};
mod widgets;
use widgets::{drone_widget::DroneWidget, web_client_widget::WebClientWidget, server_widget::ServerWidget, WidgetType};

#[derive(Clone, Debug)]
enum Events {
    DroneEvent(DroneEvent),
    WebClientEvent(WebClientEvent),
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
    web_clients_channels: HashMap<
        NodeId,
        (
            Sender<WebClientCommand>,
            Receiver<WebClientEvent>,
            Sender<Packet>,
            Receiver<Packet>,
        ),
    >,
    chat_clients_channels: HashMap<
        NodeId,
        (
            Sender<ChatClientCommand>,
            Receiver<ChatClientEvent>,
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
            Box::new(|_cc| Ok(Box::new(SimulationController::new(
                id,
                drones_channels,
                web_clients_channels,
                chat_clients_channels,
                servers_channels,
                drones,
                clients,
                servers,
            ))))
        ).expect("Failed to run simulation controller");
}


type DChannels<'a> = &'a HashMap<NodeId, (Sender<DroneCommand>, Receiver<DroneEvent>, Sender<Packet>, Receiver<Packet>)>;
type WCChannels<'a> = &'a HashMap<NodeId, (Sender<WebClientCommand>, Receiver<WebClientEvent>, Sender<Packet>, Receiver<Packet>)>;
type SChannels<'a> = &'a HashMap<NodeId, (Sender<ServerCommand>, Receiver<ServerEvent>, Sender<Packet>, Receiver<Packet>)>;
fn generate_graph(dh: DChannels, ch: WCChannels, sh: SChannels, drones: &Vec<Drone>, clients: &Vec<Client>, servers: &Vec<Server>) -> Graph<WidgetType, (), Undirected> {
    let mut g = StableUnGraph::default();
    let mut h: HashMap<u8, NodeIndex> = HashMap::new();
    let mut edges: HashSet<(u8, u8)> = HashSet::new();

    for (id, channels) in dh {
        let idx = g.add_node(WidgetType::Drone(DroneWidget::new(*id, channels.0.clone())));
        h.insert(*id, idx);
    }

    for (id, channels) in ch {
        let idx = g.add_node(WidgetType::WebClient(WebClientWidget::new(*id, channels.0.clone())));
        h.insert(*id, idx);
    }

    for (id, channels) in sh {
        let idx = g.add_node(WidgetType::Server(ServerWidget {
            id: *id,
            command_ch: channels.0.clone(),
        }));
        h.insert(*id, idx);
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
            let widget = node.payload();
            match widget {
                WidgetType::Drone(d) => (idx, format!("Drone {}", d.get_id())),
                WidgetType::WebClient(wc) => (idx, format!("Web Client {}", wc.get_id())),
                WidgetType::ChatClient(cc) => (idx, format!("Chat Client {}", cc.get_id())),
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

#[derive(Debug)]
struct SimulationController {
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
    web_clients_channels: HashMap<
        NodeId,
        (
            Sender<WebClientCommand>,
            Receiver<WebClientEvent>,
            Sender<Packet>,
            Receiver<Packet>,
        ),
    >,
    chat_clients_channels: HashMap<
        NodeId,
        (
            Sender<ChatClientCommand>,
            Receiver<ChatClientEvent>,
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
    graph: Graph<WidgetType, (), Undirected>,
    selected_node: Option<NodeIndex>,
    add_neighbor_input: String,
    rm_neighbor_input: String,
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
        web_clients_channels: HashMap<
            NodeId,
            (
                Sender<WebClientCommand>,
                Receiver<WebClientEvent>,
                Sender<Packet>,
                Receiver<Packet>,
            ),
        >,
        chat_clients_channels: HashMap<
            NodeId,
            (
                Sender<ChatClientCommand>,
                Receiver<ChatClientEvent>,
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
        let graph = generate_graph(&drones_channels, &web_clients_channels, &servers_channels, &drones, &clients, &servers);
        SimulationController {
            id,
            drones_channels,
            web_clients_channels,
            chat_clients_channels,
            servers_channels,
            drones,
            clients,
            servers,
            graph,
            selected_node: Option::default(),
            add_neighbor_input: String::default(),
            rm_neighbor_input: String::default(),
        }
    }

    fn get_node_idx(&self, id: NodeId) -> NodeIndex {
        for (node_idx, widget) in self.graph.nodes_iter() {
            match widget.payload() {
                WidgetType::Drone(drone_widget) => {
                    if drone_widget.get_id() == id {
                        return node_idx;
                    }
                }
                WidgetType::WebClient(web_client_widget) => {
                    if web_client_widget.get_id() == id {
                        return node_idx;
                    }
                }
                WidgetType::ChatClient(chat_client_widget) => {
                    if chat_client_widget.get_id() == id {
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
        } else if let Some(ch) = self.web_clients_channels.get(&id) {
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

        for (client_id, client_ch) in &self.web_clients_channels {
            if let Ok(event) = client_ch.1.try_recv() {
                event_queue.push((*client_id, Events::WebClientEvent(event)));
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
                Events::WebClientEvent(event) => self.handle_client_event(&id, event),
                Events::ServerEvent(event) => self.handle_server_event(&id, event),
            }
        }

    }

    fn handle_drone_event(&self, drone_id: &NodeId, event: DroneEvent) {
        match event {
            DroneEvent::PacketSent(packet) => {},
            DroneEvent::PacketDropped(packet) => {},
            DroneEvent::ControllerShortcut(packet) => {
                let destination_id = packet.routing_header.destination();
                match destination_id {
                    Some(id) => self.handle_shortcut(id, packet),
                    None => unreachable!("Is it possible????"),
                }
            },
        }
    }

    fn handle_client_event(&mut self, client_id: &NodeId, event: WebClientEvent) {
        match event {
            WebClientEvent::PacketSent(packet) => {},
            WebClientEvent::Shortcut(packet) => {
                let destination_id = packet.routing_header.destination();
                match destination_id {
                    Some(id) => self.handle_shortcut(id, packet),
                    None => unreachable!("Is it possible????"),
                }
            },
            WebClientEvent::ListOfFiles(files, server_id) => {
                println!("Client {} received list of files from server {}: {:?}", client_id, server_id, files);
                let client_idx = self.get_node_idx(*client_id);
                let client = self.graph.node_mut(client_idx).unwrap().payload_mut();
                match client {
                    WidgetType::WebClient(client_widget) => {
                        client_widget.add_list_of_files(server_id, files);
                    }
                    _ => {}
                }
            },
            WebClientEvent::FileFromClient(response, _) => {
                let folder = Path::new("tmp");
                let media_folder = Path::new("tmp/media");
                let (filename, html_file) = response.get_html_file();
                println!("Received file: {}", filename);

                if !folder.exists() {
                    std::fs::create_dir_all(folder).unwrap();
                }

                if !media_folder.exists() {
                    std::fs::create_dir_all(media_folder).unwrap();
                }

                let file_path = folder.join(filename);
                let mut file = File::create(&file_path).unwrap();
                file.write_all(&html_file).unwrap();

                for (media_name, media_content) in response.get_media_files() {
                    let media_path = media_folder.join(media_name);
                    let mut media_file = File::create(&media_path).unwrap();
                    media_file.write_all(media_content).unwrap();
                }

                if webbrowser::open(file_path.to_str().unwrap()).is_err() {
                    println!("Failed to open the file in the browser");
                }
            },
            WebClientEvent::ServersTypes(types) => {
                let client_idx = self.get_node_idx(*client_id);
                let client = self.graph.node_mut(client_idx).unwrap().payload_mut();
                match client {
                    WidgetType::WebClient(client_widget) => {
                        client_widget.add_server_type(types);
                    }
                    _ => {}
                }
            },
            WebClientEvent::UnsupportedRequest => {},
        }
    }
    fn handle_server_event(&self, server_id: &NodeId, event: ServerEvent) {
        match event {
            ServerEvent::PacketSent(packet) => {},
            ServerEvent::ShortCut(packet) => {
                let destination_id = packet.routing_header.destination();
                match destination_id {
                    Some(id) => self.handle_shortcut(id, packet),
                    None => unreachable!("Is it possible????"),
                }
            },
        }
    }

    fn read_data(&mut self) {
        if !self.graph.selected_nodes().is_empty() {
            let idx = self.graph.selected_nodes().first().unwrap();
            self.selected_node = Some(*idx);
        }
    }

    fn render(&mut self, ctx: &egui::Context) {
        SidePanel::right("Panel").show(ctx, |ui| {
            ui.label("Selected node:");
            if let Some(idx) = self.selected_node {
                let node = self.graph.node_mut(idx).unwrap().payload_mut();
                match node {
                    WidgetType::Drone(drone_widget) => ui.add(drone_widget),
                    WidgetType::WebClient(web_client_widget) => ui.add(web_client_widget),
                    WidgetType::ChatClient(chat_client_widget) => ui.add(chat_client_widget),
                    WidgetType::Server(server_widget) => ui.add(server_widget),
                }
            } else {
                ui.label("No node selected")
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
            > = &mut GraphView::new(&mut self.graph)
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
                    // Buttons to add/remove sender
                    ui.vertical(|ui| {
                        ui.set_max_width(71.0); // Width of the add button
                        // ui.add_sized([btn_size.x, btn_size.y], TextEdit::singleline(&mut self.add_neighbor_input));
                        ui.text_edit_singleline(&mut self.add_neighbor_input);
                        let add_btn = ui.add(Button::new("Add sender"));
                        if add_btn.clicked() {
                            let neighbor_id = self.add_neighbor_input.parse().unwrap();
                            // get the NodeIndex of the neighbor and a clone of its Sender
                            let neighbor_g_idx = self.get_node_idx(neighbor_id);
                            let neighbor_send_ch =
                            match self.graph.node(neighbor_g_idx).unwrap().payload() {
                                WidgetType::Drone(_) => {
                                    self.drones_channels[&neighbor_id].2.clone()
                                }
                                WidgetType::WebClient(_) => {
                                    self.web_clients_channels[&neighbor_id].2.clone()
                                }
                                WidgetType::ChatClient(_) => {
                                    self.chat_clients_channels[&neighbor_id].2.clone()
                                }
                                WidgetType::Server(_) => {
                                    self.servers_channels[&neighbor_id].2.clone()
                                }
                            };
                            
                            let current_node = self.graph.node_mut(idx).unwrap().payload_mut();
                            // get the id of the current and a clone of its Sender
                            let (current_node_id, current_send_ch) = match current_node {
                                WidgetType::Drone(drone_widget) => (
                                    drone_widget.get_id(),
                                    self.drones_channels[&drone_widget.get_id()].2.clone(),
                                ),
                                WidgetType::WebClient(web_client_widget) => (
                                    web_client_widget.get_id(),
                                    self.web_clients_channels[&web_client_widget.get_id()].2.clone(),
                                ),
                                WidgetType::ChatClient(chat_client_widget) => (
                                    chat_client_widget.get_id(),
                                    self.chat_clients_channels[&chat_client_widget.get_id()].2.clone(),
                                ),
                                WidgetType::Server(server_widget) => (
                                    server_widget.get_id(),
                                    self.servers_channels[&server_widget.get_id()].2.clone(),
                                ),
                            };
                            
                            match current_node {
                                WidgetType::Drone(drone_widget) => {
                                    drone_widget.add_neighbor(neighbor_id, neighbor_send_ch);
                                }
                                WidgetType::WebClient(web_client_widget) => {
                                    web_client_widget.add_neighbor(neighbor_id, neighbor_send_ch);
                                }
                                WidgetType::ChatClient(chat_client_widget) => {
                                    chat_client_widget.add_neighbor(neighbor_id, neighbor_send_ch);
                                }
                                WidgetType::Server(server_widget) => {
                                    server_widget.add_neighbor(neighbor_id, neighbor_send_ch);
                                }
                            }
                            
                            let other_node =
                            self.graph.node_mut(neighbor_g_idx).unwrap().payload_mut();
                            match other_node {
                                WidgetType::Drone(other_drone_widget) => {
                                    other_drone_widget.add_neighbor(current_node_id, current_send_ch);
                                }
                                WidgetType::WebClient(other_web_client_widget) => {
                                    other_web_client_widget.add_neighbor(current_node_id, current_send_ch);
                                }
                                WidgetType::ChatClient(other_chat_client_widget) => {
                                    other_chat_client_widget.add_neighbor(current_node_id, current_send_ch);
                                }
                                WidgetType::Server(other_server_widget) => {
                                    other_server_widget.add_neighbor(current_node_id, current_send_ch);
                                }
                            }
                            self.graph.add_edge(idx, neighbor_g_idx, ());
                        }
                    });

                    ui.add_space(15.0);

                    // Remove sender button
                    ui.vertical(|ui| {
                        ui.set_max_width(95.0); // Width of the remove button
                        ui.text_edit_singleline(&mut self.rm_neighbor_input);
                        let remove_btn = ui.add(Button::new("Remove sender"));
                        
                        if remove_btn.clicked() {
                            let neighbor_id = self.rm_neighbor_input.parse().unwrap();
                            let neighbor_g_idx = self.get_node_idx(neighbor_id);
                            let current_node = self.graph.node_mut(idx).unwrap().payload_mut();
                            let current_node_id = match current_node {
                                WidgetType::Drone(drone_widget) => {
                                    drone_widget.remove_neighbor(neighbor_id);
                                    drone_widget.get_id()
                                }
                                WidgetType::WebClient(web_client_widget) => {
                                    web_client_widget.remove_neighbor(neighbor_id);
                                    web_client_widget.get_id()
                                }
                                WidgetType::ChatClient(chat_client_widget) => {
                                    chat_client_widget.remove_neighbor(neighbor_id);
                                    chat_client_widget.get_id()
                                }
                                WidgetType::Server(server_widget) => {
                                    server_widget.remove_neighbor(neighbor_id);
                                    server_widget.get_id()
                                }
                            };
                            
                            let other_node =
                            self.graph.node_mut(neighbor_g_idx).unwrap().payload_mut();
                            match other_node {
                                WidgetType::Drone(other_drone_widget) => {
                                    other_drone_widget.remove_neighbor(current_node_id);
                                }
                                WidgetType::WebClient(other_web_client_widget) => {
                                    other_web_client_widget.remove_neighbor(current_node_id);
                                }
                                WidgetType::ChatClient(other_chat_client_widget) => {
                                    other_chat_client_widget.remove_neighbor(current_node_id);
                                }
                                WidgetType::Server(other_server_widget) => {
                                    other_server_widget.remove_neighbor(current_node_id);
                                }
                            }
                            
                            self.graph.remove_edges_between(idx, neighbor_g_idx);
                        }
                    });
                });
            }
        });
    }

}

impl eframe::App for SimulationController {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        self.handle_event();
        self.read_data();
        self.render(ctx);
    }
}
