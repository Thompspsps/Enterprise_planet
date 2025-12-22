#[cfg(test)]
mod tests {
    use super::super::ai::;
    use crossbeam_channel::{unbounded, bounded};
    use std::thread;
    use std::time::Duration;
    
    /// Test that our planet can be created and follows Type C constraints
    #[test]
    fn test_planet_creation_and_type_constraints() {
        let id = 123;
        
        // Setup
        let (orch_tx, planet_rx) = unbounded();
        let (planet_tx, _orch_rx) = unbounded();
        let (expl_tx, _expl_rx) = unbounded();
        
        // Create planet
        let planet = create_planet(id, planet_rx, planet_tx, expl_tx);
        
        // Assertions
        assert_eq!(planet.id(), id);
        assert!(matches!(planet.planet_type(), PlanetType::C));
        
        // Type C has 1 energy cell capacity
        let state = planet.state();
        assert_eq!(state.cells_count(), 1);
        
        // Type C can have rockets
        assert!(state.can_have_rocket());
        
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
        let (expl_tx, _expl_rx) = unbounded();
        
        let mut planet = create_planet(id, planet_rx, planet_tx, expl_tx);
        
        // Run in background thread
        let handle = thread::spawn(move || {
            let _ = planet.run();
        });
        
        // Start
        orch_tx.send(OrchestratorToPlanet::StartPlanetAI).unwrap();
        assert_matches!(
            orch_rx.recv_timeout(Duration::from_millis(500)),
            Ok(PlanetToOrchestrator::StartPlanetAIResult { planet_id: id })
        );
        
        // Stop
        orch_tx.send(OrchestratorToPlanet::StopPlanetAI).unwrap();
        assert_matches!(
            orch_rx.recv_timeout(Duration::from_millis(500)),
            Ok(PlanetToOrchestrator::StopPlanetAIResult { planet_id: id })
        );
        
        // Kill
        orch_tx.send(OrchestratorToPlanet::KillPlanet).unwrap();
        assert_matches!(
            orch_rx.recv_timeout(Duration::from_millis(500)),
            Ok(PlanetToOrchestrator::KillPlanetResult { planet_id: id })
        );
        
        handle.join().unwrap();
    }
}
