mod ai;
mod tests;

// #[allow(unused_imports)]
// use common_game::{
//     components::{
//         energy_cell::EnergyCell,
//         planet::{Planet, PlanetAI, PlanetState, PlanetType},
//         resource::*,
//         rocket::Rocket,
//         sunray::Sunray,
//     },
//     protocols::messages::{
//         ExplorerToPlanet, OrchestratorToPlanet, PlanetToExplorer, PlanetToOrchestrator,
//     },
// };

// use std::collections::HashSet;
// use std::sync::mpsc::{Receiver, Sender, channel};
// use std::time::SystemTime;

// //Note: I am not really sure how to handle the Result<(), String> of many functions
// //Should we keep track of the errors somewhere?
// //Examples: build_rocket(), ...


// //AI of the planet Enterprise
// pub struct EnterpriseAi {
//     // This parameter the current state of the AI
//     running: bool,
//     // This parameter represents how many explorers are on the planet
//     num_explorers: u8,
// }

// impl PlanetAI for EnterpriseAi {
//     fn start(&mut self, state: &PlanetState) {
//         // Setting the planet parameters
//         self.running = true;
//         self.num_explorers = 0; //No explorers when the planet is created
//     }

//     fn stop(&mut self, state: &PlanetState) {
//         self.running = false;
//         self.num_explorers = 0; //They are all dead
//     }

//     fn handle_orchestrator_msg(
//         &mut self,
//         state: &mut PlanetState,
//         generator: &Generator,
//         combinator: &Combinator,
//         msg: OrchestratorToPlanet,
//     ) -> Option<PlanetToOrchestrator> {
//         if !self.is_running() {
//             return match msg {
//                 OrchestratorToPlanet::StartPlanetAI => {
//                     self.start(state);
//                     Some(PlanetToOrchestrator::StartPlanetAIResult {
//                         planet_id: state.id(),
//                     })
//                 }
//                 _ => None,
//             };
//         }

//         match msg {
//             OrchestratorToPlanet::StartPlanetAI => {
//                 self.start(state);
//                 Some(PlanetToOrchestrator::StartPlanetAIResult {
//                     planet_id: state.id(),
//                 })
//             }
//             OrchestratorToPlanet::Asteroid(asteroid) => {
//                 Some(PlanetToOrchestrator::AsteroidAck {
//                     planet_id: state.id(),
//                     destroyed: match self.handle_asteroid(state, generator, combinator){
//                         None => true,
//                         Some(rocket)=> true,
//                     },
//                 })
//             }
//             OrchestratorToPlanet::StopPlanetAI => {
//                 self.stop(state);
//                 Some(PlanetToOrchestrator::StopPlanetAIResult {
//                     planet_id: state.id(),
//                 })
//             }
//             OrchestratorToPlanet::Sunray(sunray) => {
//                 state.charge_cell(sunray);

//                 // If there are no explorers, the planet will prioritize self-defense
//                 // It will only try to build a rocket if it doesn't have any rocket
//                 //Otherwise, it will store the energy cell for the explorers
//                 if self.num_explorers == 0 {
//                     if self.has_charged_cells(state) && !state.has_rocket(){ //build_rocket does all the checks except if the energy cell is charged (even if the comment on the code says otherwise)
//                         state.build_rocket(0);
//                     }
//                 }

//                 //Return acknowledgement
//                 Some(PlanetToOrchestrator::SunrayAck {
//                     planet_id: state.id(),
//                 })
//             }
//             OrchestratorToPlanet::InternalStateRequest => {
//                 // Create a dummy struct containing an overview of the internal state of a planet.
//                 let out=PlanetState::to_dummy(state);
//                 Some(PlanetToOrchestrator::InternalStateResponse { planet_id: state.id(), planet_state: out})
//             }
//             OrchestratorToPlanet::IncomingExplorerRequest { explorer_id, new_mpsc_sender, } => {
//                 //Explorer coming to the planet, increase counter
//                 self.num_explorers += 1;

