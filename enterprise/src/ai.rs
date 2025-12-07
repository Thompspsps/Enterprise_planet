#[allow(unused_imports)]
use common_game::{
    components::{
        energy_cell::EnergyCell,
        planet::{Planet, PlanetAI, PlanetState, PlanetType},
        resource::*,
        rocket::Rocket,
        sunray::Sunray,
    },
    logging::*,
    protocols::messages::{
        ExplorerToPlanet, OrchestratorToPlanet, PlanetToExplorer, PlanetToOrchestrator,
    },
};

use crossbeam_channel::{Receiver, Sender};

// The Enterprise planet AI
pub struct EnterpriseAi {
    running: bool,     // This parameter represents the current state of the AI
    num_explorers: u8, // This parameter represents how many explorers are on the planet
    planet_id: u32,    // This parameter represents the planet ID (used for logging purposes)
}

impl PlanetAI for EnterpriseAi {
    fn start(&mut self, state: &PlanetState) {
        self.running = true; // Flags the parameter to true, the planet is active
        self.num_explorers = 0; // There are no explorers when the planet is created

        // info!("[Planet - {}] AI started",self.planet_id);
        let payload = Payload::from([
            ("action".to_string(), "start".to_string()),
            ("explorer_count".to_string(), self.num_explorers.to_string()),
        ]);

        LogEvent::new(
            ActorType::Planet,
            self.planet_id,
            ActorType::SelfActor,
            "self".to_string(),
            EventType::InternalPlanetAction,
            Channel::Info,
            payload,
        )
        .emit();
    }

    fn stop(&mut self, state: &PlanetState) {
        self.running = false; // Flags the parameter to false, the planet is stopped
        self.num_explorers = 0; // The number of explorers is brought to zero
        // info!("[Planet - {}] AI stopped",self.planet_id);

        let payload = Payload::from([("action".to_string(), "stopped".to_string())]);

        LogEvent::new(
            ActorType::Planet,
            self.planet_id,
            ActorType::SelfActor,
            "self".to_string(),
            EventType::InternalPlanetAction,
            Channel::Info,
            payload,
        )
        .emit();
    }

