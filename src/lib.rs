#![warn(clippy::pedantic)]

use ap2024_rustinpeace_nosounddrone::NoSoundDroneRIP;
use common::slc_commands::{
    ChatClientCommand, ChatClientEvent, ServerCommand, ServerEvent, WebClientCommand,
    WebClientEvent,
};
use crossbeam_channel::{Receiver, Sender};
use drone_bettercalldrone::BetterCallDrone;
use eframe::egui;
use egui::{
    Button, CentralPanel, Color32, Layout, RichText, ScrollArea, SidePanel, TextStyle, TopBottomPanel
};
use egui_graphs::{
    Graph, GraphView, LayoutRandom, LayoutStateRandom, SettingsInteraction, SettingsNavigation,
    SettingsStyle,
};
use getdroned::GetDroned;
use petgraph::{
    graph::EdgeIndex,
    stable_graph::{NodeIndex, StableUnGraph},
    Undirected,
};
use rand::Rng;
use rolling_drone::RollingDrone;
use rust_do_it::RustDoIt;
use rust_roveri::RustRoveri;
use rustafarian_drone::RustafarianDrone;
use rusteze_drone::RustezeDrone;
use rusty_drones::RustyDrone;
use std::{
    collections::{HashMap, HashSet, VecDeque},
    fs::File,
    io::Write,
    path::Path,
};
use utils::EventQueue;
use wg_2024::{
    config::{Client, Drone, Server},
    controller::{DroneCommand, DroneEvent},
    drone::Drone as DroneTrait,
    network::NodeId,
    packet::Packet,
};
pub mod widgets;
use widgets::{
    chat_client_widget::ChatClientWidget, drone_widget::DroneWidget, server_widget::ServerWidget,
    web_client_widget::WebClientWidget, WidgetType,
};
pub mod utils;

use dr_ones::Drone as DrDrone;

#[derive(Clone, Debug)]
enum Events {
    Drone(DroneEvent),
    WebClient(WebClientEvent),
    ChatClient(ChatClientEvent),
    Server(ServerEvent),
}

enum UpdateType {
    Add,
    Remove,
}

// Type aliases for the channels
type DChannels = HashMap<
    NodeId,
    (
        Sender<DroneCommand>,
        Receiver<DroneEvent>,
        Sender<Packet>,
        Receiver<Packet>,
    ),
>;
type WCChannels = HashMap<
    NodeId,
    (
        Sender<WebClientCommand>,
        Receiver<WebClientEvent>,
        Sender<Packet>,
        Receiver<Packet>,
    ),
>;
type CCChannels = HashMap<
    NodeId,
    (
        Sender<ChatClientCommand>,
        Receiver<ChatClientEvent>,
        Sender<Packet>,
        Receiver<Packet>,
    ),
>;
type SChannels = HashMap<
    NodeId,
    (
        Sender<ServerCommand>,
        Receiver<ServerEvent>,
        Sender<Packet>,
        Receiver<Packet>,
    ),
>;

/// Function to run the simulation controller
///
/// # Panics
/// The function panics if the GUI fails to run
pub fn run(
    drones_channels: DChannels,
    web_clients_channels: WCChannels,
    chat_clients_channels: CCChannels,
    servers_channels: SChannels,
    drones: Vec<Drone>,
    clients: Vec<Client>,
    servers: Vec<Server>,
) {
    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "Simulation Controller",
        options,
        Box::new(|_cc| {
            Ok(Box::new(SimulationController::new(
                drones_channels,
                web_clients_channels,
                chat_clients_channels,
                servers_channels,
                drones,
                clients,
                servers,
            )))
        }),
    )
    .expect("Failed to run simulation controller");
}

/// This function generate the graph from the channels and the nodes
fn generate_graph(
    dh: &DChannels,
    wch: &WCChannels,
    cch: &CCChannels,
    sh: &SChannels,
    drones: &Vec<Drone>,
    clients: &Vec<Client>,
    servers: &Vec<Server>,
) -> Graph<WidgetType, (), Undirected> {
    let mut g = StableUnGraph::default();
    let mut h: HashMap<u8, NodeIndex> = HashMap::new();
    let mut edges: HashSet<(u8, u8)> = HashSet::new();
    
    
    // Create drone widgets
    for (id, channels) in dh {
        let idx = g.add_node(WidgetType::Drone(DroneWidget::new(*id, channels.0.clone())));
        h.insert(*id, idx);
    }
    // Create web client widgets
    for (id, channels) in wch {
        let idx = g.add_node(WidgetType::WebClient(WebClientWidget::new(
            *id,
            channels.0.clone(),
        )));
        h.insert(*id, idx);
    }
    // Create chat client widgets
    for (id, channels) in cch {
        let idx = g.add_node(WidgetType::ChatClient(ChatClientWidget::new(
            *id,
            channels.0.clone(),
        )));
        h.insert(*id, idx);
    }
    // Create server widgets
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
    for (idx, label) in &temp {
        eg_graph.node_mut(*idx).unwrap().set_label(label.clone());
    }

    eg_graph
}

type DroneFactory = fn(
    u8,
    Sender<DroneEvent>,
    Receiver<DroneCommand>,
    Receiver<Packet>,
    HashMap<u8, Sender<Packet>>,
    f32,
) -> Box<dyn DroneTrait>;
const DRONE_FACTORY: [DroneFactory; 10] = [
    create_boxed_drone!(DrDrone),
    create_boxed_drone!(RustDoIt),
    create_boxed_drone!(RustRoveri),
    create_boxed_drone!(RollingDrone),
    create_boxed_drone!(RustafarianDrone),
    create_boxed_drone!(RustezeDrone),
    create_boxed_drone!(RustyDrone),
    create_boxed_drone!(GetDroned),
    create_boxed_drone!(NoSoundDroneRIP),
    create_boxed_drone!(BetterCallDrone),
];

