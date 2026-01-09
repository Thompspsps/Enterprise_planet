use crate::create_planet;
use crate::EnterpriseAi;

#[cfg(test)]
mod tests {
    use super::*;
    use common_game::components::asteroid::Asteroid;
    use common_game::components::planet::{
        DummyPlanetState, Planet, PlanetAI, PlanetState, PlanetType,
    };
    use common_game::components::resource::*;
    use common_game::components::resource::{Combinator, Generator};
    use common_game::components::rocket::Rocket;
    use common_game::components::sunray::Sunray;
    use common_game::protocols::orchestrator_planet::*;
    use common_game::protocols::planet_explorer::*;
    use crossbeam_channel::{Receiver, Sender, unbounded};
    use std::thread;
    use std::time::Duration;

    /// Test that our planet can be created and follows Type C constraints
    #[test]
    fn test_planet_creation_and_type_constraints() {
        let id = 123;
        
        // Setup
        let (_orch_tx, planet_rx) = unbounded();
        let (planet_tx, _orch_rx) = unbounded();
        let (_expl_tx, expl_rx) = unbounded();
        
        // Create planet
        let planet = create_planet(id, planet_rx, planet_tx, expl_rx);
        
        // Assertions
        assert_eq!(planet.id(), id);
        assert!(matches!(planet.planet_type(), PlanetType::C));
        
        // Type C has 1 energy cell capacity
        let state = planet.state();
        assert_eq!(state.cells_count(), 1);
        
        // Type C can have rockets
        assert!(state.can_have_rocket());
        assert!(!state.has_rocket()); // The initial planet state shouldn't have a built rocket
        
        // Generator should have Carbon recipe
        assert!(planet.generator().contains(BasicResourceType::Carbon));
        assert_eq!(planet.generator().all_available_recipes().len(), 1);
        
        // Combinator should have all 6 recipes
        assert_eq!(planet.combinator().all_available_recipes().len(), 6);

    }
    
    /// Test basic planet lifecycle (start, stop, kill)
    #[test]
    fn test_basic_lifecycle() {
        let id = 456;
        
        let (orch_tx, planet_rx) = unbounded();
        let (planet_tx, orch_rx) = unbounded();
        let (_expl_tx, expl_rx) = unbounded();
        
        let mut planet = create_planet(id, planet_rx, planet_tx, expl_rx);
        
        // Run in background thread
        let handle = thread::spawn(move || {
            let _ = planet.run();
        });
        
        // Start
        orch_tx.send(OrchestratorToPlanet::StartPlanetAI).unwrap();
        match orch_rx.recv_timeout(Duration::from_millis(50)) {
            Ok(PlanetToOrchestrator::StartPlanetAIResult { planet_id: n }) => {
                assert_eq!(n, id)
            }
            _ => assert!(false),
        }
        
        // Stop
        orch_tx.send(OrchestratorToPlanet::StopPlanetAI).unwrap();
        match orch_rx.recv_timeout(Duration::from_millis(50)) {
            Ok(PlanetToOrchestrator::StopPlanetAIResult { planet_id: n }) => {
                assert_eq!(n, id)
            }
            _ => assert!(false),
        }
        
        // Kill
        orch_tx.send(OrchestratorToPlanet::KillPlanet).unwrap();
        match orch_rx.recv_timeout(Duration::from_millis(50)) {
            Ok(PlanetToOrchestrator::KillPlanetResult { planet_id: n }) => {
                assert_eq!(n, id)
            }
            _ => assert!(false),
        }
        
        handle.join().unwrap();
    }

    //Test the initial states of the AI
    #[test]
    fn test_ai_initial_state() {
        let ai = EnterpriseAi::new(67);

        assert_eq!(ai.planet_id, 67);
        assert!(!ai.is_running());
        assert_eq!(ai.num_explorers, 0);
    }

