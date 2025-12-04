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

//Note: I am not really sure how to handle the Result<(), String> of many functions
//Should we keep track of the errors somewhere?
//Examples: build_rocket(), ...


//AI of the planet Enterprise
pub struct EnterpriseAi {
    // This parameter the current state of the AI
    running: bool,
    // This parameter represents how many explorers are on the planet
    num_explorers: u8,
}

impl PlanetAI for EnterpriseAi {
    fn start(&mut self, state: &PlanetState) {
        // Setting the planet parameters
        self.running = true;
        self.num_explorers = 0; //No explorers when the planet is created
    }

    fn stop(&mut self, state: &PlanetState) {
        self.running = false;
        self.num_explorers = 0; //They are all dead
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
                Some(PlanetToOrchestrator::AsteroidAck {
                    planet_id: state.id(),
                    rocket: self.handle_asteroid(state, generator, combinator),
                }) // to review -> I think it is okay, all the logic is done in handle_asteroid
            }
            OrchestratorToPlanet::StopPlanetAI => {
                self.stop(state);
                Some(PlanetToOrchestrator::StopPlanetAIResult {
                    planet_id: state.id(),
                })
            }
            OrchestratorToPlanet::Sunray(sunray) => {
                state.charge_cell(sunray);

                // If there are no explorers, the planet will prioritize self-defense
                //Otherwise, it will store the energy cell for the explorers
                if self.num_explorers == 0 {
                    if self.has_charged_cells(state){ //build_rocket does all the checks except if the energy cell is charged (even if the comment on the code says otherwise)
                        state.build_rocket(0);
                    }
                }

                //Return acknowledgement
                Some(PlanetToOrchestrator::SunrayAck {
                    planet_id: state.id(),
                })
            }
            OrchestratorToPlanet::InternalStateRequest => {
                //Problem: No copy/clone trait for PlanetState
                //There is already an issue about it on GitHub

                //let out=PlanetState::clone(state);
                //Some(PlanetToOrchestrator::InternalStateResponse { planet_id: state.id(), planet_state: PlanetState::clone(state)})
                None
            }
            OrchestratorToPlanet::IncomingExplorerRequest {
                explorer_id,
                new_mpsc_sender,
            } => {

                //Explorer coming to the planet, increase counter
                self.num_explorers += 1;

                Some(PlanetToOrchestrator::IncomingExplorerResponse {
                    planet_id: state.id(),
                    res: Ok(()), // This is a Result<(), String>, I didn't understand in which cases an explorer wouldn't be accepted
                })
            }
            OrchestratorToPlanet::OutgoingExplorerRequest { explorer_id } => {

                //Explorer exiting the planet, decrease counter
                self.num_explorers -= 1;

                Some(PlanetToOrchestrator::OutgoingExplorerResponse {
                    planet_id: state.id(),
                    res: Ok(()), // This is a Result<(), String>, I didn't understand in which cases an explorer wouldn't be allowed to go out
                })
            }
        };
    }

    fn handle_asteroid(
        &mut self,
        state: &mut PlanetState,
        generator: &Generator,
        combinator: &Combinator,
    ) -> Option<Rocket> {
        if !self.is_running() { return None; }

        //This function tries to take a rocket from the planet
        //If there is no rocket, it tries to build one
        //If this does not work, it returns None

        match state.take_rocket() {
            Some(rocket) => { return Some(rocket) },
            None => {
                if self.has_charged_cells(state){state.build_rocket(0);}
                return state.take_rocket();
            }
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

        match msg {
            ExplorerToPlanet::AvailableEnergyCellRequest { .. } => {
                //Counts how many energy cells are charged (1 or 0 in our case)
                let available = state
                    .cells_iter()
                    .filter(|energy_cell| energy_cell.is_charged())
                    .count() as u32; // i know we need only to check if the first is charged - Ok, that is more complete
                Some(PlanetToExplorer::AvailableEnergyCellResponse {
                    available_cells: available,
                })
            }
            ExplorerToPlanet::CombineResourceRequest { explorer_id, msg } => {
                Some(PlanetToExplorer::CombineResourceResponse {
                    complex_response: self.handle_combine_request(msg, combinator, state),
                })
            }
            ExplorerToPlanet::GenerateResourceRequest { explorer_id, resource} => {
                //Should we call a function like in the previous match (clearer code)
                // Some(PlanetToExplorer::GenerateResourceResponse {
                //     resource: self.handle_resource_request(msg, generator, state)
                // })

                //I also think that we could avoid that unnecessary check

                if !generator.contains(resource) {
                    return Some(PlanetToExplorer::GenerateResourceResponse { resource: None });
                }
                else {
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
            // Was this message removed in the recent versions? I can still see it in the common code
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
    }
}

//Note: I deleted charge_energy_cell and try_build_rocket.
//There were already functions that did the exact same thing, same checks...
//If you disagree, we can discuss it and maybe implement it again
impl EnterpriseAi {
    pub fn new() -> Self {
        Self { running: false, num_explorers: 0 }
    }
    pub fn is_running(&self) -> bool {
        self.running
    }
    fn has_charged_cells(&self,state:&PlanetState)->bool{
        //Enterprise (planet of type C) support only 1 energy cell
        state.cell(0).is_charged()
    }

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
    let id = 67; // huhhhhhhhhhhhhhh....do we have to agree upon id values? -> We need to ask this, we cannot have planets with the same id
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


//Implement more test to show during the fair
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