struct SimulationController {
    drones_channels: DChannels,
    web_clients_channels: WCChannels,
    chat_clients_channels: CCChannels,
    servers_channels: SChannels,
    drones: Vec<Drone>,
    clients: Vec<Client>,
    servers: Vec<Server>,
    graph: Graph<WidgetType, (), Undirected>,
    selected_node: Option<NodeIndex>,
    selected_edge: Option<EdgeIndex>,
    add_neighbor_input: String,
    add_neighbor_error: String,
    rm_neighbor_error: String,
    drone_crash_error: String,
    events: EventQueue<RichText>,
}

impl SimulationController {
    pub fn new(
        drones_channels: DChannels,
        web_clients_channels: WCChannels,
        chat_clients_channels: CCChannels,
        servers_channels: SChannels,
        drones: Vec<Drone>,
        clients: Vec<Client>,
        servers: Vec<Server>,
    ) -> Self {
        let graph = generate_graph(
            &drones_channels,
            &web_clients_channels,
            &chat_clients_channels,
            &servers_channels,
            &drones,
            &clients,
            &servers,
        );
        SimulationController {
            drones_channels,
            web_clients_channels,
            chat_clients_channels,
            servers_channels,
            drones,
            clients,
            servers,
            graph,
            selected_node: Option::default(),
            selected_edge: Option::default(),
            add_neighbor_input: String::default(),
            add_neighbor_error: String::default(),
            rm_neighbor_error: String::default(),
            drone_crash_error: String::default(),
            events: EventQueue::new(100),
        }
    }

    /// Helper function to get the index of a node given its id
    ///
    /// The `NodeIndex` is the index used by the graph library to identify a node
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

    /// Utility function to get the type of the `Packet`
    /// Used for logging purposes
    fn get_pack_type(packet: &Packet) -> String {
        match &packet.pack_type {
            wg_2024::packet::PacketType::MsgFragment(_) => String::from("MsgFragment"),
            wg_2024::packet::PacketType::Ack(_) => String::from("Ack"),
            wg_2024::packet::PacketType::Nack(_) => String::from("Nack"),
            wg_2024::packet::PacketType::FloodRequest(_) => String::from("FloodRequest"),
            wg_2024::packet::PacketType::FloodResponse(_) => String::from("FloodResponse"),
        }
    }

    /// Function to handle the shortcut of a packet
    /// The packet is sent to the corresponding node
    fn handle_shortcut(&self, id: NodeId, packet: Packet) {
        if let Some(ch) = self.drones_channels.get(&id) {
            ch.2.send(packet).unwrap();
        } else if let Some(ch) = self.web_clients_channels.get(&id) {
            ch.2.send(packet).unwrap();
        } else if let Some(ch) = self.servers_channels.get(&id) {
            ch.2.send(packet).unwrap();
        }
    }

    /// Function to handle all the incoming events
    ///
    /// Each time the GUI is refreshed, this function is called.
    /// It listens to all the channels of the drones, web clients, chat clients and servers,
    /// storing the received events in a queue.
    /// Then for each event in the queue, it calls the corresponding handler function.
    fn handle_event(&mut self) {
        let mut event_queue: Vec<(NodeId, Events)> = Vec::new();
        for (drone_id, drone_ch) in &self.drones_channels {
            if let Ok(event) = drone_ch.1.try_recv() {
                event_queue.push((*drone_id, Events::Drone(event)));
            }
        }

        for (client_id, client_ch) in &self.web_clients_channels {
            if let Ok(event) = client_ch.1.try_recv() {
                event_queue.push((*client_id, Events::WebClient(event)));
            }
        }

        for (client_id, client_ch) in &self.chat_clients_channels {
            if let Ok(event) = client_ch.1.try_recv() {
                event_queue.push((*client_id, Events::ChatClient(event)));
            }
        }

        for (server_id, server_ch) in &self.servers_channels {
            if let Ok(event) = server_ch.1.try_recv() {
                event_queue.push((*server_id, Events::Server(event)));
            }
        }

        for (id, event) in event_queue {
            match event {
                Events::Drone(event) => self.handle_drone_event(id, event),
                Events::WebClient(event) => self.handle_web_client_event(id, event),
                Events::ChatClient(event) => self.handle_chat_client_event(id, event),
                Events::Server(event) => self.handle_server_event(id, event),
            }
        }
    }

    /// Handler function for the drone events
    fn handle_drone_event(&mut self, drone_id: NodeId, event: DroneEvent) {
        match event {
            DroneEvent::PacketSent(packet) => {
                let packet_type = SimulationController::get_pack_type(&packet);
                let event_string = format!("[DRONE: {drone_id}] Sent {packet_type} packet");
                let event_label = RichText::new(event_string);
                self.events.push(event_label);
            }
            DroneEvent::PacketDropped(packet) => {
                let packet_type = SimulationController::get_pack_type(&packet);
                let event_string = format!("[DRONE: {drone_id}] Dropped {packet_type} packet");
                let event_label = RichText::new(event_string).color(Color32::RED);
                self.events.push(event_label);
            }
            DroneEvent::ControllerShortcut(packet) => {
                let packet_type = SimulationController::get_pack_type(&packet);
                let destination_id = packet.routing_header.destination();
                match destination_id {
                    Some(id) => {
                        let event_string = format!("[DRONE: {drone_id}] Requested shortcut for packet {packet_type} to {id}");
                        let event_label = RichText::new(event_string).color(Color32::ORANGE);
                        self.events.push(event_label);
                        self.handle_shortcut(id, packet);
                    }
                    None => unreachable!("Is it possible????"),
                }
            }
        }
    }