//                 Some(PlanetToOrchestrator::IncomingExplorerResponse {
//                     planet_id: state.id(),
//                     res: Ok(()), // This is a Result<(), String>, I didn't understand in which cases an explorer wouldn't be accepted
//                 })
//             }
//             OrchestratorToPlanet::OutgoingExplorerRequest { explorer_id } => {

//                 //Explorer exiting the planet, decrease counter
//                 self.num_explorers -= 1;

//                 Some(PlanetToOrchestrator::OutgoingExplorerResponse {
//                     planet_id: state.id(),
//                     res: Ok(()), // This is a Result<(), String>, I didn't understand in which cases an explorer wouldn't be allowed to go out
//                 })
//             }
//         }
//     }

//     fn handle_asteroid(
//         &mut self,
//         state: &mut PlanetState,
//         generator: &Generator,
//         combinator: &Combinator,
//     ) -> Option<Rocket> {
//         if !self.running {
//             return None;
//         }
//         //This function tries to take a rocket from the planet
//         //If there is no rocket, it tries to build one
//         //If this does not work, it returns None
//         match state.take_rocket() {
//             Some(rocket) => { Some(rocket) },
//             None => {
//                 if self.has_charged_cells(state){state.build_rocket(0);}
//                 state.take_rocket()
//             }
//         }
//     }

//     fn handle_explorer_msg(
//         &mut self,
//         state: &mut PlanetState,
//         generator: &Generator,
//         combinator: &Combinator,
//         msg: ExplorerToPlanet,
//     ) -> Option<PlanetToExplorer> {
//         if !self.is_running() {
//             return None;
//         }

//         match msg {
//             ExplorerToPlanet::AvailableEnergyCellRequest { .. } => {
//                 //Counts how many energy cells are charged (1 or 0 in our case)
//                 let available = state
//                     .cells_iter()
//                     .filter(|energy_cell| energy_cell.is_charged())
//                     .count() as u32; // i know we need only to check if the first is charged - Ok, that is more complete
//                 Some(PlanetToExplorer::AvailableEnergyCellResponse {
//                     available_cells: available,
//                 })
//             }
//             ExplorerToPlanet::CombineResourceRequest { explorer_id, msg } => {
//                 Some(PlanetToExplorer::CombineResourceResponse {
//                     complex_response: self.handle_combine_request(msg, combinator, state),
//                 })
//             }
//             ExplorerToPlanet::GenerateResourceRequest { explorer_id, resource} => {
//                 Some(PlanetToExplorer::GenerateResourceResponse {
//                     resource: self.handle_resource_request(resource, generator, state)
//                 })
//             }
//             ExplorerToPlanet::SupportedCombinationRequest { .. } => {
//                 // C type planets support unbounded combination rules (up to 6)
//                 Some(PlanetToExplorer::SupportedCombinationResponse {
//                     combination_list: combinator.all_available_recipes(),
//                 })
//             }
//             ExplorerToPlanet::SupportedResourceRequest { .. } => {
//                 // C type planets support only one generation rule
//                 Some(PlanetToExplorer::SupportedResourceResponse {
//                     resource_list: generator.all_available_recipes(),
//                 })
//             }
//         }
//     }
// }

// //Note: I deleted charge_energy_cell and try_build_rocket.
// //There were already functions that did the exact same thing, same checks...
// //If you disagree, we can discuss it and maybe implement it again
// impl EnterpriseAi {
//     pub fn new() -> Self {
//         Self { running: false, num_explorers: 0 }
//     }
//     pub fn is_running(&self) -> bool {
//         self.running
//     }
//     fn has_charged_cells(&self,state:&PlanetState)->bool{
//         //Enterprise (planet of type C) support only 1 energy cell
//         state.cell(0).is_charged()
//     }
// // Do we need this function?
//     // pub fn deplete_charged_cell(&self, state: &mut PlanetState) -> Result<(), String> {
//     //     // match state.cell_mut(0).discharge(){
//     //     //     Ok(_)=>true,
//     //     //     Err(_)=>false   // maybe handle error
//     //     // }
//     //
//     //     if let Some((energy_cell, _)) = state.full_cell() {
//     //         energy_cell.discharge();
//     //         Ok(())
//     //     } else {
//     //         Err("No charged energy cells available".to_string())
//     //     }
//     // }