    fn handle_orchestrator_msg(
        &mut self,
        state: &mut PlanetState,
        generator: &Generator,
        combinator: &Combinator,
        msg: OrchestratorToPlanet,
    ) -> Option<PlanetToOrchestrator> {
        
        if !self.is_running() && !matches!(msg, OrchestratorToPlanet::StartPlanetAI) {
            // Matches returns whether the given expression matches the provided pattern
            // warn!("[Planet - {}] AI received message while stopped",self.planet_id);
            // The msg does not implement Debug trait, so it can't be printed

            let payload = Payload::from([
                (
                    "message_type".to_string(),
                    "orchestrator_to_planet:start_planet_ai".to_string(),
                ),
                ("state".to_string(), "not_yet_stopped".to_string()), // or stopped, whatever you feel is more suitable
            ]);

            LogEvent::new(
                ActorType::Planet,
                self.planet_id,
                ActorType::Orchestrator,
                "orchestrator".to_string(),
                EventType::MessageOrchestratorToPlanet,
                Channel::Warning,
                payload,
            )
            .emit();

            return None;
        }
        

        match msg {
            OrchestratorToPlanet::StartPlanetAI => {
                // Only start the planet if it wasn't already running.
                // The explorers counter is set to 0, even if there were some
                if !self.running {
                    self.start(state);
                }

                let payload = Payload::from([("action".to_string(), "start_ack".to_string())]);

                LogEvent::new(
                    ActorType::Planet,
                    self.planet_id,
                    ActorType::Orchestrator,
                    "orchestrator".to_string(),
                    EventType::MessagePlanetToOrchestrator,
                    Channel::Info,
                    payload,
                )
                .emit();

                Some(PlanetToOrchestrator::StartPlanetAIResult {
                    planet_id: state.id(),
                })
            }

            OrchestratorToPlanet::Asteroid(asteroid) => {
                let mut payload = Payload::from([
                    ("action".to_string(), "asteroid_ack".to_string()),
                    ("has_rocket".to_string(), state.has_rocket().to_string()),
                    (
                        "has_charged_cell".to_string(),
                        self.has_charged_cells(state).to_string(),
                    ),
                ]);

                // The handle_rocket returns a rocket if the planet can defend itself - check logic in the function
                let out_rocket = self.handle_asteroid(state, generator, combinator);

                match out_rocket {
                    None => {
                        payload.insert("has_built_rocket".to_string(), false.to_string());
                        true // so they are the same then? -> What are those true for?
                    }
                    Some(_) => {
                        payload.insert("has_built_rocket".to_string(), true.to_string());
                        true // <-------  -> ?
                    }
                };

                // payload.insert("was_destroyed".to_string(),destroyed.to_string());

                LogEvent::new(
                    ActorType::Planet,
                    self.planet_id,
                    ActorType::Orchestrator,
                    "orchestrator".to_string(),
                    EventType::MessagePlanetToOrchestrator,
                    Channel::Info,
                    payload,
                )
                .emit();

                Some(PlanetToOrchestrator::AsteroidAck {
                    planet_id: state.id(),
                    rocket: out_rocket,
                })
            }
            OrchestratorToPlanet::StopPlanetAI => {
                self.stop(state); //The orchestrator stops the planet AI

                let payload = Payload::from([("action".to_string(), "stop_ack".to_string())]);

                LogEvent::new(
                    ActorType::Planet,
                    self.planet_id,
                    ActorType::Orchestrator,
                    "orchestrator".to_string(),
                    EventType::MessageOrchestratorToPlanet,
                    Channel::Info,
                    payload,
                )
                .emit();

                Some(PlanetToOrchestrator::StopPlanetAIResult {
                    planet_id: state.id(),
                })
            }
            OrchestratorToPlanet::Sunray(sunray) => {
                // If there is already a charged cell, the planet will always try to build a rocket
                // If there's already a rocket, the sunray is wasted.
                // If the cell is not charged, we charge it
                // Then, we have other two possibilities: no explorers and explorers
                // If there are no explorers, the planet will prioritize self-defense
                // It will only try to build a rocket if it doesn't have any rocket
                // If there are explorers, it will store the energy cell for the explorers

                let payload = Payload::from([("action".to_string(), "sunray_ack".to_string())]);

                LogEvent::new(
                    ActorType::Planet,
                    self.planet_id,
                    ActorType::Orchestrator,
                    "orchestrator".to_string(),
                    EventType::MessageOrchestratorToPlanet,
                    Channel::Info,
                    payload,
                )
                .emit();

                let mut payload = Payload::from([
                    (
                        "visiting_explorers".to_string(),
                        self.num_explorers.to_string(),
                    ),
                    ("has_rocket".to_string(), state.has_rocket().to_string()),
                ]);

                // Here, if there are charged cells and no rockets, it uses the current cell to build the rocket
                // Add logging also here?
                if (self.has_charged_cells(state)) && !state.has_rocket() {
                    state.build_rocket(0); // Unused result for logging
                }

                // Then, the sunray is wasted only if there's a charged cell and there was already a rocket
                let wasted_sunray = match state.charge_cell(sunray) {
                    Some(sunray) => {
                        payload.insert("wasted_sunray".to_string(), true.to_string()); // or rather charged_cell ???
                        Some(sunray)
                    }
                    None => {
                        payload.insert("wasted_sunray".to_string(), false.to_string());
                        None
                    }
                };

                // We can move the logic below into the match and add more logs ^^^^^^^

                // This happens only when there is no explorer
                // If the sunray can be used to charge a cell, then we can build a rocket and then charge the depleted cell
                if self.num_explorers == 0 {
                    if let Some((_, at)) = state.full_cell() {
                        if let Ok(_) = state.build_rocket(at) {
                            payload.insert("has_built_rocket".to_string(), true.to_string());
                            if let Some(sunray) = wasted_sunray {
                                payload.insert("has_charged_cell".to_string(), true.to_string());
                                state.charge_cell(sunray);
                            }
                        }
                    }
                }

                // Add logging also for the elses?

                LogEvent::new(
                    ActorType::Planet,
                    self.planet_id,
                    ActorType::SelfActor,
                    "self".to_string(),
                    EventType::InternalPlanetAction,
                    Channel::Debug,
                    payload,
                )
                .emit();

                // Return acknowledgement
                Some(PlanetToOrchestrator::SunrayAck {
                    planet_id: state.id(),
                })
            }
            OrchestratorToPlanet::InternalStateRequest => {
                let payload =
                    Payload::from([("request".to_string(), "internal_state".to_string())]);

                LogEvent::new(
                    ActorType::Planet,
                    self.planet_id,
                    ActorType::Orchestrator,
                    "orchestrator".to_string(),
                    EventType::MessageOrchestratorToPlanet, // Still have to decide: log from orch(MessageOrchestratorToPlanet) and pl(MessagePlanetToOrchestrator) ?
                    Channel::Info,
                    payload,
                )
                .emit();

                // Create a dummy struct containing an overview of the internal state of a planet.
                let dummy_state = PlanetState::to_dummy(state);
                Some(PlanetToOrchestrator::InternalStateResponse {
                    planet_id: state.id(),
                    planet_state: dummy_state,
                })
            }
            OrchestratorToPlanet::IncomingExplorerRequest {
                explorer_id,
                new_mpsc_sender,
            } => {
                // not called
                self.num_explorers += 1; // The number of explorers inside the planet is increased, the explorer is coming

                let payload =
                    Payload::from([("action".to_string(), "explorer_arrival".to_string())]);

                LogEvent::new(
                    ActorType::Planet,
                    self.planet_id,
                    ActorType::Orchestrator,
                    "orchestrator".to_string(),
                    EventType::MessageOrchestratorToPlanet,
                    Channel::Info,
                    payload,
                )
                .emit();

                let payload = Payload::from([
                    ("in_explorer_id".to_string(), explorer_id.to_string()),
                    (
                        "visiting_explorers".to_string(),
                        self.num_explorers.to_string(),
                    ),
                ]);

                LogEvent::new(
                    ActorType::Planet,
                    self.planet_id,
                    ActorType::SelfActor,
                    "self".to_string(),
                    EventType::InternalExplorerAction,
                    Channel::Debug,
                    payload,
                )
                .emit();

                Some(PlanetToOrchestrator::IncomingExplorerResponse {
                    planet_id: state.id(),
                    res: Ok(()), // This is a Result<(), String>, I didn't understand in which cases an explorer wouldn't be accepted
                })
            }
            OrchestratorToPlanet::OutgoingExplorerRequest { explorer_id } => {
                // not called
                let mut res = Ok(());
                if self.num_explorers > 0 {
                    self.num_explorers -= 1; // The number of explorers inside the planet is decreased, the explorer is leaving
                } else {
                    res = Err("No explorer has arrived".to_string());

                    let payload = Payload::from([(
                        "cause".to_string(),
                        "no_explorer_arrived_yet".to_string(),
                    )]);

                    LogEvent::new(
                        ActorType::Planet,
                        self.planet_id,
                        ActorType::SelfActor,
                        "self".to_string(),
                        EventType::InternalExplorerAction,
                        Channel::Debug,
                        payload,
                    )
                    .emit();
                }

                let payload =
                    Payload::from([("action".to_string(), "explorer_departure".to_string())]);

                LogEvent::new(
                    ActorType::Planet,
                    self.planet_id,
                    ActorType::Orchestrator,
                    "orchestrator".to_string(),
                    EventType::MessageOrchestratorToPlanet,
                    Channel::Info,
                    payload,
                )
                .emit();

                let payload = Payload::from([
                    ("out_explorer_id".to_string(), explorer_id.to_string()),
                    (
                        "visiting_explorers".to_string(),
                        self.num_explorers.to_string(),
                    ),
                ]);

                LogEvent::new(
                    ActorType::Planet,
                    self.planet_id,
                    ActorType::SelfActor,
                    "self".to_string(),
                    EventType::InternalExplorerAction,
                    Channel::Debug,
                    payload,
                )
                .emit();

                Some(PlanetToOrchestrator::OutgoingExplorerResponse {
                    planet_id: state.id(),
                    res: res, // Error if there is a negative number of explorers in the planet, otherwise Ok
                })
            }
            OrchestratorToPlanet::KillPlanet => Some(PlanetToOrchestrator::KillPlanetResult {
                planet_id: state.id(),
            }),
        }
    }