    //Test the behaviour of the planet when receiving sunrays and no explorer is present
    #[test]
    fn test_sunray_no_explorer() {
        let (tx_orch_in, rx_orch_in) = unbounded::<OrchestratorToPlanet>();
        let (tx_orch_out, rx_orch_out) = unbounded::<PlanetToOrchestrator>();
        let (_tx_expl_in, rx_expl_in) = unbounded::<ExplorerToPlanet>();

        let mut dummy_planet = create_planet(67, rx_orch_in, tx_orch_out, rx_expl_in); // Creating the planet

        let _handle = thread::spawn(move || dummy_planet.run());

        //Start
        tx_orch_in.send(OrchestratorToPlanet::StartPlanetAI).unwrap();
        match rx_orch_out.recv_timeout(Duration::from_millis(50)) {
            Ok(PlanetToOrchestrator::StartPlanetAIResult { planet_id: 67 }) => {
                assert!(true)
            }
            _ => assert!(false),
        }

        //Send sunray
        tx_orch_in.send(OrchestratorToPlanet::Sunray(Sunray::default())).unwrap();
        match rx_orch_out.recv_timeout(Duration::from_millis(50)) {
            Ok(PlanetToOrchestrator::SunrayAck { planet_id: 67 }) => {
                assert!(true)
            }
            _ => assert!(false),
        }

        //Check internal state
        tx_orch_in.send(OrchestratorToPlanet::InternalStateRequest).unwrap();
        match rx_orch_out.recv_timeout(Duration::from_millis(50)) {
            Ok(PlanetToOrchestrator::InternalStateResponse { planet_id:67, planet_state:dummy_state }) => {
                assert!(dummy_state.has_rocket); //The planet should have a rocket because it has no explorer
                assert_eq!(dummy_state.charged_cells_count, 0); //The planet should have 0 charged cells because the energy cell was used to build the rocket
            }
            _ => assert!(false),
        }

        //Send 2nd sunray
        tx_orch_in.send(OrchestratorToPlanet::Sunray(Sunray::default())).unwrap();
        match rx_orch_out.recv_timeout(Duration::from_millis(50)) {
            Ok(PlanetToOrchestrator::SunrayAck { planet_id: 67 }) => {
                assert!(true)
            }
            _ => assert!(false),
        }

        //Check internal state (again)
        tx_orch_in.send(OrchestratorToPlanet::InternalStateRequest).unwrap();
        match rx_orch_out.recv_timeout(Duration::from_millis(50)) {
            Ok(PlanetToOrchestrator::InternalStateResponse { planet_id:67, planet_state:dummy_state }) => {
                assert!(dummy_state.has_rocket); //The planet should still have a rocket
                assert_eq!(dummy_state.charged_cells_count, 1); //The planet should have 1 charged cell
            }
            _ => assert!(false),
        }

        // Stop
        tx_orch_in.send(OrchestratorToPlanet::StopPlanetAI).unwrap();
        match rx_orch_out.recv_timeout(Duration::from_millis(50)) {
            Ok(PlanetToOrchestrator::StopPlanetAIResult { planet_id: 67 }) => {
                assert!(true)
            }
            _ => assert!(false),
        }
        
        // Kill
        tx_orch_in.send(OrchestratorToPlanet::KillPlanet).unwrap();
        match rx_orch_out.recv_timeout(Duration::from_millis(50)) {
            Ok(PlanetToOrchestrator::KillPlanetResult { planet_id: 67 }) => {
                assert!(true)
            }
            _ => assert!(false),
        }
    }

    //Test the behaviour when there is a rocket and an asteroid is sent
    #[test]
    fn test_asteroid_with_rocket() {
        let (tx_orch_in, rx_orch_in) = unbounded::<OrchestratorToPlanet>();
        let (tx_orch_out, rx_orch_out) = unbounded::<PlanetToOrchestrator>();
        let (_tx_expl_in, rx_expl_in) = unbounded::<ExplorerToPlanet>();

        let mut dummy_planet = create_planet(67, rx_orch_in, tx_orch_out, rx_expl_in); // Creating the planet

        let _handle = thread::spawn(move || dummy_planet.run());

        //Start
        tx_orch_in.send(OrchestratorToPlanet::StartPlanetAI).unwrap();
        match rx_orch_out.recv_timeout(Duration::from_millis(50)) {
            Ok(PlanetToOrchestrator::StartPlanetAIResult { planet_id: 67 }) => {
                assert!(true)
            }
            _ => assert!(false),
        }

        //Send sunray
        tx_orch_in.send(OrchestratorToPlanet::Sunray(Sunray::default())).unwrap();
        match rx_orch_out.recv_timeout(Duration::from_millis(50)) {
            Ok(PlanetToOrchestrator::SunrayAck { planet_id: 67 }) => {
                assert!(true)
            }
            _ => assert!(false),
        }

        //Send asteroid
        tx_orch_in.send(OrchestratorToPlanet::Asteroid(Asteroid::default())).unwrap();
        match rx_orch_out.recv_timeout(Duration::from_millis(50)) {
            Ok(PlanetToOrchestrator::AsteroidAck { planet_id:67, rocket:r }) => {
                assert!(r.is_some()); //There should be a rocket to destroy the asteroid
            }
            _ => assert!(false),
        }

        //Check internal state
        tx_orch_in.send(OrchestratorToPlanet::InternalStateRequest).unwrap();
        match rx_orch_out.recv_timeout(Duration::from_millis(50)) {
            Ok(PlanetToOrchestrator::InternalStateResponse { planet_id:67, planet_state:dummy_state }) => {
                assert!(!dummy_state.has_rocket); //The planet should not have a rocket because it was used
                assert_eq!(dummy_state.charged_cells_count, 0); //The planet should have 0 charged cells
            }
            _ => assert!(false),
        }

        // Stop
        tx_orch_in.send(OrchestratorToPlanet::StopPlanetAI).unwrap();
        match rx_orch_out.recv_timeout(Duration::from_millis(50)) {
            Ok(PlanetToOrchestrator::StopPlanetAIResult { planet_id: 67 }) => {
                assert!(true)
            }
            _ => assert!(false),
        }
        
        // Kill
        tx_orch_in.send(OrchestratorToPlanet::KillPlanet).unwrap();
        match rx_orch_out.recv_timeout(Duration::from_millis(50)) {
            Ok(PlanetToOrchestrator::KillPlanetResult { planet_id: 67 }) => {
                assert!(true)
            }
            _ => assert!(false),
        }
    }

