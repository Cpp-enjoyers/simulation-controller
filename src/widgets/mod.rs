use server_widget::ServerWidget;
use drone_widget::DroneWidget;
use web_client_widget::WebClientWidget;
use chat_client_widget::ChatClientWidget;

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