    /// Handler function for the web client events
    fn handle_web_client_event(&mut self, client_id: NodeId, event: WebClientEvent) {
        match event {
            WebClientEvent::PacketSent(packet) => {
                let packet_type = SimulationController::get_pack_type(&packet);
                let event_string = format!("[WEB CLIENT: {client_id}] Sent {packet_type} packet");
                let event_label = RichText::new(event_string);
                self.events.push(event_label);
            }
            WebClientEvent::Shortcut(packet) => {
                let packet_type = SimulationController::get_pack_type(&packet);
                let destination_id = packet.routing_header.destination();
                match destination_id {
                    Some(id) => {
                        let event_string = format!("[WEB CLIENT: {client_id}] Requested shortcut for packet {packet_type} to {id}");
                        let event_label = RichText::new(event_string).color(Color32::ORANGE);
                        self.events.push(event_label);
                        self.handle_shortcut(id, packet);
                    }
                    None => unreachable!("Is it possible????"),
                }
            }
            WebClientEvent::ListOfFiles(files, server_id) => {
                let client_idx = self.get_node_idx(client_id).unwrap();
                let client = self.graph.node_mut(client_idx).unwrap().payload_mut();

                if let WidgetType::WebClient(client_widget) = client {
                    client_widget.add_list_of_files(server_id, files);
                }
            }
            WebClientEvent::FileFromClient(response, _) => {
                let folder = Path::new("tmp");
                let media_folder = Path::new("tmp/media");
                let (filename, html_file) = response.get_html_file();

                if !folder.exists() {
                    std::fs::create_dir_all(folder).unwrap();
                }

                if !media_folder.exists() {
                    std::fs::create_dir_all(media_folder).unwrap();
                }

                let file_path = folder.join(filename);
                let mut file = File::create(&file_path).unwrap();
                file.write_all(html_file).unwrap();

                for (media_name, media_content) in response.get_media_files() {
                    let media_path = media_folder.join(media_name);
                    let mut media_file = File::create(&media_path).unwrap();
                    media_file.write_all(media_content).unwrap();
                }

                if webbrowser::open(file_path.to_str().unwrap()).is_err() {
                    println!("Failed to open the file in the browser");
                }
            }
            WebClientEvent::ServersTypes(types) => {
                let client_idx = self.get_node_idx(client_id).unwrap();
                let client = self.graph.node_mut(client_idx).unwrap().payload_mut();

                if let WidgetType::WebClient(client_widget) = client {
                    client_widget.add_server_type(types);
                }
            }
            WebClientEvent::UnsupportedRequest => {
                let client_idx = self.get_node_idx(client_id).unwrap();
                let client = self.graph.node_mut(client_idx).unwrap().payload_mut();

                if let WidgetType::WebClient(client_widget) = client {
                    client_widget.add_unsupported_request_error("Unsupported request".to_string());
                }
            }
        }
    }

    /// Handler function for the chat client events
    fn handle_chat_client_event(&mut self, chat_client_id: NodeId, event: ChatClientEvent) {
        match event {
            ChatClientEvent::PacketSent(packet) => {
                let packet_type = SimulationController::get_pack_type(&packet);
                let event_string =
                    format!("[CHAT CLIENT: {chat_client_id}] Sent {packet_type} packet");
                let event_label = RichText::new(event_string);
                self.events.push(event_label);
            }
            ChatClientEvent::Shortcut(packet) => {
                let packet_type = SimulationController::get_pack_type(&packet);
                let destination_id = packet.routing_header.destination();
                match destination_id {
                    Some(id) => {
                        let event_string = format!("[CHAT CLIENT: {chat_client_id}] Requested shortcut for packet {packet_type} to {id}");
                        let event_label = RichText::new(event_string).color(Color32::ORANGE);
                        self.events.push(event_label);
                        self.handle_shortcut(id, packet);
                    }
                    None => unreachable!("Is it possible????"),
                }
            }
            ChatClientEvent::ServersTypes(types) => {
                let client_idx = self.get_node_idx(chat_client_id).unwrap();
                let client = self.graph.node_mut(client_idx).unwrap().payload_mut();

                if let WidgetType::ChatClient(client_widget) = client {
                    client_widget.add_server_type(&types);
                }
            }
            ChatClientEvent::UnsupportedRequest => {}
            ChatClientEvent::MessageReceived(msg) => {
                let client_idx = self.get_node_idx(chat_client_id).unwrap();
                let client = self.graph.node_mut(client_idx).unwrap().payload_mut();

                if let WidgetType::ChatClient(client_widget) = client {
                    client_widget.update_chat(msg);
                }
            }
        }
    }

    /// Handler function for the server events
    fn handle_server_event(&mut self, server_id: NodeId, event: ServerEvent) {
        match event {
            ServerEvent::PacketSent(packet) => {
                let packet_type = SimulationController::get_pack_type(&packet);
                let event_string = format!("[SERVER: {server_id}] Sent {packet_type} packet");
                let event_label = RichText::new(event_string);
                self.events.push(event_label);
            }
            ServerEvent::ShortCut(packet) => {
                let packet_type = SimulationController::get_pack_type(&packet);
                let destination_id = packet.routing_header.destination();
                match destination_id {
                    Some(id) => {
                        let event_string = format!("[SERVER: {server_id}] Requested shortcut for packet {packet_type} to {id}");
                        let event_label = RichText::new(event_string).color(Color32::ORANGE);
                        self.events.push(event_label);
                        self.handle_shortcut(id, packet);
                    }
                    None => unreachable!("Is it possible????"),
                }
            }
        }
    }