    fn handle_asteroid(
        &mut self,
        state: &mut PlanetState,
        generator: &Generator,
        combinator: &Combinator,
    ) -> Option<Rocket> {
        if !self.is_running() {
            return None;
        }

        // This function tries to take a rocket from the planet
        // If there is no rocket, it tries to build one
        // If this does not work, it returns None
        match state.take_rocket() {
            Some(rocket) => Some(rocket),
            None => {
                if let Some((_, at)) = state.full_cell() {
                    state.build_rocket(at);
                } // Unused result for logging?
                state.take_rocket()
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
         let payload = Payload::from([
            ("error".to_string(), "ai_not_running".to_string()),
            ("explorer_id".to_string(), explorer_id.to_string()),
        ]);
        LogEvent::new(
            ActorType::Planet,
            self.planet_id,
            ActorType::Explorer,
            explorer_id.to_string(),
            EventType::MessageExplorerToPlanet,
            Channel::Warning,
            payload,
        ).emit();
        return None;
    }

        match msg {
            ExplorerToPlanet::AvailableEnergyCellRequest { .. } => {
                // Counts how many energy cells are currently charged (1 or 0 in C-type planet case)
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
            ExplorerToPlanet::GenerateResourceRequest {
                explorer_id,
                resource,
            } => Some(PlanetToExplorer::GenerateResourceResponse {
                resource: self.handle_resource_request(resource, generator, state),
            }),
            ExplorerToPlanet::SupportedCombinationRequest { .. } => {
                // C-type planets support unbounded combination rules (up to 6)
                Some(PlanetToExplorer::SupportedCombinationResponse {
                    combination_list: combinator.all_available_recipes(),
                })
            }
            ExplorerToPlanet::SupportedResourceRequest { .. } => {
                // C-type planets support only one generation rule
                Some(PlanetToExplorer::SupportedResourceResponse {
                    resource_list: generator.all_available_recipes(),
                })
            }
        }
    }
}

// Note: I deleted charge_energy_cell and try_build_rocket.
// There were already functions that did the exact same thing, same checks...
// If you disagree, we can discuss it and maybe implement it again
impl EnterpriseAi {
    pub fn new(planet_id: u32) -> Self {
        let payload = Payload::from([
            ("action".to_string(), "init".to_string()),
            ("planet_type".to_string(), "C".to_string()),
        ]);

        LogEvent::new(
            ActorType::Orchestrator,
            LogEvent::id_from_str("orchestrator"),
            ActorType::Planet,
            planet_id.to_string(),
            EventType::InternalOrchestratorAction,
            Channel::Info,
            payload,
        )
        .emit();

        Self {
            running: false,
            num_explorers: 0,
            planet_id,
        }
    }
    pub fn is_running(&self) -> bool {
        let payload = Payload::from([("is_running".to_string(), self.running.to_string())]);

        LogEvent::new(
            ActorType::Planet,
            self.planet_id,
            ActorType::SelfActor,
            "self".to_string(),
            EventType::InternalPlanetAction,
            Channel::Debug,
            payload,
        )
        .emit();

        self.running
    }

    fn has_charged_cells(&self, state: &mut PlanetState) -> bool {
        //Enterprise (planet of type C) support only 1 energy cell
        state.full_cell().is_some()
    }

    fn handle_resource_request(
        &mut self,
        request: BasicResourceType,
        generator: &Generator,
        state: &mut PlanetState,
    ) -> Option<BasicResource> {
        if !generator.contains(request) {
            return None;
        } else {
            let energy_cell = match state.full_cell() {
                Some((c, _)) => c,
                None => return None,
            };

            let new_resource = generator
                .make_carbon(energy_cell)
                .map(|new_carbon| BasicResource::Carbon(new_carbon));

            match new_resource {
                Ok(new_resource) => return Some(new_resource),
                Err(_) => {
                    let payload = Payload::from([]); //Where is the logging for this
                }
            }

            // Do we need to match the request?
            // Because if generator contains request, we know it is carbon.
            // Or should we keep the version with the match for extra safety?

            // if let Some((energy_cell, _)) = state.full_cell() {
            //     let new_resource = match request {
            //         BasicResourceType::Carbon => generator
            //             .make_carbon(energy_cell)
            //             .map(|new_carbon| BasicResource::Carbon(new_carbon)),
            //         _ => Err("Resource not supported".to_string()), // need this for compiler to shut up
            //     };
            //
            //
            //     match new_resource {
            //         Ok(new_resource) => {return Some(new_resource)},
            //         Err(_) => {}, //Handle error?
            //     }
            // }
        }
        None
    }

    fn handle_combine_request(
        &mut self,
        request: ComplexResourceRequest,
        combinator: &Combinator,
        state: &mut PlanetState,
    ) -> Result<ComplexResource, (String, GenericResource, GenericResource)> {
        //Add logging?

        // I didn't manage to find a way to do the energy cell check outside because of r1 and r2
        // Maybe a function would be better?

        // let energy_cell = match state.full_cell() {
        //     Some((c, _)) => c,
        //     None => { return Err("No energy cell available".to_string(), r1, r2)},
        // };

        // This became quite big... Maybe there is a better way to do it

        match request {
            // The "AIPartner" complex resource takes Robot + Diamond
            ComplexResourceRequest::AIPartner(r1, r2) => {
                let energy_cell = match state.full_cell() {
                    Some((c, _)) => c,
                    None => {
                        return Err((
                            "No energy cell available".to_string(),
                            GenericResource::ComplexResources(ComplexResource::Robot(r1)),
                            GenericResource::ComplexResources(ComplexResource::Diamond(r2)),
                        ));
                    }
                };

                let complex = combinator.make_aipartner(r1, r2, energy_cell);

                match complex {
                    Ok(complex) => Ok(ComplexResource::AIPartner(complex)),
                    Err((s, r1, r2)) => Err((
                        s,
                        GenericResource::ComplexResources(ComplexResource::Robot(r1)),
                        GenericResource::ComplexResources(ComplexResource::Diamond(r2)),
                    )),
                }
            }
            // The "Diamond" complex resource takes Carbon + Carbon
            ComplexResourceRequest::Diamond(r1, r2) => {
                let energy_cell = match state.full_cell() {
                    Some((c, _)) => c,
                    None => {
                        return Err((
                            "No energy cell available".to_string(),
                            GenericResource::BasicResources(BasicResource::Carbon(r1)),
                            GenericResource::BasicResources(BasicResource::Carbon(r2)),
                        ));
                    }
                };

                let complex = combinator.make_diamond(r1, r2, energy_cell);

                match complex {
                    Ok(complex) => Ok(ComplexResource::Diamond(complex)),
                    Err((s, r1, r2)) => Err((
                        s,
                        GenericResource::BasicResources(BasicResource::Carbon(r1)),
                        GenericResource::BasicResources(BasicResource::Carbon(r2)),
                    )),
                }
            }
            // The "Dolphin" complex resource takes Water + Life
            ComplexResourceRequest::Dolphin(r1, r2) => {
                let energy_cell = match state.full_cell() {
                    Some((c, _)) => c,
                    None => {
                        return Err((
                            "No energy cell available".to_string(),
                            GenericResource::ComplexResources(ComplexResource::Water(r1)),
                            GenericResource::ComplexResources(ComplexResource::Life(r2)),
                        ));
                    }
                };

                let complex = combinator.make_dolphin(r1, r2, energy_cell);

                match complex {
                    Ok(complex) => Ok(ComplexResource::Dolphin(complex)),
                    Err((s, r1, r2)) => Err((
                        s,
                        GenericResource::ComplexResources(ComplexResource::Water(r1)),
                        GenericResource::ComplexResources(ComplexResource::Life(r2)),
                    )),
                }
            }
            // The "Life" complex resource takes Water + Carbon
            ComplexResourceRequest::Life(r1, r2) => {
                let energy_cell = match state.full_cell() {
                    Some((c, _)) => c,
                    None => {
                        return Err((
                            "No energy cell available".to_string(),
                            GenericResource::ComplexResources(ComplexResource::Water(r1)),
                            GenericResource::BasicResources(BasicResource::Carbon(r2)),
                        ));
                    }
                };

                let complex = combinator.make_life(r1, r2, energy_cell);

                match complex {
                    Ok(complex) => Ok(ComplexResource::Life(complex)),
                    Err((s, r1, r2)) => Err((
                        s,
                        GenericResource::ComplexResources(ComplexResource::Water(r1)),
                        GenericResource::BasicResources(BasicResource::Carbon(r2)),
                    )),
                }
            }
            // The "Robot" complex resource takes Silicon + Life
            ComplexResourceRequest::Robot(r1, r2) => {
                let energy_cell = match state.full_cell() {
                    Some((c, _)) => c,
                    None => {
                        return Err((
                            "No energy cell available".to_string(),
                            GenericResource::BasicResources(BasicResource::Silicon(r1)),
                            GenericResource::ComplexResources(ComplexResource::Life(r2)),
                        ));
                    }
                };

                let complex = combinator.make_robot(r1, r2, energy_cell);

                match complex {
                    Ok(complex) => Ok(ComplexResource::Robot(complex)),
                    Err((s, r1, r2)) => Err((
                        s,
                        GenericResource::BasicResources(BasicResource::Silicon(r1)),
                        GenericResource::ComplexResources(ComplexResource::Life(r2)),
                    )),
                }
            }
            // The "Water" complex resource takes Hydrogen + Oxygen
            ComplexResourceRequest::Water(r1, r2) => {
                let energy_cell = match state.full_cell() {
                    Some((c, _)) => c,
                    None => {
                        return Err((
                            "No energy cell available".to_string(),
                            GenericResource::BasicResources(BasicResource::Hydrogen(r1)),
                            GenericResource::BasicResources(BasicResource::Oxygen(r2)),
                        ));
                    }
                };

                let complex = combinator.make_water(r1, r2, energy_cell);

                match complex {
                    Ok(complex) => Ok(ComplexResource::Water(complex)),
                    Err((s, r1, r2)) => Err((
                        s,
                        GenericResource::BasicResources(BasicResource::Hydrogen(r1)),
                        GenericResource::BasicResources(BasicResource::Oxygen(r2)),
                    )),
                }
            }
        }
    }
}

pub fn create_planet(
    id: u32,
    rx_orchestrator: Receiver<OrchestratorToPlanet>,
    tx_orchestrator: Sender<PlanetToOrchestrator>,
    rx_explorer: Receiver<ExplorerToPlanet>,
    //tx_explorer: Sender<PlanetToExplorer>,
) -> Planet {
    let id = id; // The planet should use the id that was given as a parameter during its creation
    let ai = Box::new(EnterpriseAi::new(id));
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
        rx_explorer,
    ) {
        Ok(planet) => planet,
        Err(error) => panic!("{error}"), // Need to handle properly error case
    }
}

// #[cfg(test)]
// mod tests {
// use common_game::{
//     components::{
//         asteroid::Asteroid,
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
// use super::*;
// use std::sync::mpsc::{Receiver, Sender, channel};
// //use crossbeam_channel::channel;

// #[test]
//     fn is_one_equal_to_one() {
//         assert_eq!(1, 1)
//     }

// #[test]
//     fn test_ai_initial_state_should_not_be_running() {
//         let ai = EnterpriseAi::new(67);
//         assert!(!ai.is_running());
//     }

//     fn create_dummy_state() -> PlanetState {
//         PlanetState {
//             id: 67,
//             energy_cells: vec![EnergyCell::new()],
//             rocket: None,
//             can_have_rocket: true,
//         }
//     }
//     fn create_state_with_charged_cell() -> PlanetState {
//        let mut state = create_dummy_state();
//        state.cell_mut(0).charge(Sunray::new());
//        state
//    }
//     fn create_state_with_rocket() -> PlanetState {
//         let mut state = create_state_with_charged_cell();
//         let _ = state.build_rocket(0);
//         state
//     }
// #[test]
//     fn ai_should_start_n_stop() {
//         let mut ai = EnterpriseAi::new(67);
//         let dummy_state = create_dummy_state();

//         ai.start(&dummy_state);
//         assert!(ai.is_running());

//         ai.stop(&dummy_state);
//         assert!(!ai.is_running());
//     }
// #[test]
//     fn planet_creation() {
//         let (tx_orchestrator, rx_orchestrator) = channel::<OrchestratorToPlanet>();
//         let (tx_to_orchestrator, _) = channel::<PlanetToOrchestrator>();
//         let (_, rx_explorer) = channel::<ExplorerToPlanet>();

//         let planet = create_planet(67, rx_orchestrator, tx_orchestrator, rx_explorer);

//         assert_eq!(planet.id(), 67);

//         let state = planet.state();
//         assert_eq!(state.cells_count(), 1);
//         assert!(!state.has_rocket());
//         assert!(state.can_have_rocket());
//     }
// #[test]
//     fn sunray_charging() {
//         let mut ai = EnterpriseAi::new(67);
//         let mut state = create_dummy_state();
//         let generator = Generator::new();
//         let combinator = Combinator::new();

//         ai.start(&state);

//         let sunray_msg = OrchestratorToPlanet::Sunray(Sunray::new());
//         let response = ai.handle_orchestrator_msg(
//             &mut state,
//             &generator,
//             &combinator,
//             sunray_msg,
//         );

//         assert!(response.is_some());
//         assert!(state.cell(0).is_charged());
//     }
// #[test]
//     fn asteroid_defensed_by_rocket() {
//         let mut ai = EnterpriseAi::new(67);
//         let mut state = create_state_with_rocket();
//         let generator = Generator::new();
//         let combinator = Combinator::new();
//         ai.start(&state);
//         let asteroid_msg = OrchestratorToPlanet::Asteroid(Asteroid::new());
//         let response = ai.handle_orchestrator_msg(
//             &mut state,
//             &generator,
//             &combinator,
//             asteroid_msg,
//         );

//     assert!(response.is_some());
// }
// #[test]
//     fn sunray_message() {
//         let mut ai = EnterpriseAi::new(67);
//         let mut state = create_dummy_state();
//         let generator = Generator::new();
//         let combinator = Combinator::new();

//         ai.start(&state);
//         let sunray_msg = OrchestratorToPlanet::Sunray(Sunray::new());

//         let response = ai.handle_orchestrator_msg(
//             &mut state,
//             &generator,
//             &combinator,
//             sunray_msg,
//         );
//         assert!(response.is_some());
// }
//     #[test]
// fn asteroid_message() {
//     let mut ai = EnterpriseAi::new(67);
//     let mut state = create_dummy_state();
//     let generator = Generator::new();
//     let combinator = Combinator::new();

//     ai.start(&state);

//     let asteroid_msg = OrchestratorToPlanet::Asteroid(Asteroid::new());
//     let response = ai.handle_orchestrator_msg(
//         &mut state,
//         &generator,
//         &combinator,
//         asteroid_msg,
//     );
//     assert!(response.is_some());
// }
// #[test]
// fn resource_generation_request() {
//     let mut ai = EnterpriseAi::new(67);
//     let mut state = create_dummy_state();
//     let generator = Generator::new();
//     let combinator = Combinator::new();

//     ai.start(&state);

//     let resource_msg =ExplorerToPlanet::GenerateResourceRequest {
//         explorer_id: 1,
//         resource: BasicResourceType::Carbon,
//     };

//     let response = ai.handle_explorer_msg(
//         &mut state,
//         &generator,
//         &combinator,
//         resource_msg,
//     );

//     assert!(response.is_some());
// }

// #[test]
// fn supported_resources_request() {
//     let mut ai = EnterpriseAi::new(67);
//     let mut state = create_dummy_state();
//     let generator = Generator::new();
//     let combinator = Combinator::new();

//     ai.start(&state);

//     let request_msg = ExplorerToPlanet::SupportedResourceRequest {
//         explorer_id: 1,
//     };

//     let response = ai.handle_explorer_msg(
//         &mut state,
//         &generator,
//         &combinator,
//         request_msg,
//     );

//     assert!(response.is_some());
// }

// #[test]
// fn available_energy_cells_request() {
//     let mut ai = EnterpriseAi::new(67);
//     let mut state = create_dummy_state();
//     let generator = Generator::new();
//     let combinator = Combinator::new();

//     ai.start(&state);

//     let request_msg = messages::ExplorerToPlanet::AvailableEnergyCellRequest {
//         explorer_id: 1,
//     };

//     let response = ai.handle_explorer_msg(
//         &mut state,
//         &generator,
//         &combinator,
//         request_msg,
//     );

//     assert!(response.is_some());

//     match response.unwrap() {
//         PlanetToExplorer::AvailableEnergyCellResponse { available_cells } => {
//             assert_eq!(available_cells, 0);
//         }
//         _ => panic!("Need AvailableEnergyCellResponse"),
//     }
// }

// #[test]
// fn incoming_explorer_request() {
//     let mut ai = EnterpriseAi::new(67);
//     let mut state = create_dummy_state();
//     let generator = Generator::new();
//     let combinator = Combinator::new();

//     ai.start(&state);

//     let (tx_explorer, _) = channel::<messages::PlanetToExplorer>();
//     let incoming_msg = OrchestratorToPlanet::IncomingExplorerRequest {
//         explorer_id: 1,
//         new_mpsc_sender: tx_explorer,
//     };

//     let response = ai.handle_orchestrator_msg(
//         &mut state,
//         &generator,
//         &combinator,
//         incoming_msg,
//     );

//     assert!(response.is_some());
// }
// #[test]
// fn outgoing_explorer_request() {
//     let mut ai = EnterpriseAi::new(67);
//     let mut state = create_dummy_state();
//     let generator = Generator::new();
//     let combinator = Combinator::new();

//     ai.start(&state);

//     let outgoing_msg = OrchestratorToPlanet::OutgoingExplorerRequest {
//         explorer_id: 1,
//     };

//     let response = ai.handle_orchestrator_msg(
//         &mut state,
//         &generator,
//         &combinator,
//         outgoing_msg,
//     );
//     assert!(response.is_some());
// }
// #[test]
// fn complex_resource_combination() {
//     let mut ai = EnterpriseAi::new(67);
//     let mut state = create_state_with_charged_cell();
//     let generator = Generator::new();
//     let mut combinator = Combinator::new();

//     combinator.add(ComplexResourceType::Water).unwrap();
//     combinator.add(ComplexResourceType::Diamond).unwrap();
//     ai.start(&state);

//     let request_msg = ExplorerToPlanet::SupportedCombinationRequest {
//         explorer_id: 1,
//     };

//     let response = ai.handle_explorer_msg(
//         &mut state,
//         &generator,
//         &combinator,
//         request_msg,
//     );

//     assert!(response.is_some());
//     match response.unwrap() {
//         PlanetToExplorer::SupportedCombinationResponse { combination_list } => {
//             assert_eq!(combination_list.len(), 2);
//         }
//         _ => panic!("Need SupportedCombinationResponse"),
//     }
// }
// #[test]
// fn ai_stopped() {
//     let mut ai = EnterpriseAi::new(67);
//     let mut state = create_dummy_state();
//     let generator = Generator::new();
//     let combinator = Combinator::new();

//     assert!(!ai.is_running());

//     let sunray_msg = OrchestratorToPlanet::Sunray(Sunray::new());
//     let response = ai.handle_orchestrator_msg(
//         &mut state,
//         &generator,
//         &combinator,
//         sunray_msg,
//     );

//     assert!(response.is_none());

//     let start_msg = OrchestratorToPlanet::StartPlanetAI;
//     let response = ai.handle_orchestrator_msg(
//         &mut state,
//         &generator,
//         &combinator,
//         start_msg,
//     );

//     assert!(response.is_some());
//     assert!(ai.is_running());
// }
// #[test]
// fn kill_planet() {
//     let mut ai = EnterpriseAi::new(67);
//     let mut state = create_dummy_state();
//     let generator = Generator::new();
//     let combinator = Combinator::new();

//     ai.start(&state);

//     let kill_msg = OrchestratorToPlanet::KillPlanet;
//     let response = ai.handle_orchestrator_msg(
//         &mut state,
//         &generator,
//         &combinator,
//         kill_msg,
//     );

//     assert!(response.is_some());
// }
// #[test]
// fn resource_generation_without_energy() {
//     let mut ai = EnterpriseAi::new(67);
//     let mut state = create_dummy_state(); // now i believe there isn't energy cell in it
//     let mut generator = Generator::new();
//     let combinator = Combinator::new();
//     generator.add(BasicResourceType::Carbon).unwrap();
//     ai.start(&state);
//     let resource_msg = ExplorerToPlanet::GenerateResourceRequest {
//         explorer_id: 1,
//         resource: BasicResourceType::Carbon,
//     };

//     let response = ai.handle_explorer_msg(
//         &mut state,
//         &generator,
//         &combinator,
//         resource_msg,
//     );
//     assert!(response.is_some());
// }
// #[test]
// fn unsupported_resource_request() {
//     let mut ai = EnterpriseAi::new(67);
//     let mut state = create_state_with_charged_cell();
//     let generator = Generator::new();
//     let combinator = Combinator::new();

//     ai.start(&state);
//     let resource_msg = ExplorerToPlanet::GenerateResourceRequest {
//         explorer_id: 1,
//         resource: BasicResourceType::Oxygen, // it is not carbon
//     };

//     let response = ai.handle_explorer_msg(
//         &mut state,
//         &generator,
//         &combinator,
//         resource_msg,
//     );
//     assert!(response.is_some());
// }
// }
