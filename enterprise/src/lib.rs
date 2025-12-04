#[allow(unused_imports)]
use common_game::{
    components::{
        energy_cell::EnergyCell,
        planet::{Planet, PlanetAI, PlanetState, PlanetType},
        resource::*,
        rocket::Rocket,
        sunray::Sunray,
    },
    protocols::messages::{
        ExplorerToPlanet, OrchestratorToPlanet, PlanetToExplorer, PlanetToOrchestrator,
    },
};

use std::collections::HashSet;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::time::SystemTime;

pub struct EnterpriseAi {
    // for starting and stopping the planet ai
    running: bool, // how about parameters like prioritizing rockets construction(always or only when needed), ecc ....
}

impl PlanetAI for EnterpriseAi {
    fn start(&mut self, state: &PlanetState) {
        self.running = true;
        // can add some initialization logic here (we have planet state after all)
    }

    fn stop(&mut self, state: &PlanetState) {
        self.running = false;
    }

    fn handle_orchestrator_msg(
        &mut self,
        state: &mut PlanetState,
        generator: &Generator,
        combinator: &Combinator,
        msg: OrchestratorToPlanet,
    ) -> Option<PlanetToOrchestrator> {
        if !self.is_running() {
            return match msg {
                OrchestratorToPlanet::StartPlanetAI => {
                    self.start(state);
                    Some(PlanetToOrchestrator::StartPlanetAIResult {
                        planet_id: state.id(),
                    })
                }
                _ => None,
            };
        }

        return match msg {
            OrchestratorToPlanet::StartPlanetAI => {
                self.start(state);
                Some(PlanetToOrchestrator::StartPlanetAIResult {
                    planet_id: state.id(),
                })
            }
            OrchestratorToPlanet::Asteroid(asteroid) => {
                // if self.running{
                //     self.try_build_rocket(state);        // maybe handle the result?
                //     Some(PlanetToOrchestrator::AsteroidAck { planet_id: state.id(), rocket:state.take_rocket()})
                // }else{
                //     None
                // }
                //None        // <----- am i missing something or there is the handle_asteroid for this
                Some(PlanetToOrchestrator::AsteroidAck {
                    planet_id: state.id(),
                    rocket: self.handle_asteroid(state, generator, combinator),
                }) // to review
            }
            OrchestratorToPlanet::StopPlanetAI => {
                self.stop(state);
                Some(PlanetToOrchestrator::StopPlanetAIResult {
                    planet_id: state.id(),
                })
            }
            OrchestratorToPlanet::Sunray(sunray) => {
                self.charge_energy_cell(state, sunray);
                let _ = self.try_build_rocket(state); // handle result value
                Some(PlanetToOrchestrator::SunrayAck {
                    planet_id: state.id(),
                })
            }
            OrchestratorToPlanet::InternalStateRequest => {
                // let out=PlanetState::clone(state);
                // Some(PlanetToOrchestrator::InternalStateResponse { planet_id: state.id(), planet_state: PlanetState::clone(state), timestamp: SystemTime::now() })   //no clone trait for planetstate?????
                None
            }
            OrchestratorToPlanet::IncomingExplorerRequest {
                explorer_id,
                new_mpsc_sender,
            } => {
                Some(PlanetToOrchestrator::IncomingExplorerResponse {
                    planet_id: state.id(),
                    res: Ok(()),
                }) // ????
            }
            OrchestratorToPlanet::OutgoingExplorerRequest { explorer_id } => {
                Some(PlanetToOrchestrator::OutgoingExplorerResponse {
                    planet_id: state.id(),
                    res: Ok(()),
                })
            }
        };
        //}else{
        //None
        //}
    }

    fn handle_asteroid(
        &mut self,
        state: &mut PlanetState,
        generator: &Generator,
        combinator: &Combinator,
    ) -> Option<Rocket> {
        // match state.take_rocket(){
        // Some(rocket)=>Some(rocket),
        // None=>{
        //     self.try_build_rocket(state);
        //     state.take_rocket()
        // }
        // }

        if !self.is_running() {
            return None;
        }

        if let Some(rocket) = state.take_rocket() {
            return Some(rocket);
        }

        // if !state.can_have_rocket(){
        //     return None;
        // }

        // self.try_build_rocket
        if let Some((energy_cell, ix)) = state.full_cell() {
            match state.build_rocket(ix) {
                // try building an emergency rocket
                Ok(_) => state.take_rocket(),
                Err(err) => {
                    eprintln!("Failed to build anemergency rocket: {err}");
                    None
                }
            }
        } else {
            None
        }
    }