    /// Function used to update the neighborhood of a node
    ///
    /// The neighborhood of a node is the set of nodes that are connected to it.
    /// This function handles the addition and removal of nodes from the neighborhood,
    /// by using the `UpdateType` enum to distinguish between the two cases.
    fn update_neighborhood(
        &mut self,
        update_type: &UpdateType,
        source_id: u8,
        source_idx: NodeIndex,
        n_id: u8,
    ) {
        match update_type {
            UpdateType::Add => match self.graph.node(source_idx).unwrap().payload() {
                WidgetType::Drone(_) => {
                    if let Some(pos) = self.drones.iter().position(|d| d.id == source_id) {
                        self.drones[pos].connected_node_ids.push(n_id);
                    }
                }
                WidgetType::Server(_) => {
                    if let Some(pos) = self.servers.iter().position(|d| d.id == source_id) {
                        self.servers[pos].connected_drone_ids.push(n_id);
                    }
                }
                _ => {
                    if let Some(pos) = self.clients.iter().position(|d| d.id == source_id) {
                        self.clients[pos].connected_drone_ids.push(n_id);
                    }
                }
            },
            UpdateType::Remove => match self.graph.node(source_idx).unwrap().payload() {
                WidgetType::Drone(_) => {
                    if let Some(pos) = self.drones.iter().position(|d| d.id == source_id) {
                        if let Some(to_remove) = self.drones[pos]
                            .connected_node_ids
                            .iter()
                            .position(|id| *id == n_id)
                        {
                            self.drones[pos].connected_node_ids.remove(to_remove);
                        }
                    }
                }
                WidgetType::Server(_) => {
                    if let Some(pos) = self.servers.iter().position(|s| s.id == source_id) {
                        if let Some(to_remove) = self.servers[pos]
                            .connected_drone_ids
                            .iter()
                            .position(|id| *id == n_id)
                        {
                            self.servers[pos].connected_drone_ids.remove(to_remove);
                        }
                    }
                }
                _ => {
                    if let Some(pos) = self.clients.iter().position(|c| c.id == source_id) {
                        if let Some(to_remove) = self.clients[pos]
                            .connected_drone_ids
                            .iter()
                            .position(|id| *id == n_id)
                        {
                            self.clients[pos].connected_drone_ids.remove(to_remove);
                        }
                    }
                }
            },
        }
    }

    /// Function to validate the input of the user when adding a neighbor to a node
    ///
    /// The input should not be empty
    /// The input should be a valid u8 number
    /// The input should be a valid id of a node in the graph
    fn validate_add_sender_input(&self, input_neighbor_id: &str) -> Result<NodeIndex, String> {
        if input_neighbor_id.is_empty() {
            return Err("The input field cannot be empty".to_string());
        }

        // Parse the input to u8, return error if parsing goes wrong
        let Ok(neighbor_id) = input_neighbor_id.parse::<u8>() else {
            return Err("Wrong ID format".to_string());
        };

        // From the u8 id, retrieve the corresponding NodeIndex in the graph
        let Some(neighbor_idx) = self.get_node_idx(neighbor_id) else {
            return Err("ID not found in te graph".to_string());
        };

        Ok(neighbor_idx)
    }

    /// Function used to verify if a client can add a new sender
    ///
    /// A client can add a new sender if it has less than 2 connections
    fn can_client_add_sender(&self, client_id: NodeId) -> Result<u8, String> {
        if let Some(client_pos) = self.clients.iter().position(|c| c.id == client_id) {
            if self.clients[client_pos].connected_drone_ids.len() == 2 {
                Err(format!("Client {client_id} reached its max connections"))
            } else {
                Ok(client_id)
            }
        } else {
            Err("Client not found".to_string())
        }
    }

    /// Function to check if a sender can be added to a node
    ///
    /// It checks if the sender and the neighbor can be connected
    /// based on the type of the nodes.
    /// Drones can be connected to drones, clients and servers.
    /// Clients can be connected only to drones. (max. 2 connections)
    /// Servers can be connected only to drones.
    fn can_add_sender(
        &self,
        source_idx: NodeIndex,
        neighbor_idx: NodeIndex,
    ) -> Result<(NodeIndex, NodeIndex), String> {
        match (
            self.graph.node(source_idx).unwrap().payload(),
            self.graph.node(neighbor_idx).unwrap().payload(),
        ) {
            (WidgetType::Drone(_), WidgetType::Drone(_)) => {
                // Avoid creating a connection to itself
                if source_idx == neighbor_idx {
                    return Err("Can't create a connection to itself".to_string());
                }
                Ok((source_idx, neighbor_idx))
            }
            // For clients, check if the client has reached its max number of connections (2)
            (WidgetType::Drone(_), WidgetType::WebClient(web_client_widget))
            | (WidgetType::WebClient(web_client_widget), WidgetType::Drone(_)) => {
                let client_id = web_client_widget.get_id();

                match self.can_client_add_sender(client_id) {
                    Ok(_) => Ok((source_idx, neighbor_idx)),
                    Err(e) => Err(e),
                }
            }
            // For clients, check if the client has reached its max number of connections (2)
            (WidgetType::Drone(_), WidgetType::ChatClient(chat_client_widget))
            | (WidgetType::ChatClient(chat_client_widget), WidgetType::Drone(_)) => {
                let client_id = chat_client_widget.get_id();

                match self.can_client_add_sender(client_id) {
                    Ok(_) => Ok((source_idx, neighbor_idx)),
                    Err(e) => Err(e),
                }
            }
            (WidgetType::Drone(_), WidgetType::Server(_))
            | (WidgetType::Server(_), WidgetType::Drone(_)) => Ok((source_idx, neighbor_idx)),
            // Server can be connected to any number of drones, but not to other clients or servers
            (WidgetType::Server(_), _) => {
                Err("Server cannot be connected directly to other client nor server".to_string())
            }

            // Here I include all patterns like ChatClient/ChatClient, ChatClient/WebClient, ChatClient/Server.
            // and all patterns like WebClient/WebClient, WebClient/ChatClient, WebClient/Server.
            (WidgetType::ChatClient(_) | WidgetType::WebClient(_), _) => {
                Err("Client cannot be connected directly to other client nor server".to_string())
            }
        }
    }

