#![warn(clippy::pedantic)]

use common::slc_commands::{ChatClientCommand, ChatClientEvent, ServerCommand, ServerEvent, WebClientCommand, WebClientEvent};
use crossbeam_channel::{Receiver, Sender};
use eframe::egui;
use egui::{Button, CentralPanel, Color32, RichText, SidePanel, TopBottomPanel};
use egui_graphs::{
    Graph, GraphView, LayoutRandom, LayoutStateRandom, SettingsInteraction, SettingsNavigation,
    SettingsStyle,
};
use petgraph::{
    graph::EdgeIndex, stable_graph::{NodeIndex, StableUnGraph}, Undirected
};
use std::{collections::{HashMap, HashSet, VecDeque}, fs::File, io::Write, path::Path};
use wg_2024::{
    config::{Client, Drone, Server},
    controller::{DroneCommand, DroneEvent},
    network::NodeId,
    packet::Packet,
};
mod widgets;
use widgets::{chat_client_widget::ChatClientWidget, drone_widget::DroneWidget, server_widget::ServerWidget, web_client_widget::WebClientWidget, WidgetType};

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
type CCChannels<'a> = &'a HashMap<NodeId, (Sender<ChatClientCommand>, Receiver<ChatClientEvent>, Sender<Packet>, Receiver<Packet>)>;
type SChannels<'a> = &'a HashMap<NodeId, (Sender<ServerCommand>, Receiver<ServerEvent>, Sender<Packet>, Receiver<Packet>)>;
fn generate_graph(dh: DChannels, wch: WCChannels, cch: CCChannels, sh: SChannels, drones: &Vec<Drone>, clients: &Vec<Client>, servers: &Vec<Server>) -> Graph<WidgetType, (), Undirected> {
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
        let idx = g.add_node(WidgetType::WebClient(WebClientWidget::new(*id, channels.0.clone())));
        h.insert(*id, idx);
    }
    // Create chat client widgets
    for (id, channels) in cch {
        let idx = g.add_node(WidgetType::ChatClient(ChatClientWidget::new(*id, channels.0.clone())));
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
    selected_edge: Option<EdgeIndex>,
    add_neighbor_input: String,
    add_neighbor_error: String,
    rm_neighbor_input: String,
    rm_neighbor_error: String,
    drone_crash_error: String,
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
        let graph = generate_graph(&drones_channels, &web_clients_channels, &chat_clients_channels, &servers_channels, &drones, &clients, &servers);
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
            selected_edge: Option::default(),
            add_neighbor_input: String::default(),
            add_neighbor_error: String::default(),
            rm_neighbor_input: String::default(),
            rm_neighbor_error: String::default(),
            drone_crash_error: String::default()
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

        // Here I should add the chat clients events

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
                println!("Received shortcut: {:?}", packet);
                let destination_id = packet.routing_header.destination();
                match destination_id {
                    Some(id) => self.handle_shortcut(id, packet),
                    None => unreachable!("Is it possible????"),
                }
            },
            WebClientEvent::ListOfFiles(files, server_id) => {
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
    fn validate_parse_neighbor_id(&mut self, input_neighbor_id: &String) -> Result<NodeIndex, String> {
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
                    return Ok(neighbor_idx)
                },

                // Web Client - check if current client has reached it max number of connections (2)
                (WidgetType::WebClient(web_client_widget), WidgetType::Drone(_)) => {
                    let client_id = web_client_widget.get_id();
                    if let Some(pos) = self.clients.iter().position(|cl| cl.id == client_id) {
                        // Check if the current client has reached its max number of connections
                        if self.clients[pos].connected_drone_ids.len() == 2 {
                            return Err(format!("Client {}, reached its max connections", client_id));
                        } else {
                            return Ok(neighbor_idx);
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
                            return Ok(neighbor_idx);
                        }
                    } else { return Err("Client not found".to_string()) }
                },
                // Here I include all patterns like ChatClient/ChatClient, ChatClient/WebClient, ChatClient/Server.
                (WidgetType::ChatClient(_), _) => return Err("Client cannot be connected directly to other client nor server".to_string()),
                
                // Servers - can be connected to any number of drones (but min. 2)
                (WidgetType::Server(_), WidgetType::Drone(_)) => return Ok(neighbor_idx),
                (WidgetType::Server(_), _) => return Err("Server cannot be connected directly to other client nor server".to_string()),
            }
        } else {
            return Err("No selected node".to_string());
        }
    }

    fn get_sender_channel(&self, idx: NodeIndex) -> (NodeId, Sender<Packet>) {
        match self.graph.node(idx).unwrap().payload() {
            WidgetType::Drone(dw) => (dw.get_id(), self.drones_channels[&dw.get_id()].2.clone()),
            WidgetType::WebClient(wcw) => (wcw.get_id(), self.web_clients_channels[&wcw.get_id()].2.clone()),
            WidgetType::ChatClient(ccw) => (ccw.get_id(), self.chat_clients_channels[&ccw.get_id()].2.clone()),
            WidgetType::Server(sw) => (sw.get_id(), self.servers_channels[&sw.get_id()].2.clone()),
        }
    }

    /**
     * Before removing an edge, I should check that without it, every client can still reach every server.
     */
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
                        if let WidgetType::Server(server_widget) = copy_graph.node(neighbor).unwrap().payload() {
                            servers_visited.insert(server_widget.get_id());
                        } else if let WidgetType::ChatClient(_) | WidgetType::WebClient(_) = copy_graph.node(neighbor).unwrap().payload() {
                            continue;
                        } else {
                            stack.push_front(neighbor);
                        }
                    }
                }
            }

            // Check if the client can reach every server
            if servers_visited.len() != self.servers.len() {
                return Err(format!("By removing edge {}, client {} wouldn't reach every server", edge_to_remove.index(), client.id));
            }
        }
        Ok(())
    }

    /**
     * This function checks whether the graph would become disconnected
     * by removing the edge between source_idx and neighbor_idx
     */
    fn is_graph_disconnected(&self, source_idx: NodeIndex, neighbor_idx: NodeIndex) -> bool {
        let mut copy_graph = self.graph.clone();
        copy_graph.remove_edges_between(source_idx, neighbor_idx);
        let cc = petgraph::algo::tarjan_scc(&copy_graph.g);
        cc.len() > 1 // Means that there are more than 1 CC, so the graph is disconnected
    }

    fn can_remove_sender(&self, node_idx: NodeIndex) -> Result<u8, String> {
        match self.graph.node(node_idx).unwrap().payload() {
            // For drones I should check if they have at least 1 connection, otherwise the graph becomes disconnected
            WidgetType::Drone(drone_widget) => {
                let drone_id = drone_widget.get_id();
                if let Some(pos) = self.drones.iter().position(|d| d.id == drone_id) {
                    if self.drones.get(pos).unwrap().connected_node_ids.len() == 1 {
                        return Err(format!("Cant remove last connection of drone {}!!!", drone_id));
                    } else {
                        return Ok(drone_id);
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
                        return Ok(client_id);
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
                        return Ok(client_id);
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
                        return Ok(server_id);
                    }
                } else {
                    return Err("Server not found".to_string());
                }
            },
        }
    }

    // idea:
    // prendo i due endpoints (NodeIndex, NodeIndex)
    // controllo se togliendo arco ottengo grafo disconnesso:
    //  - si -> torno errore
    //  - no -> procedo con il controllo
    // methodo che riceve NodeIndex e controlla se per quel nodo si può togliere una connessione
    // uso il metodo per controllare se entrambi i nodi possono rimuovere una connessione
    fn validate_edge_removal(&mut self, edge: EdgeIndex) -> Result<(u8, u8), String> {
        
        if let Err(e) = self.check_connectivity(edge) {
            return Err(e);
        }

        // Take the 2 endpoints of the edge to be removed
        let (node_1, node_2) = self.graph.edge_endpoints(edge).unwrap();
        if self.is_graph_disconnected(node_1, node_2) {
            return Err("Can't remove the edge, otherwise the graph would become disconnected".to_string());
        }

        match (self.can_remove_sender(node_1), self.can_remove_sender(node_2)) {
            (Ok(id_1), Ok(id_2)) => Ok((id_1, id_2)),
            (Ok(_), Err(e)) => Err(e),
            (Err(e), Ok(_)) => Err(e),
            (Err(_), Err(_)) => Err("Either nodes can't remove each other".to_string()),
        }
    }
    /**
     * Method to check whether a node can remove a sender or not
     * Base checks that should be verified before removing, for each type of widget:
     * - Client -> must remain connected to at least 1 drone
     * - Server -> must remain connected to at least 2 drones
     * However these checks does not take into account the possibility to leave the graph disconnected.
     * So a check to see if the removal of an edge would make the graph disconnected, should be introduced.
     */
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
            // Here we check if the graph would become disconnected
            if self.is_graph_disconnected(current_selected_node, neighbor_idx) {
                return Err("Can't remove the edge, otherwise the graph would become disconnected".to_string());
            }
            match self.graph.node(current_selected_node).unwrap().payload() {
                // For drones I should check if they have at least 1 connection, otherwise the graph becomes disconnected
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

    fn can_drone_crash(&self, drone_id: NodeId) -> Result<(), String> {
        let drone_idx = self.get_node_idx(drone_id).unwrap();

        // Check if the neighbors of the drone can remove it
        let neighbors = self.graph.g.neighbors(drone_idx).collect::<Vec<NodeIndex>>();
        for neighbor in neighbors {
            match self.graph.node(neighbor).unwrap().payload() {
                WidgetType::Drone(drone_widget) => {
                    let id = drone_widget.get_id();
                    if let Some(pos) = self.drones.iter().position(|d| d.id == id) {
                        if self.drones[pos].connected_node_ids.len() == 1 {
                            return Err(format!("Drone {} must have at least 1 connection", id));
                        }
                    }
                },
                WidgetType::WebClient(web_client_widget) => {
                    let id = web_client_widget.get_id();
                    if let Some(pos) = self.clients.iter().position(|wc| wc.id == id) {
                        if self.clients[pos].connected_drone_ids.len() == 1 {
                            return Err(format!("Client {} must have at least 1 connection", id));
                        }
                    }
                },
                WidgetType::ChatClient(chat_client_widget) => {
                    let id = chat_client_widget.get_id();
                    if let Some(pos) = self.clients.iter().position(|cc| cc.id == id) {
                        if self.clients[pos].connected_drone_ids.len() == 1 {
                            return Err(format!("Client {} must have at least 1 connection", id));
                        }
                    }
                },
                WidgetType::Server(server_widget) => {
                    let id = server_widget.get_id();
                    if let Some(pos) = self.servers.iter().position(|s| s.id == id) {
                        if self.servers[pos].connected_drone_ids.len() == 2 {
                            return Err(format!("Server {} must have at least 2 connections", id));
                        }
                    }
                },
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
                        if let WidgetType::Server(server_widget) = copy_graph.node(neighbor).unwrap().payload() {
                            servers_visited.insert(server_widget.get_id());
                        } else if let WidgetType::ChatClient(_) | WidgetType::WebClient(_) = copy_graph.node(neighbor).unwrap().payload() {
                            continue;
                        } else {
                            stack.push_front(neighbor);
                        }
                    }
                }
            }

            // Check if the client can reach every server
            if servers_visited.len() != self.servers.len() {
                return Err(format!("By removing drone {}, client {} wouldn't reach every server", drone_idx.index(), client.id));
            }
        }

        // check if graph is still connected
        let cc = petgraph::algo::tarjan_scc(&copy_graph.g);
        if cc.len() > 1 {
            return Err(format!("By removing drone {}, the graph would become disconnected", drone_idx.index()));
        }

        Ok(())
    }

    fn crash_drone(&mut self, crashing_drone: NodeIndex) {
        let drone = self.graph.node(crashing_drone).unwrap().payload();
        let neighbors = self.graph.g.neighbors(crashing_drone).collect::<Vec<NodeIndex>>();
        match drone {
            WidgetType::Drone(drone_widget) => {
                drone_widget.send_crash_command();
                let crashing_drone_id = drone_widget.get_id();
                for neighbor in neighbors {
                    match self.graph.node(neighbor).unwrap().payload() {
                        WidgetType::Drone(neighbor_widget) => {
                            let id = neighbor_widget.get_id();
                            if let Some(pos) = self.drones.iter().position(|d| d.id == id) {
                                if let Some(to_remove) = self.drones[pos].connected_node_ids.iter().position(|id| *id == crashing_drone_id) {
                                    self.drones[pos].connected_node_ids.remove(to_remove);
                                }
                            }
                            neighbor_widget.remove_neighbor(drone_widget.get_id());
                        },
                        WidgetType::WebClient(neighbor_widget) => {
                            let id = neighbor_widget.get_id();
                            if let Some(pos) = self.clients.iter().position(|c| c.id == id) {
                                if let Some(to_remove) = self.clients[pos].connected_drone_ids.iter().position(|id| *id == crashing_drone_id) {
                                    self.clients[pos].connected_drone_ids.remove(to_remove);
                                }
                            }
                            neighbor_widget.remove_neighbor(drone_widget.get_id());
                        },
                        WidgetType::ChatClient(neighbor_widget) => {
                            let id = neighbor_widget.get_id();
                            if let Some(pos) = self.clients.iter().position(|c| c.id == id) {
                                if let Some(to_remove) = self.clients[pos].connected_drone_ids.iter().position(|id| *id == crashing_drone_id) {
                                    self.clients[pos].connected_drone_ids.remove(to_remove);
                                }
                            }
                            neighbor_widget.remove_neighbor(drone_widget.get_id());
                        },
                        WidgetType::Server(neighbor_widget) => {
                            let id = neighbor_widget.get_id();
                            if let Some(pos) = self.servers.iter().position(|s| s.id == id) {
                                if let Some(to_remove) = self.servers[pos].connected_drone_ids.iter().position(|id| *id == crashing_drone_id) {
                                    self.servers[pos].connected_drone_ids.remove(to_remove);
                                }
                            }
                            neighbor_widget.remove_neighbor(drone_widget.get_id());
                        },
                    }
                }
            },
            _ => {unreachable!("Only drones can crash")}
        }
        self.graph.remove_node(crashing_drone);
        self.selected_node = None;
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

    fn render(&mut self, ctx: &egui::Context) {
        SidePanel::right("Panel").show(ctx, |ui| {
            ui.label("Selected node:");
            if let Some(idx) = self.selected_node {
                let node = self.graph.node_mut(idx).unwrap().payload_mut().clone();
                // let mut node = self.graph.node(idx).unwrap().payload_mut();
                match node {
                    WidgetType::Drone(drone_widget) => {
                        let drone_id = drone_widget.get_id();
                        ui.vertical(|ui| {
                            ui.add(drone_widget);
                            // ui.separator();
                            // ui.label("Crash the drone");
                            // let red_btn =
                            //     ui.add(Button::new(RichText::new("Crash").color(Color32::BLACK)).fill(Color32::RED));
                            // if red_btn.clicked() {
                            //     // check if the drone can crash
                            //     match self.can_drone_crash(drone_id) {
                            //         Ok(_) => self.crash_drone(idx),
                            //         Err(error) => self.drone_crash_error = error,
                            //     }
                            // }

                            // if !self.drone_crash_error.is_empty() {
                            //     ui.label(RichText::new(&self.drone_crash_error).color(egui::Color32::RED));
                            // }
                        }).response
                    },
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
                    .with_dragging_enabled(true)
                    .with_edge_selection_enabled(true),
                )
                .with_styles(&SettingsStyle::new().with_labels_always(true))
                .with_navigations(&SettingsNavigation::new().with_zoom_and_pan_enabled(true));
            ui.add(graph_widget);       
        });
        TopBottomPanel::bottom("Bottom_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                // Add sender area
                if let Some(idx) = self.selected_node {
                    ui.vertical(|ui| {
                        ui.label(format!("Selected node: {:?}", self.graph.node(idx).unwrap().payload().get_id_helper()));
                        ui.set_max_width(71.0); // Width of the add button
                        ui.text_edit_singleline(&mut self.add_neighbor_input);
                        let add_btn = ui.add(Button::new("Add sender"));

                        if add_btn.clicked() {
                            match self.validate_parse_neighbor_id(&self.add_neighbor_input.clone()) {
                                Ok(neighbor_idx) => {
                                    let (neighbor_id, neighbor_ch) = self.get_sender_channel(neighbor_idx);
                                    let (current_node_id, current_node_ch) = self.get_sender_channel(idx);

                                    let current_node_widget = self.graph.node_mut(idx).unwrap().payload_mut();
                                    current_node_widget.add_neighbor_helper(neighbor_id, neighbor_ch);

                                    let neighbor_widget = self.graph.node_mut(neighbor_idx).unwrap().payload_mut();
                                    neighbor_widget.add_neighbor_helper(current_node_id, current_node_ch);

                                    self.update_neighborhood(UpdateType::Add, current_node_id, idx, neighbor_id);
                                    self.update_neighborhood(UpdateType::Add, neighbor_id, neighbor_idx, current_node_id);
                                    self.graph.add_edge(idx, neighbor_idx, ());
                                },
                                Err(error) => self.add_neighbor_error = error,
                            }
                        }

                        if !self.add_neighbor_error.is_empty() {
                            ui.label(RichText::new(&self.add_neighbor_error).color(egui::Color32::RED));
                        }
                    });
                }

                ui.add_space(15.0);

                // Remove edge area
                if let Some(edge_idx) = self.selected_edge {
                    ui.vertical(|ui| {
                        ui.label(format!("Selected edge: {:?}", edge_idx));
                        let remove_btn = ui.add(Button::new("Remove edge"));
        
                        if remove_btn.clicked() {
                            match self.validate_edge_removal(edge_idx) {
                                Ok((node_1, node_2)) => {
                                    self.rm_neighbor_error = String::new();
        
                                    let node_1_idx = self.get_node_idx(node_1).unwrap();
                                    let node_1_widget = self.graph.node_mut(node_1_idx).unwrap().payload_mut();
                                    // Send command to source to remove neighbor
                                    node_1_widget.rm_neighbor_helper(node_2);
        
        
                                    let node_2_idx = self.get_node_idx(node_2).unwrap();
                                    let node_2_widget = self.graph.node_mut(node_2_idx).unwrap().payload_mut();
                                    // Send command to neighbor to remove source
                                    node_2_widget.rm_neighbor_helper(node_1);
                                    
                                    // Update state of SCL
                                    self.update_neighborhood(UpdateType::Remove, node_1, node_1_idx, node_2);
                                    self.update_neighborhood(UpdateType::Remove, node_2, node_2_idx, node_1);
                                    // Update graph visualization
                                    self.graph.remove_edges_between(node_1_idx, node_2_idx);
                                },
                                Err(error) => self.rm_neighbor_error = error,
                            }
                        }
        
                        // Display the error label
                        if !self.rm_neighbor_error.is_empty() {
                            ui.label(RichText::new(&self.rm_neighbor_error).color(egui::Color32::RED));
                        }
                    });
                }
            });
        });
    }

}

impl eframe::App for SimulationController {
    /**
     * TODOS:
     * 1 Event logger
     * 2 Chat client ui
     * 3 Drone crash command handling
     *  - Check if a drone can crash
     * 4 Documentation
     */
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.handle_event();
        self.read_data();
        self.render(ctx);
    }
}