    //Test the behaviour when there is no rocket and no energy cell to protect the planet
    #[test]
    fn test_asteroid_no_defense() {
        let (tx_orch_in, rx_orch_in) = unbounded::<OrchestratorToPlanet>();
        let (tx_orch_out, rx_orch_out) = unbounded::<PlanetToOrchestrator>();
        let (_tx_expl_in, rx_expl_in) = unbounded::<ExplorerToPlanet>();

        let mut dummy_planet = create_planet(67, rx_orch_in, tx_orch_out, rx_expl_in); // Creating the planet

        let _handle = thread::spawn(move || dummy_planet.run());

        //Start
        tx_orch_in.send(OrchestratorToPlanet::StartPlanetAI).unwrap();
        match rx_orch_out.recv_timeout(Duration::from_millis(50)) {
            Ok(PlanetToOrchestrator::StartPlanetAIResult { planet_id: 67 }) => {
                assert!(true)
            }
            _ => assert!(false),
        }

        //Check internal state -> No energy cell, no rocket
        tx_orch_in.send(OrchestratorToPlanet::InternalStateRequest).unwrap();
        match rx_orch_out.recv_timeout(Duration::from_millis(50)) {
            Ok(PlanetToOrchestrator::InternalStateResponse { planet_id:67, planet_state:dummy_state }) => {
                assert!(!dummy_state.has_rocket); //No rocket
                assert_eq!(dummy_state.charged_cells_count, 0); //No energy cell
            }
            _ => assert!(false),
        }

        //Send asteroid
        tx_orch_in.send(OrchestratorToPlanet::Asteroid(Asteroid::default())).unwrap();
        match rx_orch_out.recv_timeout(Duration::from_millis(50)) {
            Ok(PlanetToOrchestrator::AsteroidAck { planet_id:67, rocket:r }) => {
                assert!(r.is_none()); //There should be no rocket to destroy the asteroid
            }
            _ => assert!(false),
        }

        //Check internal state -> Should be the same
        tx_orch_in.send(OrchestratorToPlanet::InternalStateRequest).unwrap();
        match rx_orch_out.recv_timeout(Duration::from_millis(50)) {
            Ok(PlanetToOrchestrator::InternalStateResponse { planet_id:67, planet_state:dummy_state }) => {
                assert!(!dummy_state.has_rocket); //No rocket
                assert_eq!(dummy_state.charged_cells_count, 0); //No energy cell
            }
            _ => assert!(false),
        }

        // Stop
        tx_orch_in.send(OrchestratorToPlanet::StopPlanetAI).unwrap();
        match rx_orch_out.recv_timeout(Duration::from_millis(50)) {
            Ok(PlanetToOrchestrator::StopPlanetAIResult { planet_id: 67 }) => {
                assert!(true)
            }
            _ => assert!(false),
        }
        
        // Kill
        tx_orch_in.send(OrchestratorToPlanet::KillPlanet).unwrap();
        match rx_orch_out.recv_timeout(Duration::from_millis(50)) {
            Ok(PlanetToOrchestrator::KillPlanetResult { planet_id: 67 }) => {
                assert!(true)
            }
            _ => assert!(false),
        }
    }