    /// This function checks if an edge can be added between two nodes
    ///
    /// First, it checks if the input is valid, calling the `validate_add_sender_input` function.
    /// Then, it checks if the nodes can be connected, calling the `can_add_sender` function.
    fn validate_add_sender(
        &mut self,
        source_idx: NodeIndex,
        input_neighbor_id: &str,
    ) -> Result<(NodeIndex, NodeIndex), String> {
        let neighbor_idx = self.validate_add_sender_input(input_neighbor_id)?;
        
        // check if the two nodes are already connected
        if self.graph.edges_connecting(source_idx, neighbor_idx).count() > 0 {
            return Err("Nodes are already connected".to_string());
        }
        
        self.can_add_sender(source_idx, neighbor_idx)
    }

    /// Helper function to get the sender channel of a node and the corresponding `NodeId`
    fn get_sender_channel(&self, idx: NodeIndex) -> (NodeId, Sender<Packet>) {
        match self.graph.node(idx).unwrap().payload() {
            WidgetType::Drone(dw) => (dw.get_id(), self.drones_channels[&dw.get_id()].2.clone()),
            WidgetType::WebClient(wcw) => (
                wcw.get_id(),
                self.web_clients_channels[&wcw.get_id()].2.clone(),
            ),
            WidgetType::ChatClient(ccw) => (
                ccw.get_id(),
                self.chat_clients_channels[&ccw.get_id()].2.clone(),
            ),
            WidgetType::Server(sw) => (sw.get_id(), self.servers_channels[&sw.get_id()].2.clone()),
        }
    }

    /// Function that checks if the removal of the edge would make some servers/clients unreachable
    /// Furthermore, it that checks if the graph would become disconnected if the edge is removed.
    fn check_connectivity(&self, edge_to_remove: EdgeIndex) -> Result<(), String> {
        let mut copy_graph = self.graph.clone();
        copy_graph.remove_edge(edge_to_remove).unwrap();

        // For each client, perform a DFS to check if it can reach every server
        for client in &self.clients {
            let client_idx = self.get_node_idx(client.id).unwrap();
            let mut visited: HashSet<NodeIndex> = HashSet::new();
            let mut servers_visited: HashSet<NodeId> = HashSet::new();
            let mut stack: VecDeque<NodeIndex> = VecDeque::new();
            stack.push_back(client_idx);

            while let Some(node) = stack.pop_front() {
                if visited.insert(node) {
                    let neighbors = copy_graph.g.neighbors(node).collect::<Vec<NodeIndex>>();
                    for neighbor in neighbors {
                        if let WidgetType::Server(server_widget) =
                            copy_graph.node(neighbor).unwrap().payload()
                        {
                            servers_visited.insert(server_widget.get_id());
                        } else if let WidgetType::ChatClient(_) | WidgetType::WebClient(_) =
                            copy_graph.node(neighbor).unwrap().payload()
                        {
                            continue;
                        } else {
                            stack.push_front(neighbor);
                        }
                    }
                }
            }

            // Check if the client can reach every server
            if servers_visited.len() != self.servers.len() {
                return Err(format!(
                    "By removing edge {}, client {} wouldn't reach every server",
                    edge_to_remove.index(),
                    client.id
                ));
            }
        }

        // Check if graph is still connected
        let cc = petgraph::algo::tarjan_scc(&copy_graph.g);
        if cc.len() > 1 {
            return Err("By removing the edge, the graph would become disconnected".to_string());
        }

        Ok(())
    }

    /// Function to check if a node can remove a sender
    ///
    /// For drones, they must have at least 1 connection, otherwise the graph becomes disconnected.
    /// For clients, they must have at least 1 connection to a drone.
    /// For servers, they must have at least 2 connections to drones.
    fn can_remove_sender(&self, node_idx: NodeIndex) -> Result<u8, String> {
        match self.graph.node(node_idx).unwrap().payload() {
            // For drones I should check if they have at least 1 connection, otherwise the graph becomes disconnected
            WidgetType::Drone(drone_widget) => {
                let drone_id = drone_widget.get_id();
                if let Some(pos) = self.drones.iter().position(|d| d.id == drone_id) {
                    if self.drones.get(pos).unwrap().connected_node_ids.len() == 1 {
                        Err(format!("Cant remove last connection of drone {drone_id}!"))
                    } else {
                        Ok(drone_id)
                    }
                } else {
                    Err("Drone not found".to_string())
                }
            }
            // For clients I should check that they are connected to at least 1 drone
            WidgetType::WebClient(web_client_widget) => {
                let client_id = web_client_widget.get_id();
                if let Some(pos) = self.clients.iter().position(|c| c.id == client_id) {
                    if self.clients.get(pos).unwrap().connected_drone_ids.len() == 1 {
                        Err(format!(
                            "Client {client_id} must have at least 1 connection!"
                        ))
                    } else {
                        Ok(client_id)
                    }
                } else {
                    Err("Client not found".to_string())
                }
            }
            WidgetType::ChatClient(chat_client_widget) => {
                let client_id = chat_client_widget.get_id();
                if let Some(pos) = self.clients.iter().position(|c| c.id == client_id) {
                    if self.clients.get(pos).unwrap().connected_drone_ids.len() == 1 {
                        Err(format!(
                            "Client {client_id} must have at least 1 connection!"
                        ))
                    } else {
                        Ok(client_id)
                    }
                } else {
                    Err("Client not found".to_string())
                }
            }
            WidgetType::Server(server_widget) => {
                let server_id = server_widget.get_id();
                if let Some(pos) = self.servers.iter().position(|s| s.id == server_id) {
                    if self.servers.get(pos).unwrap().connected_drone_ids.len() == 2 {
                        Err(format!(
                            "Server {server_id} must have at least 2 connections"
                        ))
                    } else {
                        Ok(server_id)
                    }
                } else {
                    Err("Server not found".to_string())
                }
            }
        }
    }