    fn handle_explorer_msg(
        &mut self,
        state: &mut PlanetState,
        generator: &Generator,
        combinator: &Combinator,
        msg: ExplorerToPlanet,
    ) -> Option<PlanetToExplorer> {
        if !self.is_running() {
            return None;
        }

        //if self.running{
        match msg {
            ExplorerToPlanet::AvailableEnergyCellRequest { .. } => {
                let available = state
                    .cells_iter()
                    .filter(|energy_cell| energy_cell.is_charged())
                    .count() as u32; // i know we need only to check if the first is charged
                Some(PlanetToExplorer::AvailableEnergyCellResponse {
                    available_cells: available,
                })
            }
            ExplorerToPlanet::CombineResourceRequest { explorer_id, msg } => {
                Some(PlanetToExplorer::CombineResourceResponse {
                    complex_response: self.handle_combine_request(msg, combinator, state),
                })
            }
            ExplorerToPlanet::GenerateResourceRequest {
                explorer_id,
                resource,
            } => {
                if !generator.contains(resource) {
                    return Some(PlanetToExplorer::GenerateResourceResponse { resource: None });
                } else {
                    if let Some((energy_cell, _)) = state.full_cell() {
                        let new_resource = match resource {
                            BasicResourceType::Carbon => generator
                                .make_carbon(energy_cell)
                                .map(|new_carbon| BasicResource::Carbon(new_carbon)),
                            _ => Err("Resource not supported".to_string()), // need this for compiler to shut up
                        };
                        // if let Err(err)=new_resource{
                        //     eprintln!("Could not generate resource: {err}");
                        // }
                        return Some(PlanetToExplorer::GenerateResourceResponse {
                            resource: new_resource.ok(),
                        });
                    } else {
                        return Some(PlanetToExplorer::GenerateResourceResponse { resource: None });
                    }
                }
            }
            // ExplorerToPlanet::InternalStateRequest { explorer_id }=>{
            //     Some(PlanetToExplorer::InternalStateResponse { planet_state: PlanetState::from(state) })
            // },
            ExplorerToPlanet::SupportedCombinationRequest { .. } => {
                // C type planets support unbounded combination rules (up to 6)
                // let rules= combinator.all_available_recipes();
                // Some(PlanetToExplorer::CombineResourceResponse {
                //     complex_response: if rules.is_empty() {None} else {Some(rules)}        // <---- read documentation
                // })
                Some(PlanetToExplorer::SupportedCombinationResponse {
                    combination_list: combinator.all_available_recipes(),
                })
            }
            ExplorerToPlanet::SupportedResourceRequest { .. } => {
                // C type planets support only one generation rule
                // let mut rules=HashSet::new();
                // if let Some(&rule) = generator.all_available_recipes().iter().next(){
                //     rules.insert(rule);
                // }
                // Some(PlanetToExplorer::SupportedResourceResponse {
                //     resource_list: if rules.is_empty() {None} else {Some(rules)}
                // })
                Some(PlanetToExplorer::SupportedResourceResponse {
                    resource_list: generator.all_available_recipes(),
                })
            }
        }
        // } else{
        //     None
        // }
    }
}

impl EnterpriseAi {
    pub fn new() -> Self {
        Self { running: false }
    }

    pub fn is_running(&self) -> bool {
        self.running
    }

    fn charge_energy_cell(&self, state: &mut PlanetState, sunray: Sunray) {
        // state.cell_mut(0).charge(sunray);   //has only one cell

        if let Some((energy_cell, _)) = state.empty_cell() {
            // <-- _ because e_cell is always at 0
            energy_cell.charge(sunray);
        }
        // ^^^ if it returns none the sunray is wasted
    }