    //Test explorer coming to and leaving the planet
    #[test]
    fn test_explorer_in_out(){
        let (tx_orch_in, rx_orch_in) = unbounded::<OrchestratorToPlanet>();
        let (tx_orch_out, rx_orch_out) = unbounded::<PlanetToOrchestrator>();
        let (_tx_expl_in, rx_expl_in) = unbounded::<ExplorerToPlanet>();
        let (tx_expl_out, _rx_expl_out) = unbounded::<PlanetToExplorer>();

        let mut dummy_planet = create_planet(67, rx_orch_in, tx_orch_out, rx_expl_in); // Creating the planet

        let _handle = thread::spawn(move || dummy_planet.run());

        // Start
        tx_orch_in.send(OrchestratorToPlanet::StartPlanetAI).unwrap();
        match rx_orch_out.recv_timeout(Duration::from_millis(50)) {
            Ok(PlanetToOrchestrator::StartPlanetAIResult { planet_id: 67 }) => {
                assert!(true)
            }
            _ => assert!(false),
        }

        //Incoming explorer
        tx_orch_in.send(OrchestratorToPlanet::IncomingExplorerRequest {explorer_id: 1, new_sender: tx_expl_out,}).unwrap();
        match rx_orch_out.recv_timeout(Duration::from_millis(50)) {
            Ok(PlanetToOrchestrator::IncomingExplorerResponse { planet_id:67, explorer_id: id, res:r }) => {
                assert_eq!(id, 1); //Checking the explorer id
                assert_eq!(r, Ok(())); //Checking response
            }
            _ => assert!(false),
        }

        //Outgoing explorer
        tx_orch_in.send(OrchestratorToPlanet::OutgoingExplorerRequest { explorer_id: 1 }).unwrap();
        match rx_orch_out.recv_timeout(Duration::from_millis(50)) {
            Ok(PlanetToOrchestrator::OutgoingExplorerResponse { planet_id:67, explorer_id:id, res:r }) => {
                assert_eq!(id, 1); //Checking the explorer id
                assert_eq!(r, Ok(())); //Checking response
            }
            _ => assert!(false),
        }

        // Stop
        tx_orch_in.send(OrchestratorToPlanet::StopPlanetAI).unwrap();
        match rx_orch_out.recv_timeout(Duration::from_millis(50)) {
            Ok(PlanetToOrchestrator::StopPlanetAIResult { planet_id: 67 }) => {
                assert!(true)
            }
            _ => assert!(false),
        }
        
        // Kill
        tx_orch_in.send(OrchestratorToPlanet::KillPlanet).unwrap();
        match rx_orch_out.recv_timeout(Duration::from_millis(50)) {
            Ok(PlanetToOrchestrator::KillPlanetResult { planet_id: 67 }) => {
                assert!(true)
            }
            _ => assert!(false),
        }
    }
    #[test]
    fn test_sunray_with_explorer(){
        let (tx_orch_in, rx_orch_in) = unbounded::<OrchestratorToPlanet>();
        let (tx_orch_out, rx_orch_out) = unbounded::<PlanetToOrchestrator>();
        let (_tx_expl_in, rx_expl_in) = unbounded::<ExplorerToPlanet>();
        let (tx_expl_out, _rx_expl_out) = unbounded::<PlanetToExplorer>();

        let mut dummy_planet = create_planet(67, rx_orch_in, tx_orch_out, rx_expl_in); // Creating the planet

        let _handle = thread::spawn(move || dummy_planet.run());

        // Start
        tx_orch_in.send(OrchestratorToPlanet::StartPlanetAI).unwrap();
        match rx_orch_out.recv_timeout(Duration::from_millis(50)) {
            Ok(PlanetToOrchestrator::StartPlanetAIResult { planet_id: 67 }) => {
                assert!(true)
            }
            _ => assert!(false),
        }

        //Incoming explorer
        tx_orch_in.send(OrchestratorToPlanet::IncomingExplorerRequest {explorer_id: 1, new_sender: tx_expl_out,}).unwrap();
        match rx_orch_out.recv_timeout(Duration::from_millis(50)) {
            Ok(PlanetToOrchestrator::IncomingExplorerResponse { planet_id:67, explorer_id: id, res:r }) => {
                assert_eq!(id, 1); //Checking the explorer id
                assert_eq!(r, Ok(())); //Checking response
            }
            _ => assert!(false),
        }

        //Send sunray
        tx_orch_in.send(OrchestratorToPlanet::Sunray(Sunray::default())).unwrap();
        match rx_orch_out.recv_timeout(Duration::from_millis(50)) {
            Ok(PlanetToOrchestrator::SunrayAck { planet_id: 67 }) => {
                assert!(true)
            }
            _ => assert!(false),
        }

        //Check internal state
        tx_orch_in.send(OrchestratorToPlanet::InternalStateRequest).unwrap();
        match rx_orch_out.recv_timeout(Duration::from_millis(50)) {
            Ok(PlanetToOrchestrator::InternalStateResponse { planet_id:67, planet_state:dummy_state }) => {
                assert!(!dummy_state.has_rocket); //The planet should have no rocket because it has an explorer
                assert_eq!(dummy_state.charged_cells_count, 1); //The planet should have 1 charged cell
            }
            _ => assert!(false),
        }

        //Send 2nd sunray
        tx_orch_in.send(OrchestratorToPlanet::Sunray(Sunray::default())).unwrap();
        match rx_orch_out.recv_timeout(Duration::from_millis(50)) {
            Ok(PlanetToOrchestrator::SunrayAck { planet_id: 67 }) => {
                assert!(true)
            }
            _ => assert!(false),
        }

        //Check internal state (again)
        tx_orch_in.send(OrchestratorToPlanet::InternalStateRequest).unwrap();
        match rx_orch_out.recv_timeout(Duration::from_millis(50)) {
            Ok(PlanetToOrchestrator::InternalStateResponse { planet_id:67, planet_state:dummy_state }) => {
                assert!(dummy_state.has_rocket); //The planet should have a rocket since the energy cell was already charged
                assert_eq!(dummy_state.charged_cells_count, 1); //The planet should have 1 charged cell
            }
            _ => assert!(false),
        }

        //Outgoing explorer
        tx_orch_in.send(OrchestratorToPlanet::OutgoingExplorerRequest { explorer_id: 1 }).unwrap();
        match rx_orch_out.recv_timeout(Duration::from_millis(50)) {
            Ok(PlanetToOrchestrator::OutgoingExplorerResponse { planet_id:67, explorer_id:id, res:r }) => {
                assert_eq!(id, 1); //Checking the explorer id
                assert_eq!(r, Ok(())); //Checking response
            }
            _ => assert!(false),
        }

        // Stop
        tx_orch_in.send(OrchestratorToPlanet::StopPlanetAI).unwrap();
        match rx_orch_out.recv_timeout(Duration::from_millis(50)) {
            Ok(PlanetToOrchestrator::StopPlanetAIResult { planet_id: 67 }) => {
                assert!(true)
            }
            _ => assert!(false),
        }
        
        // Kill
        tx_orch_in.send(OrchestratorToPlanet::KillPlanet).unwrap();
        match rx_orch_out.recv_timeout(Duration::from_millis(50)) {
            Ok(PlanetToOrchestrator::KillPlanetResult { planet_id: 67 }) => {
                assert!(true)
            }
            _ => assert!(false),
        }
    }