    /// This function checks if an edge can be removed
    /// First it checks if the graph would become disconnected.
    /// The graph becomes disconnected if the removal of the edge would create more than 1 connected component.
    /// Or if the removal of the edge would make a client unable to reach every server.
    /// Then it checks if the nodes (endpoints of the edge) can remove each other.
    /// For drones, they must have at least 1 connection, otherwise the graph becomes disconnected.
    /// For clients, they must have at least 1 connection to a drone.
    /// For servers, they must have at least 2 connections to drones.
    fn validate_edge_removal(&mut self, edge: EdgeIndex) -> Result<(u8, u8), String> {
        // Check if without the edge, every client can still reach every server
        self.check_connectivity(edge)?;

        // Take the 2 endpoints of the edge to be removed
        let (node_1, node_2) = self.graph.edge_endpoints(edge).unwrap();

        match (
            self.can_remove_sender(node_1),
            self.can_remove_sender(node_2),
        ) {
            (Ok(id_1), Ok(id_2)) => Ok((id_1, id_2)),
            (Ok(_), Err(e)) | (Err(e), Ok(_)) => Err(e),
            (Err(_), Err(_)) => Err("Either nodes can't remove each other".to_string()),
        }
    }

    fn can_drone_crash(&self, drone_id: NodeId) -> Result<(), String> {
        let drone_idx = self.get_node_idx(drone_id).unwrap();

        // Check if the neighbors of the drone can remove it
        let neighbors = self
            .graph
            .g
            .neighbors(drone_idx)
            .collect::<Vec<NodeIndex>>();
        for neighbor in neighbors {
            match self.graph.node(neighbor).unwrap().payload() {
                WidgetType::Drone(drone_widget) => {
                    let id = drone_widget.get_id();
                    if let Some(pos) = self.drones.iter().position(|d| d.id == id) {
                        if self.drones[pos].connected_node_ids.len() == 1 {
                            return Err(format!("Drone {id} must have at least 1 connection"));
                        }
                    }
                }
                WidgetType::WebClient(web_client_widget) => {
                    let id = web_client_widget.get_id();
                    if let Some(pos) = self.clients.iter().position(|wc| wc.id == id) {
                        if self.clients[pos].connected_drone_ids.len() == 1 {
                            return Err(format!("Client {id} must have at least 1 connection"));
                        }
                    }
                }
                WidgetType::ChatClient(chat_client_widget) => {
                    let id = chat_client_widget.get_id();
                    if let Some(pos) = self.clients.iter().position(|cc| cc.id == id) {
                        if self.clients[pos].connected_drone_ids.len() == 1 {
                            return Err(format!("Client {id} must have at least 1 connection"));
                        }
                    }
                }
                WidgetType::Server(server_widget) => {
                    let id = server_widget.get_id();
                    if let Some(pos) = self.servers.iter().position(|s| s.id == id) {
                        if self.servers[pos].connected_drone_ids.len() == 2 {
                            return Err(format!("Server {id} must have at least 2 connections"));
                        }
                    }
                }
            }
        }

        let mut copy_graph = self.graph.clone();
        copy_graph.remove_node(drone_idx);

        // check connectivity between clients and servers
        for client in &self.clients {
            let client_idx = self.get_node_idx(client.id).unwrap();
            let mut visited: HashSet<NodeIndex> = HashSet::new();
            let mut servers_visited: HashSet<NodeId> = HashSet::new();
            let mut stack: VecDeque<NodeIndex> = VecDeque::new();
            stack.push_back(client_idx);

            while let Some(node) = stack.pop_front() {
                if visited.insert(node) {
                    let neighbors = copy_graph.g.neighbors(node).collect::<Vec<NodeIndex>>();
                    for neighbor in neighbors {
                        if let WidgetType::Server(server_widget) =
                            copy_graph.node(neighbor).unwrap().payload()
                        {
                            servers_visited.insert(server_widget.get_id());
                        } else if let WidgetType::ChatClient(_) | WidgetType::WebClient(_) =
                            copy_graph.node(neighbor).unwrap().payload()
                        {
                            continue;
                        } else {
                            stack.push_front(neighbor);
                        }
                    }
                }
            }

            // Check if the client can reach every server
            if servers_visited.len() != self.servers.len() {
                return Err(format!(
                    "By removing drone {}, client {} wouldn't reach every server",
                    drone_idx.index(),
                    client.id
                ));
            }
        }

        // check if graph is still connected
        let cc = petgraph::algo::tarjan_scc(&copy_graph.g);
        if cc.len() > 1 {
            return Err(format!(
                "By removing drone {}, the graph would become disconnected",
                drone_idx.index()
            ));
        }

        Ok(())
    }

