use std::collections::HashMap;

use common::slc_commands::{ClientCommand, ClientEvent, ServerCommand, ServerEvent};
use crossbeam_channel::{Receiver, Sender};
use wg_2024::{
    controller::{DroneCommand, DroneEvent},
    network::NodeId,
};

#[derive(Debug)]
pub struct SimulationController {
    id: NodeId,
    // drones: HashMap<NodeId, Sender<DroneCommand>>,
    // drones_rcv: Receiver<DroneEvent>,
    // clients: HashMap<NodeId, Sender<ClientCommand>>,
    // clients_rcv: Receiver<ClientEvent>,
    // servers: HashMap<NodeId, Sender<ServerCommand>>,
    // servers_rcv: Receiver<ServerEvent>,
    drones_channels: HashMap<NodeId, (Sender<DroneCommand>, Receiver<DroneEvent>)>,
    clients_channels: HashMap<NodeId, (Sender<ClientCommand>, Receiver<ClientEvent>)>,
    servers_channels: HashMap<NodeId, (Sender<ServerCommand>, Receiver<ServerEvent>)>,
}

impl SimulationController {
    fn new(
        id: NodeId,
        drones_channels: HashMap<NodeId, (Sender<DroneCommand>, Receiver<DroneEvent>)>,
        clients_channels: HashMap<NodeId, (Sender<ClientCommand>, Receiver<ClientEvent>)>,
        servers_channels: HashMap<NodeId, (Sender<ServerCommand>, Receiver<ServerEvent>)>,
    ) -> Self {
        SimulationController {
            id,
            // drones,
            // drones_rcv,
            // clients,
            // clients_rcv,
            // servers,
            // servers_rcv,
            drones_channels,
            clients_channels,
            servers_channels,
        }
    }

    fn run(&mut self) {
        todo!()
    }
}