    //Test explorer request (available energy)
    #[test]
    fn test_explorer_available_energy_request() {
        let (tx_orch_in, rx_orch_in) = unbounded::<OrchestratorToPlanet>();
        let (tx_orch_out, rx_orch_out) = unbounded::<PlanetToOrchestrator>();
        let (tx_expl_in, rx_expl_in) = unbounded::<ExplorerToPlanet>();
        let (tx_expl_out, rx_expl_out) = unbounded::<PlanetToExplorer>();

        let mut dummy_planet = create_planet(67, rx_orch_in, tx_orch_out, rx_expl_in); // Creating the planet

        let _handle = thread::spawn(move || dummy_planet.run());

        // Start
        tx_orch_in.send(OrchestratorToPlanet::StartPlanetAI).unwrap();
        match rx_orch_out.recv_timeout(Duration::from_millis(50)) {
            Ok(PlanetToOrchestrator::StartPlanetAIResult { planet_id: 67 }) => {
                assert!(true)
            }
            _ => assert!(false),
        }

        //Incoming explorer
        tx_orch_in.send(OrchestratorToPlanet::IncomingExplorerRequest {explorer_id: 1, new_sender: tx_expl_out,}).unwrap();
        match rx_orch_out.recv_timeout(Duration::from_millis(50)) {
            Ok(PlanetToOrchestrator::IncomingExplorerResponse { planet_id:67, explorer_id: id, res:r }) => {
                assert_eq!(id, 1); //Checking the explorer id
                assert_eq!(r, Ok(())); //Checking response
            }
            _ => assert!(false),
        }

        //Explorer request
        tx_expl_in.send(ExplorerToPlanet::AvailableEnergyCellRequest { explorer_id: 1 }).unwrap();
        match rx_expl_out.recv_timeout(Duration::from_millis(100)) {
            Ok(PlanetToExplorer::AvailableEnergyCellResponse { available_cells: energy }) => {
                assert_eq!(energy, 0) //There should be no energy cell
            }
            _ => assert!(false),
        }

        //Send sunray
        tx_orch_in.send(OrchestratorToPlanet::Sunray(Sunray::default())).unwrap();
        match rx_orch_out.recv_timeout(Duration::from_millis(50)) {
            Ok(PlanetToOrchestrator::SunrayAck { planet_id: 67 }) => {
                assert!(true)
            }
            _ => assert!(false),
        }

        //Explorer request
        tx_expl_in.send(ExplorerToPlanet::AvailableEnergyCellRequest { explorer_id: 1 }).unwrap();
        match rx_expl_out.recv_timeout(Duration::from_millis(100)) {
            Ok(PlanetToExplorer::AvailableEnergyCellResponse { available_cells: energy }) => {
                assert_eq!(energy, 1) //There should be one energy cell
            }
            _ => assert!(false),
        }        

        //Outgoing explorer
        tx_orch_in.send(OrchestratorToPlanet::OutgoingExplorerRequest { explorer_id: 1 }).unwrap();
        match rx_orch_out.recv_timeout(Duration::from_millis(50)) {
            Ok(PlanetToOrchestrator::OutgoingExplorerResponse { planet_id:67, explorer_id:id, res:r }) => {
                assert_eq!(id, 1); //Checking the explorer id
                assert_eq!(r, Ok(())); //Checking response
            }
            _ => assert!(false),
        }

        // Stop
        tx_orch_in.send(OrchestratorToPlanet::StopPlanetAI).unwrap();
        match rx_orch_out.recv_timeout(Duration::from_millis(50)) {
            Ok(PlanetToOrchestrator::StopPlanetAIResult { planet_id: 67 }) => {
                assert!(true)
            }
            _ => assert!(false),
        }
        
        // Kill
        tx_orch_in.send(OrchestratorToPlanet::KillPlanet).unwrap();
        match rx_orch_out.recv_timeout(Duration::from_millis(50)) {
            Ok(PlanetToOrchestrator::KillPlanetResult { planet_id: 67 }) => {
                assert!(true)
            }
            _ => assert!(false),
        }
    }