    /// Function to crash a drone
    ///
    /// When a drone crashes, it sends a crash command to the mimicked drone.
    /// Then, it removes the drone from the graph and updates the neighbors of the drone.
    fn crash_drone(&mut self, crashing_drone: NodeIndex) {
        let drone = self.graph.node(crashing_drone).unwrap().payload();
        let neighbors = self
            .graph
            .g
            .neighbors(crashing_drone)
            .collect::<Vec<NodeIndex>>();
        match drone {
            WidgetType::Drone(drone_widget) => {
                drone_widget.send_crash_command();
                let crashing_drone_id = drone_widget.get_id();
                for neighbor in neighbors {
                    match self.graph.node(neighbor).unwrap().payload() {
                        WidgetType::Drone(neighbor_widget) => {
                            let id = neighbor_widget.get_id();
                            if let Some(pos) = self.drones.iter().position(|d| d.id == id) {
                                if let Some(to_remove) = self.drones[pos]
                                    .connected_node_ids
                                    .iter()
                                    .position(|id| *id == crashing_drone_id)
                                {
                                    self.drones[pos].connected_node_ids.remove(to_remove);
                                }
                            }
                            neighbor_widget.remove_neighbor(drone_widget.get_id());
                        }
                        WidgetType::WebClient(neighbor_widget) => {
                            let id = neighbor_widget.get_id();
                            if let Some(pos) = self.clients.iter().position(|c| c.id == id) {
                                if let Some(to_remove) = self.clients[pos]
                                    .connected_drone_ids
                                    .iter()
                                    .position(|id| *id == crashing_drone_id)
                                {
                                    self.clients[pos].connected_drone_ids.remove(to_remove);
                                }
                            }
                            neighbor_widget.remove_neighbor(drone_widget.get_id());
                        }
                        WidgetType::ChatClient(neighbor_widget) => {
                            let id = neighbor_widget.get_id();
                            if let Some(pos) = self.clients.iter().position(|c| c.id == id) {
                                if let Some(to_remove) = self.clients[pos]
                                    .connected_drone_ids
                                    .iter()
                                    .position(|id| *id == crashing_drone_id)
                                {
                                    self.clients[pos].connected_drone_ids.remove(to_remove);
                                }
                            }
                            neighbor_widget.remove_neighbor(drone_widget.get_id());
                        }
                        WidgetType::Server(neighbor_widget) => {
                            let id = neighbor_widget.get_id();
                            if let Some(pos) = self.servers.iter().position(|s| s.id == id) {
                                if let Some(to_remove) = self.servers[pos]
                                    .connected_drone_ids
                                    .iter()
                                    .position(|id| *id == crashing_drone_id)
                                {
                                    self.servers[pos].connected_drone_ids.remove(to_remove);
                                }
                            }
                            neighbor_widget.remove_neighbor(drone_widget.get_id());
                        }
                    }
                }
            }
            _ => {
                unreachable!("Only drones can crash")
            }
        }
        self.graph.remove_node(crashing_drone);
        self.selected_node = None;
    }

    /// Function to spawn a new drone
    fn spawn_drone(&mut self) {
        let rand_drone_id = rand::rng().random_range(0..10);
        let drone_factory = DRONE_FACTORY[rand_drone_id];
        let new_id = 100;
        let (sender_command, receiver_command): (Sender<DroneCommand>, Receiver<DroneCommand>) =
            crossbeam_channel::unbounded();
        let (send_event, receive_event): (Sender<DroneEvent>, Receiver<DroneEvent>) =
            crossbeam_channel::unbounded();
        let (packet_send, packet_recv): (Sender<Packet>, Receiver<Packet>) =
            crossbeam_channel::unbounded();
        let nbrs = HashMap::new();
        let pdr = 0.0;
        let mut new_drone = drone_factory(
            new_id,
            send_event,
            receiver_command,
            packet_recv.clone(),
            nbrs,
            pdr,
        );

        self.drones_channels.insert(
            new_id,
            (
                sender_command.clone(),
                receive_event,
                packet_send,
                packet_recv,
            ),
        );
        self.drones.push(Drone {
            id: new_id,
            connected_node_ids: vec![],
            pdr,
        });
        let drone_idx = self.graph.add_node(WidgetType::Drone(DroneWidget::new(
            new_id,
            sender_command.clone(),
        )));
        self.graph
            .node_mut(drone_idx)
            .unwrap()
            .set_label(format!("Drone {new_id}"));
        std::thread::spawn(move || {
            new_drone.run();
        });
    }

    fn read_data(&mut self) {
        if !self.graph.selected_nodes().is_empty() {
            let idx = self.graph.selected_nodes().first().unwrap();
            self.selected_node = Some(*idx);
        }

        if !self.graph.selected_edges().is_empty() {
            let edge_idx = self.graph.selected_edges().first().unwrap();
            self.selected_edge = Some(*edge_idx);
        }
    }