//     fn handle_resource_request(
//         &mut self,
//         request: BasicResourceType,
//         generator: &Generator,
//         state: &mut PlanetState,
//     ) -> Option<BasicResource>{
//         if !generator.contains(request) {
//             return None;
//         }
//         else {

//             let energy_cell = match state.full_cell() {
//                 Some((c, _)) => c,
//                 None => return None,
//             };

//             let new_resource = generator
//                 .make_carbon(energy_cell)
//                 .map(|new_carbon| BasicResource::Carbon(new_carbon));

//             match new_resource {
//                 Ok(new_resource) => {return Some(new_resource)},
//                 Err(_) => {}, //Handle error?
//             }

//             // Do we need to match the request?
//             // Because if generator contains request, we know it is carbon.
//             // Or should we keep the version with the match for extra safety?

//             // if let Some((energy_cell, _)) = state.full_cell() {
//             //     let new_resource = match request {
//             //         BasicResourceType::Carbon => generator
//             //             .make_carbon(energy_cell)
//             //             .map(|new_carbon| BasicResource::Carbon(new_carbon)),
//             //         _ => Err("Resource not supported".to_string()), // need this for compiler to shut up
//             //     };
//             //
//             //
//             //     match new_resource {
//             //         Ok(new_resource) => {return Some(new_resource)},
//             //         Err(_) => {}, //Handle error?
//             //     }
//             // }
//         }
//         None
//     }

//     fn handle_combine_request(
//         &mut self,
//         request: ComplexResourceRequest,
//         combinator: &Combinator,
//         state: &mut PlanetState,
//     ) -> Result<ComplexResource, (String, GenericResource, GenericResource)> {

//         // I didn't manage to find a way to do the energy cell check outside because of r1 and r2
//         // Maybe a function would be better?

//         // let energy_cell = match state.full_cell() {
//         //     Some((c, _)) => c,
//         //     None => { return Err("No energy cell available".to_string(), r1, r2)},
//         // };

//         // This became quite big... Maybe there is a better way to do it

//         match request {
//             ComplexResourceRequest::AIPartner(r1, r2) => {
//                 let energy_cell = match state.full_cell() {
//                     Some((c, _)) => c,
//                     None => { return Err(("No energy cell available".to_string(),
//                                           GenericResource::ComplexResources(ComplexResource::Robot(r1)),
//                                           GenericResource::ComplexResources(ComplexResource::Diamond(r2)))) },
//                 };

//                 let complex = combinator.make_aipartner(r1, r2, energy_cell);

//                 match complex {
//                     Ok(complex) => {Ok(ComplexResource::AIPartner(complex))},
//                     Err((s, r1, r2)) => {Err((s,
//                                               GenericResource::ComplexResources(ComplexResource::Robot(r1)),
//                                               GenericResource::ComplexResources(ComplexResource::Diamond(r2))))}
//                 }


//             },
//             ComplexResourceRequest::Diamond(r1, r2) =>  {
//                 let energy_cell = match state.full_cell() {
//                     Some((c, _)) => c,
//                     None => { return Err(("No energy cell available".to_string(),
//                                           GenericResource::BasicResources(BasicResource::Carbon(r1)),
//                                           GenericResource::BasicResources(BasicResource::Carbon(r2)))) },
//                 };

//                 let complex = combinator.make_diamond(r1, r2, energy_cell);

//                 match complex {
//                     Ok(complex) => {Ok(ComplexResource::Diamond(complex))},
//                     Err((s, r1, r2)) => {Err((s,
//                                               GenericResource::BasicResources(BasicResource::Carbon(r1)),
//                                               GenericResource::BasicResources(BasicResource::Carbon(r2))))}
//                 }


//             },
//             ComplexResourceRequest::Dolphin(r1, r2) => {
//                 let energy_cell = match state.full_cell() {
//                     Some((c, _)) => c,
//                     None => { return Err(("No energy cell available".to_string(),
//                                           GenericResource::ComplexResources(ComplexResource::Water(r1)),
//                                           GenericResource::ComplexResources(ComplexResource::Life(r2)))) },
//                 };

