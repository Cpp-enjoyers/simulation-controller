#![warn(clippy::pedantic)]

use common::slc_commands::{ChatClientCommand, ChatClientEvent, ServerCommand, ServerEvent, WebClientCommand, WebClientEvent};
use crossbeam_channel::{Receiver, Sender};
use eframe::egui;
use egui::{Button, CentralPanel, RichText, SidePanel, TopBottomPanel};
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

enum UpdateType {
    Add,
    Remove
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
    add_neighbor_error: String,
    rm_neighbor_input: String,
    rm_neighbor_error: String,
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
            add_neighbor_error: String::default(),
            rm_neighbor_input: String::default(),
            rm_neighbor_error: String::default(),
        }
    }

    fn get_node_idx(&self, id: NodeId) -> Option<NodeIndex> {
        for (node_idx, widget) in self.graph.nodes_iter() {
            match widget.payload() {
                WidgetType::Drone(drone_widget) => {
                    if drone_widget.get_id() == id {
                        return Some(node_idx);
                    }
                }
                WidgetType::WebClient(web_client_widget) => {
                    if web_client_widget.get_id() == id {
                        return Some(node_idx);
                    }
                }
                WidgetType::ChatClient(chat_client_widget) => {
                    if chat_client_widget.get_id() == id {
                        return Some(node_idx);
                    }
                }
                WidgetType::Server(server_widget) => {
                    if server_widget.get_id() == id {
                        return Some(node_idx);
                    }
                }
            }
        }
        None
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
                let client_idx = self.get_node_idx(*client_id).unwrap();
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
                let client_idx = self.get_node_idx(*client_id).unwrap();
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

    fn update_neighborhood(&mut self, update_type: UpdateType, source_id: u8, source_idx: NodeIndex, n_id: u8) {
        match update_type {
            UpdateType::Add => {
                match self.graph.node(source_idx).unwrap().payload() {
                    WidgetType::Drone(_) => {
                        if let Some(pos) = self.drones.iter().position(|d| d.id == source_id) {
                            self.drones[pos].connected_node_ids.push(n_id.clone());
                        }
                    },
                    WidgetType::Server(_) => {
                        if let Some(pos) = self.servers.iter().position(|d| d.id == source_id) {
                            self.servers[pos].connected_drone_ids.push(n_id.clone());
                        }
                    },
                    _ => {
                        if let Some(pos) = self.clients.iter().position(|d| d.id == source_id) {
                            self.clients[pos].connected_drone_ids.push(n_id.clone());
                        }
                    }
                }
            },
            UpdateType::Remove => {
                match self.graph.node(source_idx).unwrap().payload() {
                    WidgetType::Drone(_) => {
                        if let Some(pos) = self.drones.iter().position(|d| d.id == source_id) {
                            if let Some(to_remove) = self.drones[pos].connected_node_ids.iter().position(|id| *id == n_id) {
                                self.drones[pos].connected_node_ids.remove(to_remove);
                            }
                        }
                    },
                    WidgetType::Server(_) => {
                        if let Some(pos) = self.servers.iter().position(|s| s.id == source_id) {
                            if let Some(to_remove) = self.servers[pos].connected_drone_ids.iter().position(|id| *id == n_id) {
                                self.servers[pos].connected_drone_ids.remove(to_remove);
                            }
                        }
                    },
                    _ => {
                        if let Some(pos) = self.clients.iter().position(|c| c.id == source_id) {
                            if let Some(to_remove) = self.clients[pos].connected_drone_ids.iter().position(|id| *id == n_id) {
                                self.clients[pos].connected_drone_ids.remove(to_remove);
                            }
                        }
                    }
                }
            },
        }
    }

    /**
     * Here I should validate the input and parse it to a NodeId
     * The input shouldn't be empty and should be a number
     * I should take into account who is trying to add who as a neighbor
     * If the current node is a drone, the neighbor could be drone/client/server
     * If the current node is either a client or a server, the neighbor must be a drone
     * Lastly, the neighbor must exist in the graph
     */
    fn validate_parse_neighbor_id(&mut self, input_neighbor_id: &String) -> Result<(u8, NodeIndex), String> {
        if input_neighbor_id.is_empty() {
            return Err("The input field cannot be empty".to_string());
        }

        // Parse the input to u8, return error if parsing goes wrong
        let neighbor_id = match input_neighbor_id.parse::<u8>(){
            Ok(id) => id,
            Err(_) => return Err("Wrong ID format".to_string()),
        };
        // From the u8 id, retrieve the corresponding NodeIndex in the graph
        let neighbor_idx = match self.get_node_idx(neighbor_id) {
            Some(id) => id,
            None => return Err("ID not found in te graph".to_string()),
        };

        if let Some(current_select_node) = self.selected_node {
            match (self.graph.node(current_select_node).unwrap().payload(), self.graph.node(neighbor_idx).unwrap().payload()) {
                (WidgetType::Drone(drone_widget), _) => {
                    if drone_widget.get_id() == neighbor_id {
                        return Err("Can't create a connection to itself".to_string())
                    }
                    return Ok((neighbor_id, neighbor_idx))
                },

                // Web Client - check if current client has reached it max number of connections (2)
                (WidgetType::WebClient(web_client_widget), WidgetType::Drone(_)) => {
                    let client_id = web_client_widget.get_id();
                    if let Some(pos) = self.clients.iter().position(|cl| cl.id == client_id) {
                        // Check if the current client has reached its max number of connections
                        if self.clients[pos].connected_drone_ids.len() == 2 {
                            return Err(format!("Client {}, reached its max connections", client_id));
                        } else {
                            return Ok((neighbor_id, neighbor_idx));
                        }
                    } else { return Err("Client not found".to_string()) }
                },
                // Here I include all patterns like WebClient/WebClient, WebClient/ChatClient, WebClient/Server.
                (WidgetType::WebClient(_), _) => return Err("Client cannot be connected directly to other client nor server".to_string()),
                
                // Chat Clients - check if current client has reached it max number of connections (2)
                (WidgetType::ChatClient(chat_client_widget), WidgetType::Drone(_)) => {
                    let client_id = chat_client_widget.get_id();
                    if let Some(pos) = self.clients.iter().position(|cl| cl.id == client_id) {
                        // Check if the current client has reached its max number of connections
                        if self.clients[pos].connected_drone_ids.len() == 2 {
                            return Err(format!("Client {}, reached its max connections", client_id));
                        } else {
                            return Ok((neighbor_id, neighbor_idx));
                        }
                    } else { return Err("Client not found".to_string()) }
                },
                // Here I include all patterns like ChatClient/ChatClient, ChatClient/WebClient, ChatClient/Server.
                (WidgetType::ChatClient(_), _) => return Err("Client cannot be connected directly to other client nor server".to_string()),
                
                // Servers - can be connected to any number of drones (but min. 2)
                (WidgetType::Server(_), WidgetType::Drone(_)) => return Ok((neighbor_id, neighbor_idx)),
                (WidgetType::Server(_), _) => return Err("Server cannot be connected directly to other client nor server".to_string()),
            }
        } else {
            return Err("No selected node".to_string());
        }
    }

    fn validate_parse_remove_neighbor_id(&mut self, input_neighbor_id: &String) -> Result<(u8, NodeIndex), String> {
        if input_neighbor_id.is_empty() {
            return Err("The input field cannot be empty".to_string());
        }

        // Parse the input to u8, return error if parsing goes wrong
        let neighbor_id = match input_neighbor_id.parse::<u8>(){
            Ok(id) => id,
            Err(_) => return Err("Wrong ID format".to_string()),
        };
        // From the u8 id, retrieve the corresponding NodeIndex in the graph
        let neighbor_idx = match self.get_node_idx(neighbor_id) {
            Some(id) => id,
            None => return Err("ID not found in the graph".to_string()),
        };

        if let Some(current_selected_node) = self.selected_node {
            match self.graph.node(current_selected_node).unwrap().payload() {
                // For drones I should check if they have at least 2 connections, otherwise the graph becomes disconnected
                WidgetType::Drone(drone_widget) => {
                    let drone_id = drone_widget.get_id();
                    if let Some(pos) = self.drones.iter().position(|d| d.id == drone_id) {
                        if self.drones.get(pos).unwrap().connected_node_ids.len() == 1 {
                            return Err(format!("Cant remove last connection of drone {}!!!", drone_id));
                        } else {
                            return Ok((neighbor_id, neighbor_idx));
                        }
                    } else {
                        return Err("Drone not found".to_string());
                    }
                },
                // For clients I should check that they are connected to at least 1 drone
                WidgetType::WebClient(web_client_widget) => {
                    let client_id = web_client_widget.get_id();
                    if let Some(pos) = self.clients.iter().position(|c| c.id == client_id) {
                        if self.clients.get(pos).unwrap().connected_drone_ids.len() == 1 {
                            return Err(format!("Client {} must have at least 1 connection!", client_id));
                        } else {
                            return Ok((neighbor_id, neighbor_idx));
                        }
                    } else {
                        return Err("Client not found".to_string());
                    }
                },
                WidgetType::ChatClient(chat_client_widget) => {
                    let client_id = chat_client_widget.get_id();
                    if let Some(pos) = self.clients.iter().position(|c| c.id == client_id) {
                        if self.clients.get(pos).unwrap().connected_drone_ids.len() == 1 {
                            return Err(format!("Client {} must have at least 1 connection!", client_id));
                        } else {
                            return Ok((neighbor_id, neighbor_idx));
                        }
                    } else {
                        return Err("Client not found".to_string());
                    }
                },
                WidgetType::Server(server_widget) => {
                    let server_id = server_widget.get_id();
                    if let Some(pos) = self.servers.iter().position(|s| s.id == server_id) {
                        if self.servers.get(pos).unwrap().connected_drone_ids.len() == 2 {
                            return Err(format!("Server {} must have at least 2 connections", server_id));
                        } else {
                            return Ok((neighbor_id, neighbor_idx));
                        }
                    } else {
                        return Err("Server not found".to_string());
                    }
                },
            }
        } else {
            return Err("No selected node".to_string());
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
                    &SettingsInteraction::new()
                    .with_node_selection_enabled(true)
                    .with_dragging_enabled(true),
                )
                .with_styles(&SettingsStyle::new().with_labels_always(true))
                .with_navigations(&SettingsNavigation::new().with_zoom_and_pan_enabled(true));
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
                            match self.validate_parse_neighbor_id(&self.add_neighbor_input.clone()) {
                                Ok((neighbor_id, neighbor_idx)) => {
                                    self.add_neighbor_error = String::new();
                                    // get the NodeIndex of the neighbor and a clone of its Sender
                                    let neighbor_send_ch =
                                    match self.graph.node(neighbor_idx).unwrap().payload() {
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

                                    current_node.add_neighbor_helper(neighbor_id, neighbor_send_ch);
                                    let other_node_widget =
                                    self.graph.node_mut(neighbor_idx).unwrap().payload_mut();
                                    other_node_widget.add_neighbor_helper(current_node_id, current_send_ch);
                                    self.update_neighborhood(UpdateType::Add, current_node_id, idx, neighbor_id);
                                    self.update_neighborhood(UpdateType::Add, neighbor_id, neighbor_idx, current_node_id);
                                    self.graph.add_edge(idx, neighbor_idx, ());
                                },
                                Err(error) => self.add_neighbor_error = error,
                            }
                        }

                        // Display the potential error
                        if !self.add_neighbor_error.is_empty() {
                            ui.label(RichText::new(&self.add_neighbor_error).color(egui::Color32::RED));
                        }
                    });

                    ui.add_space(15.0);

                    // Remove sender button
                    ui.vertical(|ui| {
                        ui.set_max_width(95.0); // Width of the remove button
                        ui.text_edit_singleline(&mut self.rm_neighbor_input);
                        let remove_btn = ui.add(Button::new("Remove sender"));
                        
                        if remove_btn.clicked() {
                            match self.validate_parse_remove_neighbor_id(&self.rm_neighbor_input.clone()) {
                                Ok((neighbor_id, neighbor_idx)) => {
                                    self.rm_neighbor_error = String::new();

                                    // Send command to source to remove neighbor
                                    let current_node = self.graph.node_mut(idx).unwrap().payload_mut();
                                    let current_node_id = current_node.get_id_helper();
                                    current_node.rm_neighbor_helper(neighbor_id);
                                    
                                    // Send command to neighbor to remove source
                                    let other_node = self.graph.node_mut(neighbor_idx).unwrap().payload_mut();
                                    other_node.rm_neighbor_helper(current_node_id);
                                    
                                    // Update state of SCL
                                    self.update_neighborhood(UpdateType::Remove, current_node_id, idx, neighbor_id);
                                    self.update_neighborhood(UpdateType::Remove, neighbor_id, neighbor_idx, current_node_id);
                                    // Update graph visualization
                                    self.graph.remove_edges_between(idx, neighbor_idx);

                                },
                                Err(error) => self.rm_neighbor_error = error,
                            }
                        }

                        // Display the error label
                        if !self.rm_neighbor_error.is_empty() {
                            ui.label(RichText::new(&self.rm_neighbor_error).color(egui::Color32::RED));
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
