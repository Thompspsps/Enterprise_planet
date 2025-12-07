#[cfg(test)]
mod tests {
    use common_game::{
        components::{
            energy_cell::EnergyCell,
            planet::PlanetState,
            rocket::Rocket,
        },
        protocols::messages,
    };
    use crate::ai::EnterpriseAi;

    use super::*;
    use std::sync::mpsc::{Receiver, Sender, channel};
    use std::time::SystemTime;

    #[test]
    fn is_one_equal_to_one() {
        assert_eq!(1, 1)
    }

    #[test]
    fn test_ai_initial_state_should_not_be_running() {
        let ai = EnterpriseAi::new();
        assert!(!ai.is_running());
    }

    fn create_dummy_state() -> PlanetState {
        PlanetState {
            id: 67,
            energy_cells: vec![EnergyCell::new()],
            rocket: None,
            can_have_rocket: true,
        }
    }

    #[test]
    fn ai_should_start_n_stop() {
        let mut ai = EnterpriseAi::new();
        let dummy_state = create_dummy_state();

        ai.start(&dummy_state);
        assert!(ai.is_running());

        ai.stop(&dummy_state);
        assert!(!ai.is_running());
    }

    #[test]
    fn test_planet_creation() {
        // let (tx_orchestrator,rx_orchestrator)=channel();
        // let (tx_explorer,rx_explorer)=channel();

        // let planet=create_planet(rx_orchestrator, tx_orchestrator, rx_explorer, tx_explorer);

        // assert_eq!(planet.id(),67);
        // assert_eq!(planet.planet_type(),PlanetType::C);
    }
}