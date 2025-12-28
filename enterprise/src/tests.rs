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
    use common_game::logging::Payload;
    use common_game::protocols::orchestrator_planet::*;
    use common_game::protocols::planet_explorer::*;
    use crossbeam_channel::{Receiver, Sender, unbounded};
    use std::thread;
    use std::time::Duration;

    use log::{Log,Level,Metadata,Record};
    use std::sync::{
        Mutex,
        Once    // low-level synchronization primitive for one-time global execution
                // (the given closure will be executed if this is the first time call_once has been called), and otherwise the routine will not be invoked
    };
    
    // custom test logger implementing the Log trait
    // it memorizes all received messages in a synchronized vec for testing purposes
    struct TestLogger{
        messages: Mutex<Vec<(Level,String)>>
    }

    // test logger for capturing emitted messages
    // it memorizes a list of 2D-tuples for verifying the messages are emitted with apropriate content and level
    static TEST_LOGGER:TestLogger=TestLogger{
        messages:Mutex::new(vec![])
    };

    // the Once type ensures that the test logger is initialized only once
    static LOGGER_INIT:Once=Once::new();

    impl Log for TestLogger{
        fn enabled(&self, _metadata: &Metadata) -> bool {
            true    // enables all log levels for the tests
        }

        fn log(&self, record: &Record) {
            let mut guard = self.messages.lock().expect("logger mutex poisoned");
            guard.push((record.level(), format!("{}", record.args())));
        }

        fn flush(&self) {}
    }

    fn init_test_logger(){
        LOGGER_INIT.call_once(|| {
            log::set_logger(&TEST_LOGGER).expect("failed to install logger");
            log::set_max_level(log::LevelFilter::Trace);    
        });

        // clearing all previous messages for ensuging isolated tests
        TEST_LOGGER.messages.lock().unwrap().clear();
    }

    fn get_logged_messages()->Vec<(Level,String)>{
        TEST_LOGGER.messages.lock().unwrap().clone()
    }

    // verify that a log message contains a certain string
    fn assert_log_contains(level:Level,content:&str){
        let messages=get_logged_messages();
        let found = messages.iter().any(|(l,c)| *l == level && c.contains(content));

        assert!(found);
    }

    // fn assert_log_contains_all(level:Level,contents:Vec<&str>){
    //     let messages=get_logged_messages();
    //     for content in contents{
    //         let found = messages.iter().any(|(l,c)| *l==level && c.contains(content));
    //         assert!(found);
    //     }
    // }

    fn assert_log_not_contains(level:Level,content:&str){
        let messages=get_logged_messages();
        let found = messages.iter().any(|(l,c)| *l == level && c.contains(content));

        assert!(!found);
    }

    // fn count_logs_at_level(level:Level)->usize{
    //     let messages=get_logged_messages();
    //     messages.iter().filter(|(l,_)| *l==level).count()
    // }


    // functions for creating sample payloads simulating real data for tests
    // fn sample_payload()->Payload{
    //     let mut payload = Payload::new();
    //     payload.insert("key".to_string(), "value".to_string());
    //     payload.insert("action".to_string(), "test_action".to_string());
    //     payload
    // }

    /// Test that our planet can be created and follows Type C constraints
    #[test]
    fn test_planet_creation_and_type_constraints() {
        init_test_logger();

        let id = 123;
        
        // Setup
        let (_orch_tx, planet_rx) = unbounded();
        let (planet_tx, _orch_rx) = unbounded();
        let (_expl_tx, expl_rx) = unbounded();
        
        // Create planet
        let planet = create_planet(id, planet_rx, planet_tx, expl_rx);

        thread::sleep(Duration::from_millis(10));
        assert_log_contains(Level::Info, "init");
        assert_log_contains(Level::Info, "planet_type");
        assert_log_contains(Level::Info, "C");
        assert_log_contains(Level::Info, &id.to_string());
        
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
        init_test_logger();

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


        thread::sleep(Duration::from_millis(10));
        assert_log_contains(Level::Info, "started");
        assert_log_contains(Level::Info, "explorer_count");
        assert_log_contains(Level::Info, "0");
        
        assert_log_contains(Level::Debug, "is_running");
        assert_log_contains(Level::Debug, "true");

        
        // Stop
        orch_tx.send(OrchestratorToPlanet::StopPlanetAI).unwrap();
        match orch_rx.recv_timeout(Duration::from_millis(50)) {
            Ok(PlanetToOrchestrator::StopPlanetAIResult { planet_id: n }) => {
                assert_eq!(n, id)
            }
            _ => assert!(false),
        }

        // Verifica log di stop
        thread::sleep(Duration::from_millis(10));
        assert_log_contains(Level::Info, "stopped");
        
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
        init_test_logger();

        let ai = EnterpriseAi::new(67);

        assert_log_contains(Level::Info, "init");
        assert_log_contains(Level::Info, "planet_type");
        assert_log_contains(Level::Info, "C");
        assert_log_contains(Level::Info, "67");

        assert_eq!(ai.planet_id, 67);
        assert!(!ai.is_running());
        assert_eq!(ai.num_explorers, 0);
    }

    //Test the behaviour of the planet when receiving sunrays and no explorer is present
    #[test]
    fn test_sunray_no_explorer() {
        init_test_logger();

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

        assert_log_contains(Level::Info, "started");
        assert_log_contains(Level::Debug, "is_running");

        //Send sunray
        tx_orch_in.send(OrchestratorToPlanet::Sunray(Sunray::default())).unwrap();
        match rx_orch_out.recv_timeout(Duration::from_millis(50)) {
            Ok(PlanetToOrchestrator::SunrayAck { planet_id: 67 }) => {
                assert!(true)
            }
            _ => assert!(false),
        }


        assert_log_contains(Level::Info, "sunray_ack");
        assert_log_contains(Level::Debug, "visiting_explorers");
        assert_log_contains(Level::Debug, "0");
        assert_log_contains(Level::Debug, "had_charged_cells");
        assert_log_contains(Level::Debug, "false");
        
        assert_log_contains(Level::Info, "built_rocket");
        
        assert_log_contains(Level::Debug, "final_has_rocket");
        assert_log_contains(Level::Debug, "true");
        assert_log_contains(Level::Debug, "rocket_build");
        assert_log_contains(Level::Debug, "true");


        //Check internal state
        tx_orch_in.send(OrchestratorToPlanet::InternalStateRequest).unwrap();
        match rx_orch_out.recv_timeout(Duration::from_millis(50)) {
            Ok(PlanetToOrchestrator::InternalStateResponse { planet_id:67, planet_state:dummy_state }) => {
                assert!(dummy_state.has_rocket); //The planet should have a rocket because it has no explorer
                assert_eq!(dummy_state.charged_cells_count, 0); //The planet should have 0 charged cells because the energy cell was used to build the rocket
            }
            _ => assert!(false),
        }

        assert_log_contains(Level::Info, "internal_state");

        //Send 2nd sunray
        tx_orch_in.send(OrchestratorToPlanet::Sunray(Sunray::default())).unwrap();
        match rx_orch_out.recv_timeout(Duration::from_millis(50)) {
            Ok(PlanetToOrchestrator::SunrayAck { planet_id: 67 }) => {
                assert!(true)
            }
            _ => assert!(false),
        }

        assert_log_contains(Level::Debug, "sunray_used");
        assert_log_contains(Level::Debug, "has_charged_cells");
        assert_log_contains(Level::Debug, "true");

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
        init_test_logger();

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

        assert_log_contains(Level::Info, "built_rocket");

        //Send asteroid
        tx_orch_in.send(OrchestratorToPlanet::Asteroid(Asteroid::default())).unwrap();
        match rx_orch_out.recv_timeout(Duration::from_millis(50)) {
            Ok(PlanetToOrchestrator::AsteroidAck { planet_id:67, rocket:r }) => {
                assert!(r.is_some()); //There should be a rocket to destroy the asteroid
            }
            _ => assert!(false),
        }

        assert_log_contains(Level::Debug, "handle_asteroid_start");
        assert_log_contains(Level::Debug, "has_rocket");
        assert_log_contains(Level::Debug, "true");
        assert_log_contains(Level::Info, "using_existing_rocket");

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
        init_test_logger();

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

        assert_log_contains(Level::Warn, "no_charged_cell_for_rocket");

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
        init_test_logger();

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


        assert_log_contains(Level::Info, "explorer_arrival");
        assert_log_contains(Level::Debug, "in_explorer_id");
        assert_log_contains(Level::Debug, "1");
        assert_log_contains(Level::Debug, "visiting_explorers");
        assert_log_contains(Level::Debug, "1");

        //Outgoing explorer
        tx_orch_in.send(OrchestratorToPlanet::OutgoingExplorerRequest { explorer_id: 1 }).unwrap();
        match rx_orch_out.recv_timeout(Duration::from_millis(50)) {
            Ok(PlanetToOrchestrator::OutgoingExplorerResponse { planet_id:67, explorer_id:id, res:r }) => {
                assert_eq!(id, 1); //Checking the explorer id
                assert_eq!(r, Ok(())); //Checking response
            }
            _ => assert!(false),
        }


        assert_log_contains(Level::Info, "explorer_departure");
        assert_log_contains(Level::Debug, "out_explorer_id");
        assert_log_contains(Level::Debug, "1");
        assert_log_contains(Level::Debug, "visiting_explorers");
        assert_log_contains(Level::Debug, "0");

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
        init_test_logger();

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

        assert_log_contains(Level::Debug, "visiting_explorers");
        assert_log_contains(Level::Debug, "1");
        assert_log_contains(Level::Debug, "sunray_used");
        assert_log_not_contains(Level::Info, "built_rocket");

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

        assert_log_contains(Level::Debug, "sunray_wasted");
        assert_log_contains(Level::Warn, "has_built_rocket_with_new_energy_cell");
        assert_log_contains(Level::Warn, "true");

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
        init_test_logger();

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


        assert_log_contains(Level::Info, "message_type");
        assert_log_contains(Level::Info, "AvailableEnergyCellRequest");
        assert_log_contains(Level::Info, "explorer_id");
        assert_log_contains(Level::Info, "1");
        assert_log_contains(Level::Info, "available_energy_cells");
        assert_log_contains(Level::Info, "count");
        assert_log_contains(Level::Info, "0");

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


        let messages = get_logged_messages();
        let energy_count_logs: Vec<_> = messages.iter()
            .filter(|(l, msg)| *l == Level::Info && msg.contains("count") && msg.contains("1"))
            .collect();
        assert!(!energy_count_logs.is_empty());        

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
        init_test_logger();

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

        assert_log_contains(Level::Warn, "no_charged_cell");

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

        assert_log_contains(Level::Warn, "unsupported_resource");
        assert_log_contains(Level::Warn, "Hydrogen");

        //Generate carbon request
        tx_expl_in.send(ExplorerToPlanet::GenerateResourceRequest {explorer_id: 1, resource: BasicResourceType::Carbon,}).unwrap();

        match rx_expl_out.recv_timeout(Duration::from_millis(100)) {
            Ok(PlanetToExplorer::GenerateResourceResponse { resource: r }) => {
                assert!(r.is_some()) //It should be some because there is energy cell and Enterprise can generate Carbon
            }
            _ => assert!(false),
        }

        assert_log_contains(Level::Debug, "using_charged_cell");
        assert_log_contains(Level::Info, "resource_generated");
        assert_log_contains(Level::Info, "Carbon");


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
        init_test_logger();

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

        assert_log_contains(Level::Warn, "no_charged_cell_for_rocket");
        let messages = get_logged_messages();
        let built_rocket_for_defense = messages.iter()
            .any(|(l, msg)| *l == Level::Info && msg.contains("using_existing_rocket"));
        assert!(built_rocket_for_defense);

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
        init_test_logger();

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

        assert_log_contains(Level::Debug, "visiting_explorers");
        assert_log_contains(Level::Debug, "1");

        //Incoming explorer (SECOND explorer)
        tx_orch_in.send(OrchestratorToPlanet::IncomingExplorerRequest {explorer_id: 2, new_sender: tx_expl_out_s,}).unwrap();
        match rx_orch_out.recv_timeout(Duration::from_millis(50)) {
            Ok(PlanetToOrchestrator::IncomingExplorerResponse { planet_id:67, explorer_id: id, res:r }) => {
                assert_eq!(id, 2); //Checking the explorer id
                assert_eq!(r, Ok(())); //Checking response
            }
            _ => assert!(false),
        }

        let messages = get_logged_messages();
        let explorer_count_logs: Vec<_> = messages.iter()
            .filter(|(l, msg)| *l == Level::Debug && msg.contains("visiting_explorers") && msg.contains("2"))
            .collect();
        assert!(!explorer_count_logs.is_empty());

        //Send sunray
        tx_orch_in.send(OrchestratorToPlanet::Sunray(Sunray::default())).unwrap();
        match rx_orch_out.recv_timeout(Duration::from_millis(50)) {
            Ok(PlanetToOrchestrator::SunrayAck { planet_id: 67 }) => {
                assert!(true)
            }
            _ => assert!(false),
        }

        assert_log_contains(Level::Debug, "visiting_explorers");
        let sunray_messages: Vec<_> = messages.iter()
            .filter(|(l, msg)| *l == Level::Debug && msg.contains("visiting_explorers"))
            .collect();
        assert!(!sunray_messages.is_empty());

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

        let messages_after_first = get_logged_messages();
        let explorer_count_after_first: Vec<_> = messages_after_first.iter()
            .filter(|(l, msg)| *l == Level::Debug && msg.contains("visiting_explorers") && msg.contains("1"))
            .collect();
        assert!(!explorer_count_after_first.is_empty());

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


        let messages_final = get_logged_messages();
        let explorer_count_final: Vec<_> = messages_final.iter()
            .filter(|(l, msg)| *l == Level::Debug && msg.contains("visiting_explorers") && msg.contains("0"))
            .collect();
        assert!(!explorer_count_final.is_empty());

        //Check the behaviour of the planet again. It should behave the differently since there is no explorer in the planet

        //Send sunray
        tx_orch_in.send(OrchestratorToPlanet::Sunray(Sunray::default())).unwrap();
        match rx_orch_out.recv_timeout(Duration::from_millis(50)) {
            Ok(PlanetToOrchestrator::SunrayAck { planet_id: 67 }) => {
                assert!(true)
            }
            _ => assert!(false),
        }


        assert_log_contains(Level::Info, "built_rocket");

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

    #[test]
    fn test_logging_patterns() {
        init_test_logger();
        
        let _ai = EnterpriseAi::new(67);
        
        let messages = get_logged_messages();
        
        let init_log_found = messages.iter()
            .any(|(level, msg)| {
                *level == Level::Info &&
                msg.contains("init") &&
                msg.contains("planet_type") &&
                msg.contains("C") &&
                msg.contains("67")
            });
        
        assert!(init_log_found);
        
        let init_log = messages.iter()
            .find(|(level, msg)| *level == Level::Info && msg.contains("init"))
            .expect("init log not found");
        
        assert!(init_log.1.contains("LogEvent"));
        assert!(init_log.1.contains("actor_type"));
        assert!(init_log.1.contains("id"));
        assert!(init_log.1.contains("event_type"));
        assert!(init_log.1.contains("channel"));
        assert!(init_log.1.contains("payload"));
    }
}