    #[allow(clippy::too_many_lines)]
    fn render(&mut self, ctx: &egui::Context) {
        SidePanel::right("Panel").show(ctx, |ui| {
            if let Some(idx) = self.selected_node {
                let node = self.graph.node_mut(idx).unwrap().payload_mut().clone();
                match node {
                    WidgetType::Drone(drone_widget) => {
                        let drone_id = drone_widget.get_id();
                        ui.vertical(|ui| {
                            ui.add(drone_widget);
                            ui.separator();
                            ui.label("Crash the drone");
                            let red_btn = ui.add(
                                Button::new(RichText::new("Crash").color(Color32::BLACK))
                                    .fill(Color32::RED),
                            );
                            if red_btn.clicked() {
                                // check if the drone can crash
                                match self.can_drone_crash(drone_id) {
                                    Ok(()) => self.crash_drone(idx),
                                    Err(error) => self.drone_crash_error = error,
                                }
                            }

                            if !self.drone_crash_error.is_empty() {
                                ui.label(
                                    RichText::new(&self.drone_crash_error)
                                        .color(egui::Color32::RED),
                                );
                            }
                        })
                        .response
                    }
                    WidgetType::WebClient(web_client_widget) => ui.add(web_client_widget),
                    WidgetType::ChatClient(chat_client_widget) => ui.add(chat_client_widget),
                    WidgetType::Server(server_widget) => ui.add(server_widget),
                };
            } else {
                ui.label("No node selected");
            }

            ui.with_layout(Layout::bottom_up(egui::Align::Center), |ui| {
                ui.add_space(10.0);
                if ui.button("Add Drone").clicked() {
                    self.spawn_drone();
                }
            });
        });
        TopBottomPanel::bottom("Bottom_panel")
            .resizable(true)
            .show(ctx, |ui| {
                let text_style = TextStyle::Body;
                let row_height = ui.text_style_height(&text_style);
                ui.columns_const(|[left, right]| {
                    // Left column should containt the add sender and remove edge buttons
                    left.horizontal(|ui| {
                        if let Some(idx) = self.selected_node {
                            ui.vertical(|ui| {
                                ui.label(format!(
                                    "Selected node: {:?}",
                                    self.graph.node(idx).unwrap().payload().get_id_helper()
                                ));
                                ui.set_max_width(71.0); // Width of the add button
                                ui.text_edit_singleline(&mut self.add_neighbor_input);
                                let add_btn = ui.add(Button::new("Add sender"));
                                if add_btn.clicked() {
                                    match self
                                        .validate_add_sender(idx, &self.add_neighbor_input.clone())
                                    {
                                        Ok((source_idx, neighbor_idx)) => {
                                            let (neighbor_id, neighbor_ch) =
                                                self.get_sender_channel(neighbor_idx);
                                            let (current_node_id, current_node_ch) =
                                                self.get_sender_channel(source_idx);

                                            let current_node_widget =
                                                self.graph.node_mut(idx).unwrap().payload_mut();
                                            current_node_widget
                                                .add_neighbor_helper(neighbor_id, neighbor_ch);

                                            let neighbor_widget = self
                                                .graph
                                                .node_mut(neighbor_idx)
                                                .unwrap()
                                                .payload_mut();
                                            neighbor_widget.add_neighbor_helper(
                                                current_node_id,
                                                current_node_ch,
                                            );

                                            self.update_neighborhood(
                                                &UpdateType::Add,
                                                current_node_id,
                                                idx,
                                                neighbor_id,
                                            );
                                            self.update_neighborhood(
                                                &UpdateType::Add,
                                                neighbor_id,
                                                neighbor_idx,
                                                current_node_id,
                                            );
                                            self.graph.add_edge(idx, neighbor_idx, ());
                                        }
                                        Err(error) => self.add_neighbor_error = error,
                                    }
                                }

                                if !self.add_neighbor_error.is_empty() {
                                    ui.label(
                                        RichText::new(&self.add_neighbor_error)
                                            .color(egui::Color32::RED),
                                    );
                                }
                            });
                        }

                        ui.add_space(15.0);

                        // Remove edge area
                        if let Some(edge_idx) = self.selected_edge {
                            ui.vertical(|ui| {
                                ui.label(format!("Selected edge: {edge_idx:?}"));
                                let remove_btn = ui.add(Button::new("Remove edge"));
                                if remove_btn.clicked() {
                                    match self.validate_edge_removal(edge_idx) {
                                        Ok((node_1, node_2)) => {
                                            self.rm_neighbor_error = String::new();

                                            let node_1_idx = self.get_node_idx(node_1).unwrap();
                                            let node_1_widget = self
                                                .graph
                                                .node_mut(node_1_idx)
                                                .unwrap()
                                                .payload_mut();
                                            // Send command to source to remove neighbor
                                            node_1_widget.rm_neighbor_helper(node_2);

                                            let node_2_idx = self.get_node_idx(node_2).unwrap();
                                            let node_2_widget = self
                                                .graph
                                                .node_mut(node_2_idx)
                                                .unwrap()
                                                .payload_mut();
                                            // Send command to neighbor to remove source
                                            node_2_widget.rm_neighbor_helper(node_1);

                                            // Update state of SCL
                                            self.update_neighborhood(
                                                &UpdateType::Remove,
                                                node_1,
                                                node_1_idx,
                                                node_2,
                                            );
                                            self.update_neighborhood(
                                                &UpdateType::Remove,
                                                node_2,
                                                node_2_idx,
                                                node_1,
                                            );
                                            // Deselect the edge
                                            self.selected_edge = None;
                                            // Update graph visualization
                                            self.graph.remove_edges_between(node_1_idx, node_2_idx);
                                        }
                                        Err(error) => self.rm_neighbor_error = error,
                                    }
                                }

                                // Display the error label
                                if !self.rm_neighbor_error.is_empty() {
                                    ui.label(
                                        RichText::new(&self.rm_neighbor_error)
                                            .color(egui::Color32::RED),
                                    );
                                }
                            });
                        }
                        // ui.add(Separator::default().vertical());
                    }); // End of left column

                    // Right column should contain the event logger
                    ScrollArea::vertical().stick_to_bottom(true).show_rows(
                        right,
                        row_height,
                        self.events.len(),
                        |ui, row_range| {
                            let events = self.events.get();
                            for row in row_range {
                                ui.label(events[row].clone());
                            }
                        },
                    );
                });
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
                        .with_dragging_enabled(true)
                        .with_edge_selection_enabled(true),
                )
                .with_styles(&SettingsStyle::new().with_labels_always(true))
                .with_navigations(&SettingsNavigation::new().with_zoom_and_pan_enabled(true));
            ui.add(graph_widget);
        });
    }
}

impl eframe::App for SimulationController {
    /**
     * TODOS:
     * 1 Event logger (in progress)
     * 2 Chat client ui (in progress)
     * 4 Documentation (partially done)
     *
     * DONE (hopefully)
     * 3 Drone crash command handling
     *  - Check if a drone can crash
     */
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.handle_event();
        self.read_data();
        self.render(ctx);
    }
}
