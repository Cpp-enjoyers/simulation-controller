use server_widget::ServerWidget;
use drone_widget::DroneWidget;
use web_client_widget::ClientWidget;

pub mod drone_widget;
pub mod web_client_widget;
pub mod server_widget;

#[derive(Clone, Debug)]
pub enum WidgetType {
    Drone(DroneWidget),
    Client(ClientWidget),
    Server(ServerWidget),
}