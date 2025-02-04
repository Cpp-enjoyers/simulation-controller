use crossbeam_channel::Sender;
use server_widget::ServerWidget;
use drone_widget::DroneWidget;
use web_client_widget::WebClientWidget;
use chat_client_widget::ChatClientWidget;
use wg_2024::packet::Packet;

pub mod drone_widget;
pub mod web_client_widget;
pub mod chat_client_widget;
pub mod server_widget;

#[derive(Clone, Debug)]
pub enum WidgetType {
    Drone(DroneWidget),
    WebClient(WebClientWidget),
    ChatClient(ChatClientWidget),
    Server(ServerWidget),
}

impl WidgetType {
    pub fn add_neighbor_helper(&mut self, nid: u8, nch: Sender<Packet>) {
        match self {
            WidgetType::Drone(drone_widget) => drone_widget.add_neighbor(nid, nch),
            WidgetType::WebClient(web_client_widget) => web_client_widget.add_neighbor(nid, nch),
            WidgetType::ChatClient(chat_client_widget) => chat_client_widget.add_neighbor(nid, nch),
            WidgetType::Server(server_widget) => server_widget.add_neighbor(nid, nch),
        }
    }
}