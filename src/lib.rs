use wg_2024::network::NodeId;

#[derive(Debug)]
pub struct SimulationController {
    id: NodeId,
}

impl SimulationController {
    fn new(id: NodeId) -> Self {
        SimulationController { id }
    }

    fn run(&mut self) {
        todo!()
    }
}