    #[test]
    fn test_explorer_generate_carbon() {
        let (tx_orch_in, rx_orch_in) = unbounded::<OrchestratorToPlanet>();
        let (tx_orch_out, rx_orch_out) = unbounded::<PlanetToOrchestrator>();
        let (tx_expl_in, rx_expl_in) = unbounded::<ExplorerToPlanet>();
        let (tx_expl_out, rx_expl_out) = unbounded::<PlanetToExplorer>();

        let mut dummy_planet = create_planet(67, rx_orch_in, tx_orch_out, rx_expl_in); // Creating the planet
        
        let _handle = thread::spawn(move || dummy_planet.run());

        // Start
        tx_orch_in.send(OrchestratorToPlanet::StartPlanetAI).unwrap();
        match rx_orch_out.recv_timeout(Duration::from_millis(50)) {
            Ok(PlanetToOrchestrator::StartPlanetAIResult { planet_id: 67 }) => {
                assert!(true)
            }
            _ => assert!(false),
        }

        //Incoming explorer
        tx_orch_in.send(OrchestratorToPlanet::IncomingExplorerRequest {explorer_id: 1, new_sender: tx_expl_out,}).unwrap();
        match rx_orch_out.recv_timeout(Duration::from_millis(50)) {
            Ok(PlanetToOrchestrator::IncomingExplorerResponse { planet_id:67, explorer_id: id, res:r }) => {
                assert_eq!(id, 1); //Checking the explorer id
                assert_eq!(r, Ok(())); //Checking response
            }
            _ => assert!(false),
        }

        //Generate carbon request
        tx_expl_in.send(ExplorerToPlanet::GenerateResourceRequest {explorer_id: 1, resource: BasicResourceType::Carbon,}).unwrap();

        match rx_expl_out.recv_timeout(Duration::from_millis(100)) {
            Ok(PlanetToExplorer::GenerateResourceResponse { resource: r }) => {
                assert!(r.is_none()) //It should be none because there is no energy cell
            }
            _ => assert!(false),
        }

        //Send sunray
        tx_orch_in.send(OrchestratorToPlanet::Sunray(Sunray::default())).unwrap();
        match rx_orch_out.recv_timeout(Duration::from_millis(50)) {
            Ok(PlanetToOrchestrator::SunrayAck { planet_id: 67 }) => {
                assert!(true)
            }
            _ => assert!(false),
        }

        //Generate other resource request
        tx_expl_in.send(ExplorerToPlanet::GenerateResourceRequest {explorer_id: 1, resource: BasicResourceType::Hydrogen,}).unwrap();

        match rx_expl_out.recv_timeout(Duration::from_millis(100)) {
            Ok(PlanetToExplorer::GenerateResourceResponse { resource: r }) => {
                assert!(r.is_none()) //It should be none because Enterprise does not generate Hydrogen
            }
            _ => assert!(false),
        }

        //Generate carbon request
        tx_expl_in.send(ExplorerToPlanet::GenerateResourceRequest {explorer_id: 1, resource: BasicResourceType::Carbon,}).unwrap();

        match rx_expl_out.recv_timeout(Duration::from_millis(100)) {
            Ok(PlanetToExplorer::GenerateResourceResponse { resource: r }) => {
                assert!(r.is_some()) //It should be some because there is energy cell and Enterprise can generate Carbon
            }
            _ => assert!(false),
        }

        //Check internal state
        tx_orch_in.send(OrchestratorToPlanet::InternalStateRequest).unwrap();
        match rx_orch_out.recv_timeout(Duration::from_millis(50)) {
            Ok(PlanetToOrchestrator::InternalStateResponse { planet_id:67, planet_state:dummy_state }) => {
                assert!(!dummy_state.has_rocket); //The planet should have no rocket because it has an explorer
                assert_eq!(dummy_state.charged_cells_count, 0); //The planet should have 0 charged cells since it was used to generate carbon
            }
            _ => assert!(false),
        }

        //Outgoing explorer
        tx_orch_in.send(OrchestratorToPlanet::OutgoingExplorerRequest { explorer_id: 1 }).unwrap();
        match rx_orch_out.recv_timeout(Duration::from_millis(50)) {
            Ok(PlanetToOrchestrator::OutgoingExplorerResponse { planet_id:67, explorer_id:id, res:r }) => {
                assert_eq!(id, 1); //Checking the explorer id
                assert_eq!(r, Ok(())); //Checking response
            }
            _ => assert!(false),
        }

        // Stop
        tx_orch_in.send(OrchestratorToPlanet::StopPlanetAI).unwrap();
        match rx_orch_out.recv_timeout(Duration::from_millis(50)) {
            Ok(PlanetToOrchestrator::StopPlanetAIResult { planet_id: 67 }) => {
                assert!(true)
            }
            _ => assert!(false),
        }
        
        // Kill
        tx_orch_in.send(OrchestratorToPlanet::KillPlanet).unwrap();
        match rx_orch_out.recv_timeout(Duration::from_millis(50)) {
            Ok(PlanetToOrchestrator::KillPlanetResult { planet_id: 67 }) => {
                assert!(true)
            }
            _ => assert!(false),
        }
    }
    
    #[test]
    fn test_asteroid_energy_cell_no_rocket() {
        let (tx_orch_in, rx_orch_in) = unbounded::<OrchestratorToPlanet>();
        let (tx_orch_out, rx_orch_out) = unbounded::<PlanetToOrchestrator>();
        let (_tx_expl_in, rx_expl_in) = unbounded::<ExplorerToPlanet>();
        let (tx_expl_out, _rx_expl_out) = unbounded::<PlanetToExplorer>();

        let mut dummy_planet = create_planet(67, rx_orch_in, tx_orch_out, rx_expl_in); // Creating the planet

        let _handle = thread::spawn(move || dummy_planet.run());

        // Start
        tx_orch_in.send(OrchestratorToPlanet::StartPlanetAI).unwrap();
        match rx_orch_out.recv_timeout(Duration::from_millis(50)) {
            Ok(PlanetToOrchestrator::StartPlanetAIResult { planet_id: 67 }) => {
                assert!(true)
            }
            _ => assert!(false),
        }

        //Incoming explorer
        tx_orch_in.send(OrchestratorToPlanet::IncomingExplorerRequest {explorer_id: 1, new_sender: tx_expl_out,}).unwrap();
        match rx_orch_out.recv_timeout(Duration::from_millis(50)) {
            Ok(PlanetToOrchestrator::IncomingExplorerResponse { planet_id:67, explorer_id: id, res:r }) => {
                assert_eq!(id, 1); //Checking the explorer id
                assert_eq!(r, Ok(())); //Checking response
            }
            _ => assert!(false),
        }

        //Send sunray
        tx_orch_in.send(OrchestratorToPlanet::Sunray(Sunray::default())).unwrap();
        match rx_orch_out.recv_timeout(Duration::from_millis(50)) {
            Ok(PlanetToOrchestrator::SunrayAck { planet_id: 67 }) => {
                assert!(true)
            }
            _ => assert!(false),
        }

        //Check internal state (no rocket, only an energy cell)
        tx_orch_in.send(OrchestratorToPlanet::InternalStateRequest).unwrap();
        match rx_orch_out.recv_timeout(Duration::from_millis(50)) {
            Ok(PlanetToOrchestrator::InternalStateResponse { planet_id:67, planet_state:dummy_state }) => {
                assert!(!dummy_state.has_rocket); //The planet should have no rocket because it has an explorer
                assert_eq!(dummy_state.charged_cells_count, 1); //The planet should have 1 charged cell
            }
            _ => assert!(false),
        }

        //Send asteroid
        tx_orch_in.send(OrchestratorToPlanet::Asteroid(Asteroid::default())).unwrap();
        match rx_orch_out.recv_timeout(Duration::from_millis(50)) {
            Ok(PlanetToOrchestrator::AsteroidAck { planet_id:67, rocket:r }) => {
                assert!(r.is_some()); //A new rocket should be built using the energy cell
            }
            _ => assert!(false),
        }

        //Check internal state (again)
        tx_orch_in.send(OrchestratorToPlanet::InternalStateRequest).unwrap();
        match rx_orch_out.recv_timeout(Duration::from_millis(50)) {
            Ok(PlanetToOrchestrator::InternalStateResponse { planet_id:67, planet_state:dummy_state }) => {
                assert!(!dummy_state.has_rocket); //The planet should have no rocket
                assert_eq!(dummy_state.charged_cells_count, 0); //The planet should have no charged cell sice it was used to build a rocket
            }
            _ => assert!(false),
        }

        //Outgoing explorer
        tx_orch_in.send(OrchestratorToPlanet::OutgoingExplorerRequest { explorer_id: 1 }).unwrap();
        match rx_orch_out.recv_timeout(Duration::from_millis(50)) {
            Ok(PlanetToOrchestrator::OutgoingExplorerResponse { planet_id:67, explorer_id:id, res:r }) => {
                assert_eq!(id, 1); //Checking the explorer id
                assert_eq!(r, Ok(())); //Checking response
            }
            _ => assert!(false),
        }

        // Stop
        tx_orch_in.send(OrchestratorToPlanet::StopPlanetAI).unwrap();
        match rx_orch_out.recv_timeout(Duration::from_millis(50)) {
            Ok(PlanetToOrchestrator::StopPlanetAIResult { planet_id: 67 }) => {
                assert!(true)
            }
            _ => assert!(false),
        }
        
        // Kill
        tx_orch_in.send(OrchestratorToPlanet::KillPlanet).unwrap();
        match rx_orch_out.recv_timeout(Duration::from_millis(50)) {
            Ok(PlanetToOrchestrator::KillPlanetResult { planet_id: 67 }) => {
                assert!(true)
            }
            _ => assert!(false),
        }
    }