//                 let complex = combinator.make_dolphin(r1, r2, energy_cell);

//                 match complex {
//                     Ok(complex) => {Ok(ComplexResource::Dolphin(complex))},
//                     Err((s, r1, r2)) => {Err((s,
//                                               GenericResource::ComplexResources(ComplexResource::Water(r1)),
//                                               GenericResource::ComplexResources(ComplexResource::Life(r2))))}
//                 }
//             },
//             ComplexResourceRequest::Life(r1, r2) => {
//                 let energy_cell = match state.full_cell() {
//                     Some((c, _)) => c,
//                     None => { return Err(("No energy cell available".to_string(),
//                                           GenericResource::ComplexResources(ComplexResource::Water(r1)),
//                                           GenericResource::BasicResources(BasicResource::Carbon(r2)))) },
//                 };

//                 let complex = combinator.make_life(r1, r2, energy_cell);

//                 match complex {
//                     Ok(complex) => {Ok(ComplexResource::Life(complex))},
//                     Err((s, r1, r2)) => {Err((s,
//                                               GenericResource::ComplexResources(ComplexResource::Water(r1)),
//                                               GenericResource::BasicResources(BasicResource::Carbon(r2))))}
//                 }
//             },
//             ComplexResourceRequest::Robot(r1, r2) => {
//                 let energy_cell = match state.full_cell() {
//                     Some((c, _)) => c,
//                     None => { return Err(("No energy cell available".to_string(),
//                                           GenericResource::BasicResources(BasicResource::Silicon(r1)),
//                                           GenericResource::ComplexResources(ComplexResource::Life(r2)))) },
//                 };

//                 let complex = combinator.make_robot(r1, r2, energy_cell);

//                 match complex {
//                     Ok(complex) => {Ok(ComplexResource::Robot(complex))},
//                     Err((s, r1, r2)) => {Err((s,
//                                               GenericResource::BasicResources(BasicResource::Silicon(r1)),
//                                               GenericResource::ComplexResources(ComplexResource::Life(r2))))}
//                 }
//             },
//             ComplexResourceRequest::Water(r1, r2) => {
//                 let energy_cell = match state.full_cell() {
//                     Some((c, _)) => c,
//                     None => { return Err(("No energy cell available".to_string(),
//                                           GenericResource::BasicResources(BasicResource::Hydrogen(r1)),
//                                           GenericResource::BasicResources(BasicResource::Oxygen(r2)))) },
//                 };

//                 let complex = combinator.make_water(r1, r2, energy_cell);

//                 match complex {
//                     Ok(complex) => {Ok(ComplexResource::Water(complex))},
//                     Err((s, r1, r2)) => {Err((s,
//                                               GenericResource::BasicResources(BasicResource::Hydrogen(r1)),
//                                               GenericResource::BasicResources(BasicResource::Oxygen(r2))))}
//                 }
//             },

//         }
//     }
// }

// pub fn create_planet(
//     id: u8,
//     rx_orchestrator: Receiver<OrchestratorToPlanet>,
//     tx_orchestrator: Sender<PlanetToOrchestrator>,
//     rx_explorer: Receiver<ExplorerToPlanet>,
//     tx_explorer: Sender<PlanetToExplorer>,
// ) -> Planet {
//     //let id = 67; // huhhhhhhhhhhhhhh....do we have to agree upon id values? -> We need to ask this, we cannot have planets with the same id
//     let ai = Box::new(EnterpriseAi::new());
//     let gen_rules = vec![BasicResourceType::Carbon];
//     let comb_rules = vec![
//         ComplexResourceType::Water,
//         ComplexResourceType::Diamond,
//         ComplexResourceType::Life,
//         ComplexResourceType::Robot,
//         ComplexResourceType::Dolphin,
//         ComplexResourceType::AIPartner,
//     ];

//     match Planet::new(
//         id,
//         PlanetType::C,
//         ai,
//         gen_rules,
//         comb_rules,
//         (rx_orchestrator, tx_orchestrator),
//         rx_explorer,
//     ) {
//         Ok(planet) => planet,
//         Err(error) => panic!("{error}"), // need to handle properly error case
//     }
// }