    fn try_build_rocket(&self, state: &mut PlanetState) -> Result<(), String> {
        // if !state.has_rocket(){
        //     if state.cell(0).is_charged(){
        //         let cell=state.cell_mut(0);
        //         match state.build_rocket(cell){
        //             Ok(_)=>Ok(()),
        //             Err(str)=>Err(str)
        //         }
        //     }else{
        //         Err(String::from("Energy cell already depleted"))
        //     }
        // }else{
        //     Ok(())
        // }

        if !state.has_rocket() {
            if let Some((_, ix)) = state.full_cell() {
                state.build_rocket(ix)?;
            }
        }
        Ok(())
    }

    // fn has_charged_cells(&self,state:&PlanetState)->bool{
    //     state.cell(0).is_charged()
    // }

    pub fn deplete_charged_cell<'a>(&self, state: &'a mut PlanetState) -> Result<(), String> {
        // match state.cell_mut(0).discharge(){
        //     Ok(_)=>true,
        //     Err(_)=>false   // maybe handle error
        // }

        if let Some((energy_cell, _)) = state.full_cell() {
            energy_cell.discharge();
            Ok(())
        } else {
            Err("No charged energy cells available".to_string())
        }
    }

    fn handle_combine_request(
        &mut self,
        request: ComplexResourceRequest,
        combinator: &Combinator,
        state: &mut PlanetState,
    ) -> Option<ComplexResource> {
        let energy_cell = match state.full_cell() {
            Some((c, _)) => c,
            None => return None,
        };

        match request {
            ComplexResourceRequest::AIPartner(r1, r2) => combinator
                .make_aipartner(r1, r2, energy_cell)
                .ok()
                .map(|new_aipartner| ComplexResource::AIPartner(new_aipartner)),
            ComplexResourceRequest::Diamond(r1, r2) => combinator
                .make_diamond(r1, r2, energy_cell)
                .ok()
                .map(|new_diamond| ComplexResource::Diamond(new_diamond)),
            ComplexResourceRequest::Dolphin(r1, r2) => combinator
                .make_dolphin(r1, r2, energy_cell)
                .ok()
                .map(|new_dolphin| ComplexResource::Dolphin(new_dolphin)),
            ComplexResourceRequest::Life(r1, r2) => combinator
                .make_life(r1, r2, energy_cell)
                .ok()
                .map(|new_life| ComplexResource::Life(new_life)),
            ComplexResourceRequest::Robot(r1, r2) => combinator
                .make_robot(r1, r2, energy_cell)
                .ok()
                .map(|new_robot| ComplexResource::Robot(new_robot)),
            ComplexResourceRequest::Water(r1, r2) => combinator
                .make_water(r1, r2, energy_cell)
                .ok()
                .map(|new_water| ComplexResource::Water(new_water)),
        }
    }
}

pub fn create_planet(
    rx_orchestrator: Receiver<OrchestratorToPlanet>,
    tx_orchestrator: Sender<PlanetToOrchestrator>,
    rx_explorer: Receiver<ExplorerToPlanet>,
    tx_explorer: Sender<PlanetToExplorer>,
) -> Planet {
    let id = 67; // huhhhhhhhhhhhhhh....do we have to agree upon id values?
    let ai = Box::new(EnterpriseAi::new());
    let gen_rules = vec![BasicResourceType::Carbon];
    let comb_rules = vec![
        ComplexResourceType::Water,
        ComplexResourceType::Diamond,
        ComplexResourceType::Life,
        ComplexResourceType::Robot,
        ComplexResourceType::Dolphin,
        ComplexResourceType::AIPartner,
    ];

    match Planet::new(
        id,
        PlanetType::C,
        ai,
        gen_rules,
        comb_rules,
        (rx_orchestrator, tx_orchestrator),
        (rx_explorer, tx_explorer),
    ) {
        Ok(planet) => planet,
        Err(error) => panic!("{error}"), // need to handle properly error case
    }
}

#[cfg(test)]
mod tests {
    use common_game::{components::planet, protocols::messages};

    use super::*;

    #[test]
    fn is_one_equal_to_one() {
        assert_eq!(1, 1)
    }

    #[test]
    fn should_not_be_runnning_when_new() {
        assert!(!EnterpriseAi::new().is_running());
    }

    fn create_dummy_state() -> PlanetState {
        unimplemented!()
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