    #[test]
    fn test_two_explorers() {
        let (tx_orch_in, rx_orch_in) = unbounded::<OrchestratorToPlanet>();
        let (tx_orch_out, rx_orch_out) = unbounded::<PlanetToOrchestrator>();
        let (_tx_expl_in, rx_expl_in) = unbounded::<ExplorerToPlanet>();
        //First explorer
        let (tx_expl_out_f, _rx_expl_out_f) = unbounded::<PlanetToExplorer>();
        //Second explorer
        let (tx_expl_out_s, _rx_expl_out_s) = unbounded::<PlanetToExplorer>();

        let mut dummy_planet = create_planet(67, rx_orch_in, tx_orch_out, rx_expl_in); // Creating the planet

        let _handle = thread::spawn(move || dummy_planet.run());

        // Start
        tx_orch_in.send(OrchestratorToPlanet::StartPlanetAI).unwrap();
        match rx_orch_out.recv_timeout(Duration::from_millis(50)) {
            Ok(PlanetToOrchestrator::StartPlanetAIResult { planet_id: 67 }) => {
                assert!(true)
            }
            _ => assert!(false),
        }

        //Incoming explorer (FIRST explorer)
        tx_orch_in.send(OrchestratorToPlanet::IncomingExplorerRequest {explorer_id: 1, new_sender: tx_expl_out_f,}).unwrap();
        match rx_orch_out.recv_timeout(Duration::from_millis(50)) {
            Ok(PlanetToOrchestrator::IncomingExplorerResponse { planet_id:67, explorer_id: id, res:r }) => {
                assert_eq!(id, 1); //Checking the explorer id
                assert_eq!(r, Ok(())); //Checking response
            }
            _ => assert!(false),
        }

        //Incoming explorer (SECOND explorer)
        tx_orch_in.send(OrchestratorToPlanet::IncomingExplorerRequest {explorer_id: 2, new_sender: tx_expl_out_s,}).unwrap();
        match rx_orch_out.recv_timeout(Duration::from_millis(50)) {
            Ok(PlanetToOrchestrator::IncomingExplorerResponse { planet_id:67, explorer_id: id, res:r }) => {
                assert_eq!(id, 2); //Checking the explorer id
                assert_eq!(r, Ok(())); //Checking response
            }
            _ => assert!(false),
        }

        //Send sunray
        tx_orch_in.send(OrchestratorToPlanet::Sunray(Sunray::default())).unwrap();
        match rx_orch_out.recv_timeout(Duration::from_millis(50)) {
            Ok(PlanetToOrchestrator::SunrayAck { planet_id: 67 }) => {
                assert!(true)
            }
            _ => assert!(false),
        }

        //Check internal state (no rocket, only an energy cell)
        tx_orch_in.send(OrchestratorToPlanet::InternalStateRequest).unwrap();
        match rx_orch_out.recv_timeout(Duration::from_millis(50)) {
            Ok(PlanetToOrchestrator::InternalStateResponse { planet_id:67, planet_state:dummy_state }) => {
                assert!(!dummy_state.has_rocket); //The planet should have no rocket because it has an explorer
                assert_eq!(dummy_state.charged_cells_count, 1); //The planet should have 1 charged cell
            }
            _ => assert!(false),
        }

        //Send asteroid
        tx_orch_in.send(OrchestratorToPlanet::Asteroid(Asteroid::default())).unwrap();
        match rx_orch_out.recv_timeout(Duration::from_millis(50)) {
            Ok(PlanetToOrchestrator::AsteroidAck { planet_id:67, rocket:r }) => {
                assert!(r.is_some()); //A new rocket should be built using the energy cell
            }
            _ => assert!(false),
        }

        //Check internal state (again)
        tx_orch_in.send(OrchestratorToPlanet::InternalStateRequest).unwrap();
        match rx_orch_out.recv_timeout(Duration::from_millis(50)) {
            Ok(PlanetToOrchestrator::InternalStateResponse { planet_id:67, planet_state:dummy_state }) => {
                assert!(!dummy_state.has_rocket); //The planet should have no rocket
                assert_eq!(dummy_state.charged_cells_count, 0); //The planet should have no charged cell sice it was used to build a rocket
            }
            _ => assert!(false),
        }

        //Outgoing explorer (FIRST explorer)
        tx_orch_in.send(OrchestratorToPlanet::OutgoingExplorerRequest { explorer_id: 1 }).unwrap();
        match rx_orch_out.recv_timeout(Duration::from_millis(50)) {
            Ok(PlanetToOrchestrator::OutgoingExplorerResponse { planet_id:67, explorer_id:id, res:r }) => {
                assert_eq!(id, 1); //Checking the explorer id
                assert_eq!(r, Ok(())); //Checking response
            }
            _ => assert!(false),
        }

        //Check the behaviour of the planet again. It should behave the same way since there is still one explorer in the planet

        //Send sunray
        tx_orch_in.send(OrchestratorToPlanet::Sunray(Sunray::default())).unwrap();
        match rx_orch_out.recv_timeout(Duration::from_millis(50)) {
            Ok(PlanetToOrchestrator::SunrayAck { planet_id: 67 }) => {
                assert!(true)
            }
            _ => assert!(false),
        }

        //Check internal state (no rocket, only an energy cell)
        tx_orch_in.send(OrchestratorToPlanet::InternalStateRequest).unwrap();
        match rx_orch_out.recv_timeout(Duration::from_millis(50)) {
            Ok(PlanetToOrchestrator::InternalStateResponse { planet_id:67, planet_state:dummy_state }) => {
                assert!(!dummy_state.has_rocket); //The planet should have no rocket because it has an explorer
                assert_eq!(dummy_state.charged_cells_count, 1); //The planet should have 1 charged cell
            }
            _ => assert!(false),
        }

        //Send asteroid
        tx_orch_in.send(OrchestratorToPlanet::Asteroid(Asteroid::default())).unwrap();
        match rx_orch_out.recv_timeout(Duration::from_millis(50)) {
            Ok(PlanetToOrchestrator::AsteroidAck { planet_id:67, rocket:r }) => {
                assert!(r.is_some()); //A new rocket should be built using the energy cell
            }
            _ => assert!(false),
        }

        //Check internal state (again)
        tx_orch_in.send(OrchestratorToPlanet::InternalStateRequest).unwrap();
        match rx_orch_out.recv_timeout(Duration::from_millis(50)) {
            Ok(PlanetToOrchestrator::InternalStateResponse { planet_id:67, planet_state:dummy_state }) => {
                assert!(!dummy_state.has_rocket); //The planet should have no rocket
                assert_eq!(dummy_state.charged_cells_count, 0); //The planet should have no charged cell sice it was used to build a rocket
            }
            _ => assert!(false),
        }

        //Outgoing explorer (SECOND explorer)
        tx_orch_in.send(OrchestratorToPlanet::OutgoingExplorerRequest { explorer_id: 2 }).unwrap();
        match rx_orch_out.recv_timeout(Duration::from_millis(50)) {
            Ok(PlanetToOrchestrator::OutgoingExplorerResponse { planet_id:67, explorer_id:id, res:r }) => {
                assert_eq!(id, 2); //Checking the explorer id
                assert_eq!(r, Ok(())); //Checking response
            }
            _ => assert!(false),
        }

        //Check the behaviour of the planet again. It should behave the differently since there is no explorer in the planet

        //Send sunray
        tx_orch_in.send(OrchestratorToPlanet::Sunray(Sunray::default())).unwrap();
        match rx_orch_out.recv_timeout(Duration::from_millis(50)) {
            Ok(PlanetToOrchestrator::SunrayAck { planet_id: 67 }) => {
                assert!(true)
            }
            _ => assert!(false),
        }

        //Check internal state (one rocket, no energy cell)
        tx_orch_in.send(OrchestratorToPlanet::InternalStateRequest).unwrap();
        match rx_orch_out.recv_timeout(Duration::from_millis(50)) {
            Ok(PlanetToOrchestrator::InternalStateResponse { planet_id:67, planet_state:dummy_state }) => {
                assert!(dummy_state.has_rocket); //The planet should have a rocket because it has no explorer
                assert_eq!(dummy_state.charged_cells_count, 0); //The planet should have no charged cell
            }
            _ => assert!(false),
        }


        // Stop
        tx_orch_in.send(OrchestratorToPlanet::StopPlanetAI).unwrap();
        match rx_orch_out.recv_timeout(Duration::from_millis(50)) {
            Ok(PlanetToOrchestrator::StopPlanetAIResult { planet_id: 67 }) => {
                assert!(true)
            }
            _ => assert!(false),
        }
        
        // Kill
        tx_orch_in.send(OrchestratorToPlanet::KillPlanet).unwrap();
        match rx_orch_out.recv_timeout(Duration::from_millis(50)) {
            Ok(PlanetToOrchestrator::KillPlanetResult { planet_id: 67 }) => {
                assert!(true)
            }
            _ => assert!(false),
        }
    }
}