use crossbeam_channel::{Receiver, Sender};
use wg_2024::{controller::{DroneCommand, DroneEvent}, network::NodeId};
use common::slc_commands::{ClientCommand, ClientEvent, ServerCommand, ServerEvent};



#[derive(Clone)]
pub struct Widget {
    id: NodeId,
    node_type: NodeType,
}

#[derive(Clone)]
pub enum NodeType {
    Drone { command_ch: Sender<DroneCommand>, event_ch: Receiver<DroneEvent> },
    Client { command_ch: Sender<ClientCommand>, event_ch: Receiver<ClientEvent> },
    Server { command_ch: Sender<ServerCommand>, event_ch: Receiver<ServerEvent> },
}
