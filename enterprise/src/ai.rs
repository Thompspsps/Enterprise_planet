#[allow(unused_imports)]
use common_game::components::planet::{
    DummyPlanetState, Planet, PlanetAI, PlanetState, PlanetType,
};
use common_game::components::resource::*;
use common_game::components::resource::{Combinator, Generator};
use common_game::components::rocket::Rocket;
use common_game::components::sunray::Sunray;
use common_game::logging::*;
use common_game::protocols::orchestrator_planet::*;
use common_game::protocols::planet_explorer::*;
use common_game::utils::ID;
use crossbeam_channel::{Receiver, Sender};

//I've commented everything related to logging until it is fixed

// The Enterprise planet AI
pub struct EnterpriseAi {
    running: bool,     // This parameter represents the current state of the AI
    num_explorers: u8, // This parameter represents how many explorers are on the planet
    planet_id: u32,    // This parameter represents the planet ID (used for logging purposes)
}

const ORCHESTRATOR: &str = "orchestrator";

impl PlanetAI for EnterpriseAi {
    fn handle_sunray(
        &mut self,
        state: &mut PlanetState,
        generator: &Generator,
        combinator: &Combinator,
        sunray: Sunray,
    ) {
        // If there is already a charged cell, the planet will always try to build a rocket
        // If there's already a rocket, the sunray is wasted.
        // If the cell is not charged, we charge it
        // Then, we have other two possibilities: no explorers and explorers
        // If there are no explorers, the planet will prioritize self-defense
        // It will only try to build a rocket if it doesn't have any rocket
        // If there are explorers, it will store the energy cell for the explorers

        let had_charged_cell = self.has_charged_cells(state);

        let mut payload = Payload::from([
            (
                "visiting_explorers".to_string(),
                self.num_explorers.to_string(),
            ),
            ("has_rocket".to_string(), state.has_rocket().to_string()),
            (
                "had_charged_cells".to_string(),
                had_charged_cell.to_string(),
            ),
        ]);

        LogEvent::new(
            Some(Participant {
                actor_type: ActorType::Planet,
                id: self.planet_id,
            }),
            Some(Participant {
                actor_type: ActorType::Orchestrator,
                id: LogEvent::id_from_str(ORCHESTRATOR) as u32,
            }),
            EventType::MessageOrchestratorToPlanet,
            Channel::Info,
            Payload::from([("action".to_string(), "sunray_ack".to_string())]),
        )
        .emit();

        // Here the planet tries to build a rocket with a charged cell
        let mut rocket_built = false;
        if had_charged_cell && !state.has_rocket() {
            if let Some((_, at)) = state.full_cell() {
                match state.build_rocket(at) {
                    Ok(_) => {
                        rocket_built = true;
                        payload.insert(
                            "has_built_rocket_with_existing_charged_cells".to_string(),
                            true.to_string(),
                        );

                        LogEvent::new(
                            Some(Participant {
                                actor_type: ActorType::Planet,
                                id: self.planet_id,
                            }),
                            Some(Participant {
                                actor_type: ActorType::SelfActor,
                                id: self.planet_id,
                            }),
                            EventType::InternalPlanetAction,
                            Channel::Info,
                            Payload::from([("action".to_string(), "built_rocket".to_string())]),
                        )
                        .emit();
                    }
                    Err(e) => {
                        payload.insert("build_rocket_error".to_string(), e);

                        LogEvent::new(
                            Some(Participant {
                                actor_type: ActorType::Planet,
                                id: self.planet_id,
                            }),
                            Some(Participant {
                                actor_type: ActorType::SelfActor,
                                id: self.planet_id,
                            }),
                            EventType::InternalPlanetAction,
                            Channel::Warning,
                            Payload::from([(
                                "action".to_string(),
                                "rocket_build_failed".to_string(),
                            )]),
                        )
                        .emit();
                    }
                }
            }
        }

        match state.charge_cell(sunray) {
            Some(_) => {
                payload.insert("received_sunray".to_string(), "wasted".to_string());

                LogEvent::new(
                    Some(Participant {
                        actor_type: ActorType::Planet,
                        id: self.planet_id,
                    }),
                    Some(Participant {
                        actor_type: ActorType::SelfActor,
                        id: self.planet_id,
                    }),
                    EventType::InternalPlanetAction,
                    Channel::Debug,
                    Payload::from([("action".to_string(), "sunray_wasted".to_string())]),
                )
                .emit();
            }
            None => {
                payload.insert("received_sunray".to_string(), "used".to_string());

                LogEvent::new(
                    Some(Participant {
                        actor_type: ActorType::Planet,
                        id: self.planet_id,
                    }),
                    Some(Participant {
                        actor_type: ActorType::SelfActor,
                        id: self.planet_id,
                    }),
                    EventType::InternalPlanetAction,
                    Channel::Debug,
                    Payload::from([("action".to_string(), "sunray_used".to_string())]),
                )
                .emit();

                if self.num_explorers == 0 && !state.has_rocket() {
                    if let Some((_, at)) = state.full_cell() {
                        match state.build_rocket(at) {
                            Ok(_) => {
                                rocket_built = true;
                                payload.insert(
                                    "has_built_rocket_with_new_energy_cell".to_string(),
                                    true.to_string(),
                                );

                                LogEvent::new(
                                    Some(Participant {
                                        actor_type: ActorType::Planet,
                                        id: self.planet_id,
                                    }),
                                    Some(Participant {
                                        actor_type: ActorType::SelfActor,
                                        id: self.planet_id,
                                    }),
                                    EventType::InternalPlanetAction,
                                    Channel::Warning,
                                    Payload::from([(
                                        "action".to_string(),
                                        "has_built_rocket_with_new_energy_cell".to_string(),
                                    )]),
                                )
                                .emit();
                            }
                            Err(e) => {
                                payload.insert(
                                    "has_built_rocket_with_new_energy_cell".to_string(),
                                    false.to_string(),
                                );
                            }
                        }
                    }
                }
            }
        }

        payload.insert(
            "final_has_rocket".to_string(),
            state.has_rocket().to_string(),
        );
        payload.insert(
            "has_charged_cells".to_string(),
            self.has_charged_cells(state).to_string(),
        );
        payload.insert("rocket_build".to_string(), rocket_built.to_string());

        LogEvent::new(
            Some(Participant {
                actor_type: ActorType::Planet,
                id: self.planet_id,
            }),
            Some(Participant {
                actor_type: ActorType::SelfActor,
                id: self.planet_id,
            }),
            EventType::InternalPlanetAction,
            Channel::Debug,
            payload,
        )
        .emit();
    }

    fn handle_internal_state_req(
        &mut self,
        state: &mut PlanetState,
        generator: &Generator,
        combinator: &Combinator,
    ) -> DummyPlanetState {
        let payload = Payload::from([("request".to_string(), "internal_state".to_string())]);

        LogEvent::new(
            Some(Participant {
                actor_type: ActorType::Planet,
                id: self.planet_id,
            }),
            Some(Participant {
                actor_type: ActorType::Orchestrator,
                id: LogEvent::id_from_str(ORCHESTRATOR) as u32,
            }),
            EventType::MessageOrchestratorToPlanet,
            Channel::Info,
            payload,
        )
        .emit();

        state.to_dummy()
    }

    fn handle_asteroid(
        &mut self,
        state: &mut PlanetState,
        generator: &Generator,
        combinator: &Combinator,
    ) -> Option<Rocket> {
        let start_payload = Payload::from([
            ("action".to_string(), "handle_asteroid_start".to_string()),
            ("has_rocket".to_string(), state.has_rocket().to_string()),
            (
                "has_charged_cell".to_string(),
                state.full_cell().is_some().to_string(),
            ),
        ]);
        LogEvent::new(
            Some(Participant {
                actor_type: ActorType::Planet,
                id: self.planet_id,
            }),
            Some(Participant {
                actor_type: ActorType::SelfActor,
                id: self.planet_id,
            }),
            EventType::InternalPlanetAction,
            Channel::Debug,
            start_payload,
        )
        .emit();

        if !self.is_running() {
            let payload = Payload::from([("error".to_string(), "ai_not_running".to_string())]);
            LogEvent::new(
                Some(Participant {
                    actor_type: ActorType::Planet,
                    id: self.planet_id,
                }),
                Some(Participant {
                    actor_type: ActorType::SelfActor,
                    id: self.planet_id,
                }),
                EventType::InternalPlanetAction,
                Channel::Warning,
                payload,
            )
            .emit();
            return None;
        }

        // This function tries to take a rocket from the planet
        // If there is no rocket, it tries to build one
        // If this does not work, it returns None
        match state.take_rocket() {
            Some(rocket) => {
                let payload =
                    Payload::from([("defense".to_string(), "using_existing_rocket".to_string())]);
                LogEvent::new(
                    Some(Participant {
                        actor_type: ActorType::Planet,
                        id: self.planet_id,
                    }),
                    Some(Participant {
                        actor_type: ActorType::SelfActor,
                        id: self.planet_id,
                    }),
                    EventType::InternalPlanetAction,
                    Channel::Info,
                    payload,
                )
                .emit();
                Some(rocket)
            }
            None => {
                if let Some((_, at)) = state.full_cell() {
                    state.build_rocket(at);
                } else {
                    let payload = Payload::from([(
                        "warning".to_string(),
                        "no_charged_cell_for_rocket".to_string(),
                    )]);
                    LogEvent::new(
                        Some(Participant {
                            actor_type: ActorType::Planet,
                            id: self.planet_id,
                        }),
                        Some(Participant {
                            actor_type: ActorType::SelfActor,
                            id: self.planet_id,
                        }),
                        EventType::InternalPlanetAction,
                        Channel::Warning,
                        payload,
                    )
                    .emit();
                }
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
        let explorer_id = msg.explorer_id();
        let msg_type = match &msg {
            ExplorerToPlanet::SupportedResourceRequest { .. } => "SupportedResourceRequest",
            ExplorerToPlanet::SupportedCombinationRequest { .. } => "SupportedCombinationRequest",
            ExplorerToPlanet::GenerateResourceRequest { .. } => "GenerateResourceRequest",
            ExplorerToPlanet::CombineResourceRequest { .. } => "CombineResourceRequest",
            ExplorerToPlanet::AvailableEnergyCellRequest { .. } => "AvailableEnergyCellRequest",
        };

        let incoming_payload = Payload::from([
            ("message_type".to_string(), msg_type.to_string()),
            ("explorer_id".to_string(), explorer_id.to_string()),
        ]);
        LogEvent::new(
            Some(Participant {
                actor_type: ActorType::Explorer,
                id: explorer_id,
            }),
            Some(Participant {
                actor_type: ActorType::Planet,
                id: self.planet_id,
            }),
            EventType::MessageExplorerToPlanet,
            Channel::Info,
            incoming_payload,
        )
        .emit();

        if !self.is_running() {
            let payload = Payload::from([
                ("error".to_string(), "ai_not_running".to_string()),
                ("explorer_id".to_string(), explorer_id.to_string()),
            ]);
            LogEvent::new(
                Some(Participant {
                    actor_type: ActorType::Planet,
                    id: self.planet_id,
                }),
                Some(Participant {
                    actor_type: ActorType::Explorer,
                    id: explorer_id,
                }),
                EventType::MessageExplorerToPlanet,
                Channel::Warning,
                payload,
            )
            .emit();
            return Some(PlanetToExplorer::Stopped); // from the documentation "This variant is used by planets that are currently in a stopped state to acknowledge any message coming from an explorer"
        }

        match msg {
            ExplorerToPlanet::AvailableEnergyCellRequest { .. } => {
                // Counts how many energy cells are currently charged (1 or 0 in C-type planet case)
                let available = state
                    .cells_iter()
                    .filter(|energy_cell| energy_cell.is_charged())
                    .count() as u32; // i know we need only to check if the first is charged - Ok, that is more complete

                let payload = Payload::from([
                    ("request".to_string(), "available_energy_cells".to_string()),
                    ("count".to_string(), available.to_string()),
                ]);

                LogEvent::new(
                    Some(Participant {
                        actor_type: ActorType::Explorer,
                        id: explorer_id,
                    }),
                    Some(Participant {
                        actor_type: ActorType::Planet,
                        id: self.planet_id,
                    }),
                    EventType::MessageExplorerToPlanet,
                    Channel::Info,
                    payload,
                )
                .emit();

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
                let count = combinator.all_available_recipes().len();
                let payload = Payload::from([
                    ("request".to_string(), "supported_combinations".to_string()),
                    (
                        "count".to_string(),
                        combinator.all_available_recipes().len().to_string(),
                    ),
                ]);
                LogEvent::new(
                    Some(Participant {
                        actor_type: ActorType::Explorer,
                        id: explorer_id,
                    }),
                    Some(Participant {
                        actor_type: ActorType::Planet,
                        id: self.planet_id,
                    }),
                    EventType::MessageExplorerToPlanet,
                    Channel::Info,
                    payload,
                )
                .emit();
                Some(PlanetToExplorer::SupportedCombinationResponse {
                    combination_list: combinator.all_available_recipes(),
                })
            }
            ExplorerToPlanet::SupportedResourceRequest { .. } => {
                // C-type planets support only one generation rule

                let resources = generator.all_available_recipes();
                let resources_list = resources
                    .iter()
                    .map(|r| format!("{:?}", r))
                    .collect::<Vec<String>>();

                let payload = Payload::from([
                    ("request".to_string(), "supported_resources".to_string()),
                    ("resources".to_string(), resources_list.join(", ")),
                ]);
                LogEvent::new(
                    Some(Participant {
                        actor_type: ActorType::Explorer,
                        id: explorer_id,
                    }),
                    Some(Participant {
                        actor_type: ActorType::Planet,
                        id: self.planet_id,
                    }),
                    EventType::MessageExplorerToPlanet,
                    Channel::Info,
                    payload,
                )
                .emit();

                Some(PlanetToExplorer::SupportedResourceResponse {
                    resource_list: generator.all_available_recipes(),
                })
            }
        }
    }

    fn on_explorer_arrival(
        &mut self,
        state: &mut PlanetState,
        generator: &Generator,
        combinator: &Combinator,
        explorer_id: ID,
    ) {
        self.num_explorers += 1; // The number of explorers inside the planet is increased, the explorer is coming

        let payload = Payload::from([("action".to_string(), "explorer_arrival".to_string())]);
        LogEvent::new(
            Some(Participant {
                actor_type: ActorType::Planet,
                id: self.planet_id,
            }),
            Some(Participant {
                actor_type: ActorType::Orchestrator,
                id: LogEvent::id_from_str(ORCHESTRATOR) as u32,
            }),
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
            Some(Participant {
                actor_type: ActorType::Planet,
                id: self.planet_id,
            }),
            Some(Participant {
                actor_type: ActorType::SelfActor,
                id: self.planet_id,
            }),
            EventType::InternalExplorerAction,
            Channel::Debug,
            payload,
        )
        .emit();
    }

    fn on_explorer_departure(
        &mut self,
        state: &mut PlanetState,
        generator: &Generator,
        combinator: &Combinator,
        explorer_id: ID,
    ) {
        if self.num_explorers > 0 {
            self.num_explorers -= 1; // The number of explorers inside the planet is decreased, the explorer is leaving
        } else {
            let payload =
                Payload::from([("cause".to_string(), "no_explorer_arrived_yet".to_string())]);

            LogEvent::new(
                Some(Participant {
                    actor_type: ActorType::Planet,
                    id: self.planet_id,
                }),
                Some(Participant {
                    actor_type: ActorType::SelfActor,
                    id: self.planet_id,
                }),
                EventType::InternalExplorerAction,
                Channel::Debug,
                payload,
            )
            .emit();
        }

        let payload = Payload::from([("action".to_string(), "explorer_departure".to_string())]);

        LogEvent::new(
            Some(Participant {
                actor_type: ActorType::Planet,
                id: self.planet_id,
            }),
            Some(Participant {
                actor_type: ActorType::Orchestrator,
                id: LogEvent::id_from_str(ORCHESTRATOR) as u32,
            }),
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
            Some(Participant {
                actor_type: ActorType::Planet,
                id: self.planet_id,
            }),
            Some(Participant {
                actor_type: ActorType::SelfActor,
                id: self.planet_id,
            }),
            EventType::InternalExplorerAction,
            Channel::Debug,
            payload,
        )
        .emit();
    }

    fn on_start(&mut self, _state: &PlanetState, _generator: &Generator, _combinator: &Combinator) {
        self.running = true; // Flags the parameter to true, the planet is active
        self.num_explorers = 0; // There are no explorers when the planet is created

        let payload = Payload::from([
            ("action".to_string(), "started".to_string()),
            ("explorer_count".to_string(), self.num_explorers.to_string()),
        ]);

        LogEvent::new(
            Some(Participant {
                actor_type: ActorType::Planet,
                id: self.planet_id,
            }),
            Some(Participant {
                actor_type: ActorType::SelfActor,
                id: self.planet_id,
            }),
            EventType::InternalPlanetAction,
            Channel::Info,
            payload,
        )
        .emit();
    }

    fn on_stop(&mut self, state: &PlanetState, generator: &Generator, combinator: &Combinator) {
        self.running = false; // Flags the parameter to false, the planet is stopped
        self.num_explorers = 0; // The number of explorers is brought to zero

        let payload = Payload::from([("action".to_string(), "stopped".to_string())]);

        LogEvent::new(
            Some(Participant {
                actor_type: ActorType::Planet,
                id: self.planet_id,
            }),
            Some(Participant {
                actor_type: ActorType::SelfActor,
                id: self.planet_id,
            }),
            EventType::InternalPlanetAction,
            Channel::Info,
            payload,
        )
        .emit();
    }
}

impl EnterpriseAi {
    pub fn new(planet_id: u32) -> Self {
        let payload = Payload::from([
            ("action".to_string(), "init".to_string()),
            ("planet_type".to_string(), "C".to_string()),
        ]);

        LogEvent::new(
            Some(Participant {
                actor_type: ActorType::Orchestrator,
                id: LogEvent::id_from_str(ORCHESTRATOR) as u32,
            }),
            Some(Participant {
                actor_type: ActorType::Planet,
                id: planet_id,
            }),
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
            Some(Participant {
                actor_type: ActorType::Planet,
                id: self.planet_id,
            }),
            Some(Participant {
                actor_type: ActorType::SelfActor,
                id: self.planet_id,
            }),
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
            let payload = Payload::from([
                ("error".to_string(), "unsupported_resource".to_string()),
                ("requested_resource".to_string(), format!("{:?}", request)),
            ]);
            LogEvent::new(
                Some(Participant {
                    actor_type: ActorType::Planet,
                    id: self.planet_id,
                }),
                Some(Participant {
                    actor_type: ActorType::SelfActor,
                    id: self.planet_id,
                }),
                EventType::InternalPlanetAction,
                Channel::Warning,
                payload,
            )
            .emit();
            return None;
        } else {
            let energy_cell = match state.full_cell() {
                Some((c, i)) => {
                    let payload = Payload::from([
                        ("action".to_string(), "using_charged_cell".to_string()),
                        ("cell_index".to_string(), i.to_string()),
                    ]);
                    LogEvent::new(
                        Some(Participant {
                            actor_type: ActorType::Planet,
                            id: self.planet_id,
                        }),
                        Some(Participant {
                            actor_type: ActorType::SelfActor,
                            id: self.planet_id,
                        }),
                        EventType::InternalPlanetAction,
                        Channel::Debug,
                        payload,
                    )
                    .emit();
                    c
                }
                None => {
                    let payload =
                        Payload::from([("error".to_string(), "no_charged_cell".to_string())]);
                    LogEvent::new(
                        Some(Participant {
                            actor_type: ActorType::Planet,
                            id: self.planet_id,
                        }),
                        Some(Participant {
                            actor_type: ActorType::SelfActor,
                            id: self.planet_id,
                        }),
                        EventType::InternalPlanetAction,
                        Channel::Warning,
                        payload,
                    )
                    .emit();
                    return None;
                }
            };

            let new_resource = generator
                .make_carbon(energy_cell)
                .map(|new_carbon| BasicResource::Carbon(new_carbon));

            match new_resource {
                Ok(new_resource) => {
                    let payload = Payload::from([
                        ("action".to_string(), "resource_generated".to_string()),
                        ("resource".to_string(), "Carbon".to_string()),
                    ]);
                    LogEvent::new(
                        Some(Participant {
                            actor_type: ActorType::Planet,
                            id: self.planet_id,
                        }),
                        Some(Participant {
                            actor_type: ActorType::SelfActor,
                            id: self.planet_id,
                        }),
                        EventType::InternalPlanetAction,
                        Channel::Info,
                        payload,
                    )
                    .emit();
                    return Some(new_resource);
                }
                Err(e) => {
                    let payload = Payload::from([
                        ("error".to_string(), "generation_failed".to_string()),
                        ("error_message".to_string(), e),
                    ]);
                    LogEvent::new(
                        Some(Participant {
                            actor_type: ActorType::Planet,
                            id: self.planet_id,
                        }),
                        Some(Participant {
                            actor_type: ActorType::SelfActor,
                            id: self.planet_id,
                        }),
                        EventType::InternalPlanetAction,
                        Channel::Error,
                        payload,
                    )
                    .emit();
                }
            };
        }
        None
    }

    fn handle_combine_request(
        &mut self,
        request: ComplexResourceRequest,
        combinator: &Combinator,
        state: &mut PlanetState,
    ) -> Result<ComplexResource, (String, GenericResource, GenericResource)> {
        let request_type = match &request {
            ComplexResourceRequest::Water(_, _) => "Water",
            ComplexResourceRequest::Diamond(_, _) => "Diamond",
            ComplexResourceRequest::Life(_, _) => "Life",
            ComplexResourceRequest::Robot(_, _) => "Robot",
            ComplexResourceRequest::Dolphin(_, _) => "Dolphin",
            ComplexResourceRequest::AIPartner(_, _) => "AIPartner",
        };
        match state.full_cell() {
            Some((c, i)) => {
                let cell_payload = Payload::from([
                    ("action".to_string(), "found_charged_cell".to_string()),
                    ("cell_index".to_string(), i.to_string()),
                    ("request_type".to_string(), request_type.to_string()),
                ]);
                LogEvent::new(
                    Some(Participant {
                        actor_type: ActorType::Planet,
                        id: self.planet_id,
                    }),
                    Some(Participant {
                        actor_type: ActorType::SelfActor,
                        id: self.planet_id,
                    }),
                    EventType::InternalPlanetAction,
                    Channel::Debug,
                    cell_payload,
                )
                .emit();
            }
            None => {
                let no_cell_payload = Payload::from([
                    (
                        "action".to_string(),
                        "no_charged_cell_for_combine".to_string(),
                    ),
                    ("request_type".to_string(), request_type.to_string()),
                ]);
                LogEvent::new(
                    Some(Participant {
                        actor_type: ActorType::Planet,
                        id: self.planet_id,
                    }),
                    Some(Participant {
                        actor_type: ActorType::SelfActor,
                        id: self.planet_id,
                    }),
                    EventType::InternalPlanetAction,
                    Channel::Warning,
                    no_cell_payload,
                )
                .emit();
            }
        }

        match request {
            // The "AIPartner" complex resource takes Robot + Diamond
            ComplexResourceRequest::AIPartner(r1, r2) => {
                let energy_cell = match state.full_cell() {
                    Some((c, _)) => c,
                    None => {
                        let error_payload = Payload::from([
                            ("error".to_string(), "no_energy_cell_available".to_string()),
                            ("request_type".to_string(), "AIPartner".to_string()),
                        ]);

                        LogEvent::new(
                            Some(Participant {
                                actor_type: ActorType::Planet,
                                id: self.planet_id,
                            }),
                            Some(Participant {
                                actor_type: ActorType::SelfActor,
                                id: self.planet_id,
                            }),
                            EventType::InternalPlanetAction,
                            Channel::Error,
                            error_payload,
                        )
                        .emit();

                        return Err((
                            "No energy cell available".to_string(),
                            GenericResource::ComplexResources(ComplexResource::Robot(r1)),
                            GenericResource::ComplexResources(ComplexResource::Diamond(r2)),
                        ));
                    }
                };
                let complex = combinator.make_aipartner(r1, r2, energy_cell);

                match complex {
                    Ok(complex) => {
                        let success_payload = Payload::from([
                            ("action".to_string(), "combine_success".to_string()),
                            ("resource".to_string(), "AIPartner".to_string()),
                        ]);

                        LogEvent::new(
                            Some(Participant {
                                actor_type: ActorType::Planet,
                                id: self.planet_id,
                            }),
                            Some(Participant {
                                actor_type: ActorType::SelfActor,
                                id: self.planet_id,
                            }),
                            EventType::InternalPlanetAction,
                            Channel::Info,
                            success_payload,
                        )
                        .emit();

                        Ok(ComplexResource::AIPartner(complex))
                    }
                    Err((s, r1, r2)) => {
                        let fail_payload = Payload::from([
                            ("error".to_string(), "combine_failed".to_string()),
                            ("request_type".to_string(), "AIPartner".to_string()),
                            ("error_message".to_string(), s.clone()),
                        ]);
                        LogEvent::new(
                            Some(Participant {
                                actor_type: ActorType::Planet,
                                id: self.planet_id,
                            }),
                            Some(Participant {
                                actor_type: ActorType::SelfActor,
                                id: self.planet_id,
                            }),
                            EventType::InternalPlanetAction,
                            Channel::Error,
                            fail_payload,
                        )
                        .emit();

                        Err((
                            s,
                            GenericResource::ComplexResources(ComplexResource::Robot(r1)),
                            GenericResource::ComplexResources(ComplexResource::Diamond(r2)),
                        ))
                    }
                }
            }
            // The "Diamond" complex resource takes Carbon + Carbon
            ComplexResourceRequest::Diamond(r1, r2) => {
                let energy_cell = match state.full_cell() {
                    Some((c, _)) => c,
                    None => {
                        let error_payload = Payload::from([
                            ("error".to_string(), "no_energy_cell_available".to_string()),
                            ("request_type".to_string(), "Diamond".to_string()),
                        ]);
                        LogEvent::new(
                            Some(Participant {
                                actor_type: ActorType::Planet,
                                id: self.planet_id,
                            }),
                            Some(Participant {
                                actor_type: ActorType::SelfActor,
                                id: self.planet_id,
                            }),
                            EventType::InternalPlanetAction,
                            Channel::Error,
                            error_payload,
                        )
                        .emit();

                        return Err((
                            "No energy cell available".to_string(),
                            GenericResource::BasicResources(BasicResource::Carbon(r1)),
                            GenericResource::BasicResources(BasicResource::Carbon(r2)),
                        ));
                    }
                };

                let complex = combinator.make_diamond(r1, r2, energy_cell);

                match complex {
                    Ok(complex) => {
                        let success_payload = Payload::from([
                            ("action".to_string(), "combine_success".to_string()),
                            ("resource".to_string(), "Diamond".to_string()),
                        ]);
                        LogEvent::new(
                            Some(Participant {
                                actor_type: ActorType::Planet,
                                id: self.planet_id,
                            }),
                            Some(Participant {
                                actor_type: ActorType::SelfActor,
                                id: self.planet_id,
                            }),
                            EventType::InternalPlanetAction,
                            Channel::Info,
                            success_payload,
                        )
                        .emit();

                        Ok(ComplexResource::Diamond(complex))
                    }
                    Err((s, r1, r2)) => {
                        let fail_payload = Payload::from([
                            ("error".to_string(), "combine_failed".to_string()),
                            ("request_type".to_string(), "Diamond".to_string()),
                            ("error_message".to_string(), s.clone()),
                        ]);
                        LogEvent::new(
                            Some(Participant {
                                actor_type: ActorType::Planet,
                                id: self.planet_id,
                            }),
                            Some(Participant {
                                actor_type: ActorType::SelfActor,
                                id: self.planet_id,
                            }),
                            EventType::InternalPlanetAction,
                            Channel::Error,
                            fail_payload,
                        )
                        .emit();

                        Err((
                            s,
                            GenericResource::BasicResources(BasicResource::Carbon(r1)),
                            GenericResource::BasicResources(BasicResource::Carbon(r2)),
                        ))
                    }
                }
            }
            // The "Dolphin" complex resource takes Water + Life
            ComplexResourceRequest::Dolphin(r1, r2) => {
                let energy_cell = match state.full_cell() {
                    Some((c, _)) => c,
                    None => {
                        let error_payload = Payload::from([
                            ("error".to_string(), "no_energy_cell_available".to_string()),
                            ("request_type".to_string(), "Dolphin".to_string()),
                        ]);
                        LogEvent::new(
                            Some(Participant {
                                actor_type: ActorType::Planet,
                                id: self.planet_id,
                            }),
                            Some(Participant {
                                actor_type: ActorType::SelfActor,
                                id: self.planet_id,
                            }),
                            EventType::InternalPlanetAction,
                            Channel::Error,
                            error_payload,
                        )
                        .emit();

                        return Err((
                            "No energy cell available".to_string(),
                            GenericResource::ComplexResources(ComplexResource::Water(r1)),
                            GenericResource::ComplexResources(ComplexResource::Life(r2)),
                        ));
                    }
                };

                let complex = combinator.make_dolphin(r1, r2, energy_cell);

                match complex {
                    Ok(complex) => {
                        let success_payload = Payload::from([
                            ("action".to_string(), "combine_success".to_string()),
                            ("resource".to_string(), "Dolphin".to_string()),
                        ]);
                        LogEvent::new(
                            Some(Participant {
                                actor_type: ActorType::Planet,
                                id: self.planet_id,
                            }),
                            Some(Participant {
                                actor_type: ActorType::SelfActor,
                                id: self.planet_id,
                            }),
                            EventType::InternalPlanetAction,
                            Channel::Info,
                            success_payload,
                        )
                        .emit();

                        Ok(ComplexResource::Dolphin(complex))
                    }
                    Err((s, r1, r2)) => {
                        let fail_payload = Payload::from([
                            ("error".to_string(), "combine_failed".to_string()),
                            ("request_type".to_string(), "Dolphin".to_string()),
                            ("error_message".to_string(), s.clone()),
                        ]);
                        LogEvent::new(
                            Some(Participant {
                                actor_type: ActorType::Planet,
                                id: self.planet_id,
                            }),
                            Some(Participant {
                                actor_type: ActorType::SelfActor,
                                id: self.planet_id,
                            }),
                            EventType::InternalPlanetAction,
                            Channel::Error,
                            fail_payload,
                        )
                        .emit();

                        Err((
                            s,
                            GenericResource::ComplexResources(ComplexResource::Water(r1)),
                            GenericResource::ComplexResources(ComplexResource::Life(r2)),
                        ))
                    }
                }
            }
            // The "Life" complex resource takes Water + Carbon
            ComplexResourceRequest::Life(r1, r2) => {
                let energy_cell = match state.full_cell() {
                    Some((c, _)) => c,
                    None => {
                        let error_payload = Payload::from([
                            ("error".to_string(), "no_energy_cell_available".to_string()),
                            ("request_type".to_string(), "Life".to_string()),
                        ]);
                        LogEvent::new(
                            Some(Participant {
                                actor_type: ActorType::Planet,
                                id: self.planet_id,
                            }),
                            Some(Participant {
                                actor_type: ActorType::SelfActor,
                                id: self.planet_id,
                            }),
                            EventType::InternalPlanetAction,
                            Channel::Error,
                            error_payload,
                        )
                        .emit();

                        return Err((
                            "No energy cell available".to_string(),
                            GenericResource::ComplexResources(ComplexResource::Water(r1)),
                            GenericResource::BasicResources(BasicResource::Carbon(r2)),
                        ));
                    }
                };

                let complex = combinator.make_life(r1, r2, energy_cell);

                match complex {
                    Ok(complex) => {
                        let success_payload = Payload::from([
                            ("action".to_string(), "combine_success".to_string()),
                            ("resource".to_string(), "Life".to_string()),
                        ]);
                        LogEvent::new(
                            Some(Participant {
                                actor_type: ActorType::Planet,
                                id: self.planet_id,
                            }),
                            Some(Participant {
                                actor_type: ActorType::SelfActor,
                                id: self.planet_id,
                            }),
                            EventType::InternalPlanetAction,
                            Channel::Info,
                            success_payload,
                        )
                        .emit();

                        Ok(ComplexResource::Life(complex))
                    }
                    Err((s, r1, r2)) => {
                        let fail_payload = Payload::from([
                            ("error".to_string(), "combine_failed".to_string()),
                            ("request_type".to_string(), "Life".to_string()),
                            ("error_message".to_string(), s.clone()),
                        ]);
                        LogEvent::new(
                            Some(Participant {
                                actor_type: ActorType::Planet,
                                id: self.planet_id,
                            }),
                            Some(Participant {
                                actor_type: ActorType::SelfActor,
                                id: self.planet_id,
                            }),
                            EventType::InternalPlanetAction,
                            Channel::Error,
                            fail_payload,
                        )
                        .emit();

                        Err((
                            s,
                            GenericResource::ComplexResources(ComplexResource::Water(r1)),
                            GenericResource::BasicResources(BasicResource::Carbon(r2)),
                        ))
                    }
                }
            }
            // The "Robot" complex resource takes Silicon + Life
            ComplexResourceRequest::Robot(r1, r2) => {
                let energy_cell = match state.full_cell() {
                    Some((c, _)) => c,
                    None => {
                        let error_payload = Payload::from([
                            ("error".to_string(), "no_energy_cell_available".to_string()),
                            ("request_type".to_string(), "Robot".to_string()),
                        ]);
                        LogEvent::new(
                            Some(Participant {
                                actor_type: ActorType::Planet,
                                id: self.planet_id,
                            }),
                            Some(Participant {
                                actor_type: ActorType::SelfActor,
                                id: self.planet_id,
                            }),
                            EventType::InternalPlanetAction,
                            Channel::Error,
                            error_payload,
                        )
                        .emit();

                        return Err((
                            "No energy cell available".to_string(),
                            GenericResource::BasicResources(BasicResource::Silicon(r1)),
                            GenericResource::ComplexResources(ComplexResource::Life(r2)),
                        ));
                    }
                };

                let complex = combinator.make_robot(r1, r2, energy_cell);

                match complex {
                    Ok(complex) => {
                        let success_payload = Payload::from([
                            ("action".to_string(), "combine_success".to_string()),
                            ("resource".to_string(), "Robot".to_string()),
                        ]);
                        LogEvent::new(
                            Some(Participant {
                                actor_type: ActorType::Planet,
                                id: self.planet_id,
                            }),
                            Some(Participant {
                                actor_type: ActorType::SelfActor,
                                id: self.planet_id,
                            }),
                            EventType::InternalPlanetAction,
                            Channel::Info,
                            success_payload,
                        )
                        .emit();

                        Ok(ComplexResource::Robot(complex))
                    }
                    Err((s, r1, r2)) => {
                        let fail_payload = Payload::from([
                            ("error".to_string(), "combine_failed".to_string()),
                            ("request_type".to_string(), "Robot".to_string()),
                            ("error_message".to_string(), s.clone()),
                        ]);
                        LogEvent::new(
                            Some(Participant {
                                actor_type: ActorType::Planet,
                                id: self.planet_id,
                            }),
                            Some(Participant {
                                actor_type: ActorType::SelfActor,
                                id: self.planet_id,
                            }),
                            EventType::InternalPlanetAction,
                            Channel::Error,
                            fail_payload,
                        )
                        .emit();

                        Err((
                            s,
                            GenericResource::BasicResources(BasicResource::Silicon(r1)),
                            GenericResource::ComplexResources(ComplexResource::Life(r2)),
                        ))
                    }
                }
            }
            // The "Water" complex resource takes Hydrogen + Oxygen
            ComplexResourceRequest::Water(r1, r2) => {
                let energy_cell = match state.full_cell() {
                    Some((c, _)) => c,
                    None => {
                        let error_payload = Payload::from([
                            ("error".to_string(), "no_energy_cell_available".to_string()),
                            ("request_type".to_string(), "Water".to_string()),
                        ]);
                        LogEvent::new(
                            Some(Participant {
                                actor_type: ActorType::Planet,
                                id: self.planet_id,
                            }),
                            Some(Participant {
                                actor_type: ActorType::SelfActor,
                                id: self.planet_id,
                            }),
                            EventType::InternalPlanetAction,
                            Channel::Error,
                            error_payload,
                        )
                        .emit();

                        return Err((
                            "No energy cell available".to_string(),
                            GenericResource::BasicResources(BasicResource::Hydrogen(r1)),
                            GenericResource::BasicResources(BasicResource::Oxygen(r2)),
                        ));
                    }
                };

                let complex = combinator.make_water(r1, r2, energy_cell);

                match complex {
                    Ok(complex) => {
                        let success_payload = Payload::from([
                            ("action".to_string(), "combine_success".to_string()),
                            ("resource".to_string(), "Water".to_string()),
                        ]);
                        LogEvent::new(
                            Some(Participant {
                                actor_type: ActorType::Planet,
                                id: self.planet_id,
                            }),
                            Some(Participant {
                                actor_type: ActorType::SelfActor,
                                id: self.planet_id,
                            }),
                            EventType::InternalPlanetAction,
                            Channel::Info,
                            success_payload,
                        )
                        .emit();

                        Ok(ComplexResource::Water(complex))
                    }
                    Err((s, r1, r2)) => {
                        let fail_payload = Payload::from([
                            ("error".to_string(), "combine_failed".to_string()),
                            ("request_type".to_string(), "Water".to_string()),
                            ("error_message".to_string(), s.clone()),
                        ]);
                        LogEvent::new(
                            Some(Participant {
                                actor_type: ActorType::Planet,
                                id: self.planet_id,
                            }),
                            Some(Participant {
                                actor_type: ActorType::SelfActor,
                                id: self.planet_id,
                            }),
                            EventType::InternalPlanetAction,
                            Channel::Error,
                            fail_payload,
                        )
                        .emit();

                        Err((
                            s,
                            GenericResource::BasicResources(BasicResource::Hydrogen(r1)),
                            GenericResource::BasicResources(BasicResource::Oxygen(r2)),
                        ))
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
}

//Start of tests

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

    fn create_dummy_planet() -> Planet {
        let (tx_orchestrator, rx_orchestrator) = unbounded::<OrchestratorToPlanet>();
        let (tx_to_orchestrator, _) = unbounded::<PlanetToOrchestrator>();
        let (_, rx_explorer) = unbounded::<ExplorerToPlanet>();

        let dummy_planet =
            EnterpriseAi::create_planet(67, rx_orchestrator, tx_to_orchestrator, rx_explorer); // Creating the planet

        dummy_planet
    }

    #[test]
    fn ai_initial_state_should_not_be_running() {
        let ai = EnterpriseAi::new(67);
        assert!(!ai.is_running());
    }

    #[test]
    fn planet_creation_should_be_correct() {
        let dummy_planet = create_dummy_planet(); // Creating the planet
        assert_eq!(dummy_planet.id(), 67); // Checking if the id is correct

        let state = dummy_planet.state();
        assert_eq!(state.cells_count(), 1); // Checking if the number of cells is correct (1 is the max for C-type planet)
        assert!(!state.has_rocket()); // The initial planet state shouldn't have a built rocket
        assert!(state.can_have_rocket()); // C-type planets can have a rocket
    }

    #[test]
    fn start_orchestrator() {
        let (tx_orch_in, rx_orch_in) = unbounded::<OrchestratorToPlanet>();
        let (tx_orch_out, rx_orch_out) = unbounded::<PlanetToOrchestrator>();
        let (_tx_expl_in, rx_expl_in) = unbounded::<ExplorerToPlanet>();

        let mut dummy_planet = EnterpriseAi::create_planet(67, rx_orch_in, tx_orch_out, rx_expl_in); // Creating the planet

        let start_msg = OrchestratorToPlanet::StartPlanetAI;

        let _handle = thread::spawn(move || dummy_planet.run());

        tx_orch_in.send(start_msg).unwrap();

        match rx_orch_out.recv_timeout(Duration::from_millis(50)) {
            Ok(PlanetToOrchestrator::StartPlanetAIResult { planet_id: 67 }) => {
                assert!(true)
            }
            _ => assert!(false),
        }
        thread::sleep(Duration::from_millis(50));
    }

    #[test]
    fn stop_orchestrator() {
        let (tx_orch_in, rx_orch_in) = unbounded::<OrchestratorToPlanet>();
        let (tx_orch_out, rx_orch_out) = unbounded::<PlanetToOrchestrator>();
        let (_tx_expl_in, rx_expl_in) = unbounded::<ExplorerToPlanet>();

        let mut dummy_planet = EnterpriseAi::create_planet(67, rx_orch_in, tx_orch_out, rx_expl_in); // Creating the planet

        let start_msg = OrchestratorToPlanet::StartPlanetAI;
        let stop_msg = OrchestratorToPlanet::StopPlanetAI;

        let _handle = thread::spawn(move || dummy_planet.run());

        tx_orch_in.send(start_msg).unwrap();

        match rx_orch_out.recv_timeout(Duration::from_millis(50)) {
            Ok(PlanetToOrchestrator::StartPlanetAIResult { planet_id: 67 }) => {
                assert!(true)
            }
            _ => assert!(false),
        }
        thread::sleep(Duration::from_millis(50));

        tx_orch_in.send(stop_msg).unwrap();

        match rx_orch_out.recv_timeout(Duration::from_millis(50)) {
            Ok(PlanetToOrchestrator::StopPlanetAIResult { planet_id: 67 }) => {
                assert!(true)
            }
            _ => assert!(false),
        }
        thread::sleep(Duration::from_millis(50));
    }

    #[test]
    fn sunray_orchestrator() {
        let (tx_orch_in, rx_orch_in) = unbounded::<OrchestratorToPlanet>();
        let (tx_orch_out, rx_orch_out) = unbounded::<PlanetToOrchestrator>();
        let (_tx_expl_in, rx_expl_in) = unbounded::<ExplorerToPlanet>();

        let mut dummy_planet = EnterpriseAi::create_planet(67, rx_orch_in, tx_orch_out, rx_expl_in); // Creating the planet

        let start_msg = OrchestratorToPlanet::StartPlanetAI;
        let sunray_msg = OrchestratorToPlanet::Sunray(Sunray::default());

        let _handle = thread::spawn(move || dummy_planet.run());

        tx_orch_in.send(start_msg).unwrap();

        match rx_orch_out.recv_timeout(Duration::from_millis(50)) {
            Ok(PlanetToOrchestrator::StartPlanetAIResult { planet_id: 67 }) => {
                assert!(true)
            }
            _ => assert!(false),
        }
        thread::sleep(Duration::from_millis(50));

        tx_orch_in.send(sunray_msg).unwrap();

        match rx_orch_out.recv_timeout(Duration::from_millis(50)) {
            Ok(PlanetToOrchestrator::SunrayAck { planet_id: 67 }) => {
                assert!(true)
            }
            _ => assert!(false),
        }
        thread::sleep(Duration::from_millis(50));
    }

    #[test]
    fn asteroid_orchestrator_with_sunray() {
        let (tx_orch_in, rx_orch_in) = unbounded::<OrchestratorToPlanet>();
        let (tx_orch_out, rx_orch_out) = unbounded::<PlanetToOrchestrator>();
        let (_tx_expl_in, rx_expl_in) = unbounded::<ExplorerToPlanet>();

        let mut dummy_planet = EnterpriseAi::create_planet(67, rx_orch_in, tx_orch_out, rx_expl_in); // Creating the planet

        let start_msg = OrchestratorToPlanet::StartPlanetAI;
        let sunray_msg = OrchestratorToPlanet::Sunray(Sunray::default());
        let asteroid_msg = OrchestratorToPlanet::Asteroid(Asteroid::default());

        let _handle = thread::spawn(move || dummy_planet.run());

        tx_orch_in.send(start_msg).unwrap();

        match rx_orch_out.recv_timeout(Duration::from_millis(50)) {
            Ok(PlanetToOrchestrator::StartPlanetAIResult { planet_id: 67 }) => {
                assert!(true)
            }
            _ => assert!(false),
        }
        thread::sleep(Duration::from_millis(50));

        tx_orch_in.send(sunray_msg).unwrap();

        match rx_orch_out.recv_timeout(Duration::from_millis(50)) {
            Ok(PlanetToOrchestrator::SunrayAck { planet_id: 67 }) => {
                assert!(true)
            }
            _ => assert!(false),
        }
        thread::sleep(Duration::from_millis(50));

        tx_orch_in.send(asteroid_msg).unwrap();

        match rx_orch_out.recv_timeout(Duration::from_millis(50)) {
            Ok(PlanetToOrchestrator::AsteroidAck {
                planet_id: 67,
                rocket: r,
            }) => {
                assert!(true);
                assert!(r.is_some());
            }
            _ => assert!(false),
        }
        thread::sleep(Duration::from_millis(50));
    }

    #[test]

    fn asteroid_orchestrator_with_sunray_and_energy_cell() {
        let (tx_orch_in, rx_orch_in) = unbounded::<OrchestratorToPlanet>();
        let (tx_orch_out, rx_orch_out) = unbounded::<PlanetToOrchestrator>();
        let (_tx_expl_in, rx_expl_in) = unbounded::<ExplorerToPlanet>();

        let mut dummy_planet = EnterpriseAi::create_planet(67, rx_orch_in, tx_orch_out, rx_expl_in); // Creating the planet

        let start_msg = OrchestratorToPlanet::StartPlanetAI;
        let sunray_msg = OrchestratorToPlanet::Sunray(Sunray::default());
        let asteroid_msg = OrchestratorToPlanet::Asteroid(Asteroid::default());

        let _handle = thread::spawn(move || dummy_planet.run());

        tx_orch_in.send(start_msg).unwrap();

        match rx_orch_out.recv_timeout(Duration::from_millis(50)) {
            Ok(PlanetToOrchestrator::StartPlanetAIResult { planet_id: 67 }) => {
                assert!(true)
            }
            _ => assert!(false),
        }
        thread::sleep(Duration::from_millis(50));

        tx_orch_in.send(sunray_msg).unwrap();

        match rx_orch_out.recv_timeout(Duration::from_millis(50)) {
            Ok(PlanetToOrchestrator::SunrayAck { planet_id: 67 }) => {
                assert!(true)
            }
            _ => assert!(false),
        }
        thread::sleep(Duration::from_millis(50));

        tx_orch_in
            .send(OrchestratorToPlanet::Sunray(Sunray::default()))
            .unwrap();

        match rx_orch_out.recv_timeout(Duration::from_millis(50)) {
            Ok(PlanetToOrchestrator::SunrayAck { planet_id: 67 }) => {
                assert!(true)
            }
            _ => assert!(false),
        }
        thread::sleep(Duration::from_millis(50));

        tx_orch_in.send(asteroid_msg).unwrap();

        match rx_orch_out.recv_timeout(Duration::from_millis(50)) {
            Ok(PlanetToOrchestrator::AsteroidAck {
                planet_id: 67,
                rocket: r,
            }) => {
                assert!(true);
                assert!(r.is_some());
            }
            _ => assert!(false),
        }
        thread::sleep(Duration::from_millis(50));

        tx_orch_in
            .send(OrchestratorToPlanet::Asteroid(Asteroid::default()))
            .unwrap();

        match rx_orch_out.recv_timeout(Duration::from_millis(50)) {
            Ok(PlanetToOrchestrator::AsteroidAck {
                planet_id: 67,
                rocket: r,
            }) => {
                assert!(true);
                assert!(r.is_some());
            }
            _ => assert!(false),
        }
        thread::sleep(Duration::from_millis(50));
    }

    #[test]
    fn asteroid_orchestrator_no_sunray() {
        let (tx_orch_in, rx_orch_in) = unbounded::<OrchestratorToPlanet>();
        let (tx_orch_out, rx_orch_out) = unbounded::<PlanetToOrchestrator>();
        let (_tx_expl_in, rx_expl_in) = unbounded::<ExplorerToPlanet>();

        let mut dummy_planet = EnterpriseAi::create_planet(67, rx_orch_in, tx_orch_out, rx_expl_in); // Creating the planet

        let start_msg = OrchestratorToPlanet::StartPlanetAI;
        let asteroid_msg = OrchestratorToPlanet::Asteroid(Asteroid::default());

        let _handle = thread::spawn(move || dummy_planet.run());

        tx_orch_in.send(start_msg).unwrap();

        match rx_orch_out.recv_timeout(Duration::from_millis(50)) {
            Ok(PlanetToOrchestrator::StartPlanetAIResult { planet_id: 67 }) => {
                assert!(true)
            }
            _ => assert!(false),
        }
        thread::sleep(Duration::from_millis(50));

        tx_orch_in.send(asteroid_msg).unwrap();

        match rx_orch_out.recv_timeout(Duration::from_millis(50)) {
            Ok(PlanetToOrchestrator::AsteroidAck {
                planet_id: 67,
                rocket: r,
            }) => {
                assert!(true);
                assert!(r.is_none());
            }
            _ => assert!(false),
        }
        thread::sleep(Duration::from_millis(50));
    }

    #[test]
    fn internal_state_orchestrator() {
        let (tx_orch_in, rx_orch_in) = unbounded::<OrchestratorToPlanet>();
        let (tx_orch_out, rx_orch_out) = unbounded::<PlanetToOrchestrator>();
        let (_tx_expl_in, rx_expl_in) = unbounded::<ExplorerToPlanet>();

        let mut dummy_planet = EnterpriseAi::create_planet(67, rx_orch_in, tx_orch_out, rx_expl_in); // Creating the planet

        let start_msg = OrchestratorToPlanet::StartPlanetAI;
        let internal_msg = OrchestratorToPlanet::InternalStateRequest;

        let _handle = thread::spawn(move || dummy_planet.run());

        tx_orch_in.send(start_msg).unwrap();

        match rx_orch_out.recv_timeout(Duration::from_millis(50)) {
            Ok(PlanetToOrchestrator::StartPlanetAIResult { planet_id: 67 }) => {
                assert!(true)
            }
            _ => assert!(false),
        }
        thread::sleep(Duration::from_millis(50));

        tx_orch_in.send(internal_msg).unwrap();

        match rx_orch_out.recv_timeout(Duration::from_millis(50)) {
            Ok(PlanetToOrchestrator::InternalStateResponse {
                planet_id: 67,
                planet_state: _,
            }) => {
                assert!(true)
            }
            _ => assert!(false),
        }
        thread::sleep(Duration::from_millis(50));
    }

    #[test]
    fn explorer_available_energy_zero() {
        let (tx_orch_in, rx_orch_in) = unbounded::<OrchestratorToPlanet>();
        let (tx_orch_out, rx_orch_out) = unbounded::<PlanetToOrchestrator>();
        let (tx_expl_in, rx_expl_in) = unbounded::<ExplorerToPlanet>();
        let (tx_expl_out, rx_expl_out) = unbounded::<PlanetToExplorer>();

        let mut dummy_planet = EnterpriseAi::create_planet(67, rx_orch_in, tx_orch_out, rx_expl_in); // Creating the planet

        let start_msg = OrchestratorToPlanet::StartPlanetAI;
        let explo_request = ExplorerToPlanet::AvailableEnergyCellRequest { explorer_id: 1 };

        let _handle = thread::spawn(move || dummy_planet.run());

        tx_orch_in.send(start_msg).unwrap();

        match rx_orch_out.recv_timeout(Duration::from_millis(50)) {
            Ok(PlanetToOrchestrator::StartPlanetAIResult { planet_id: 67 }) => {
                assert!(true)
            }
            _ => assert!(false),
        }
        thread::sleep(Duration::from_millis(50));

        tx_orch_in
            .send(OrchestratorToPlanet::IncomingExplorerRequest {
                explorer_id: 1,
                new_sender: tx_expl_out,
            })
            .unwrap();

        tx_expl_in.send(explo_request).unwrap();

        match rx_expl_out.recv_timeout(Duration::from_millis(100)) {
            Ok(PlanetToExplorer::AvailableEnergyCellResponse { available_cells: 0 }) => {
                assert!(true)
            }
            _ => assert!(false),
        }
        thread::sleep(Duration::from_millis(50));
    }

    #[test]

    fn explorer_available_energy_one() {
        let (tx_orch_in, rx_orch_in) = unbounded::<OrchestratorToPlanet>();
        let (tx_orch_out, rx_orch_out) = unbounded::<PlanetToOrchestrator>();
        let (tx_expl_in, rx_expl_in) = unbounded::<ExplorerToPlanet>();
        let (tx_expl_out, rx_expl_out) = unbounded::<PlanetToExplorer>();

        let mut dummy_planet = EnterpriseAi::create_planet(67, rx_orch_in, tx_orch_out, rx_expl_in); // Creating the planet

        let start_msg = OrchestratorToPlanet::StartPlanetAI;
        let explo_request = ExplorerToPlanet::AvailableEnergyCellRequest { explorer_id: 1 };

        let _handle = thread::spawn(move || dummy_planet.run());

        tx_orch_in.send(start_msg).unwrap();

        match rx_orch_out.recv_timeout(Duration::from_millis(50)) {
            Ok(PlanetToOrchestrator::StartPlanetAIResult { planet_id: 67 }) => {
                assert!(true)
            }
            _ => assert!(false),
        }
        thread::sleep(Duration::from_millis(50));

        tx_orch_in
            .send(OrchestratorToPlanet::Sunray(Sunray::default()))
            .unwrap();

        match rx_orch_out.recv_timeout(Duration::from_millis(50)) {
            Ok(PlanetToOrchestrator::SunrayAck { planet_id: 67 }) => {
                assert!(true)
            }
            _ => assert!(false),
        }
        thread::sleep(Duration::from_millis(50));

        tx_orch_in
            .send(OrchestratorToPlanet::Sunray(Sunray::default()))
            .unwrap();

        match rx_orch_out.recv_timeout(Duration::from_millis(50)) {
            Ok(PlanetToOrchestrator::SunrayAck { planet_id: 67 }) => {
                assert!(true)
            }
            _ => assert!(false),
        }
        thread::sleep(Duration::from_millis(50));

        tx_orch_in
            .send(OrchestratorToPlanet::IncomingExplorerRequest {
                explorer_id: 1,
                new_sender: tx_expl_out,
            })
            .unwrap();

        tx_expl_in.send(explo_request).unwrap();

        match rx_expl_out.recv_timeout(Duration::from_millis(100)) {
            Ok(PlanetToExplorer::AvailableEnergyCellResponse { available_cells: 1 }) => {
                assert!(true)
            }
            _ => assert!(false),
        }
        thread::sleep(Duration::from_millis(50));
    }

    #[test]
    fn explorer_generate_carbon() {
        let (tx_orch_in, rx_orch_in) = unbounded::<OrchestratorToPlanet>();
        let (tx_orch_out, rx_orch_out) = unbounded::<PlanetToOrchestrator>();
        let (tx_expl_in, rx_expl_in) = unbounded::<ExplorerToPlanet>();
        let (tx_expl_out, rx_expl_out) = unbounded::<PlanetToExplorer>();

        let mut dummy_planet = EnterpriseAi::create_planet(67, rx_orch_in, tx_orch_out, rx_expl_in); // Creating the planet

        let start_msg = OrchestratorToPlanet::StartPlanetAI;
        let explo_request = ExplorerToPlanet::GenerateResourceRequest {
            explorer_id: 1,
            resource: BasicResourceType::Carbon,
        };

        let _handle = thread::spawn(move || dummy_planet.run());

        tx_orch_in.send(start_msg).unwrap();

        match rx_orch_out.recv_timeout(Duration::from_millis(50)) {
            Ok(PlanetToOrchestrator::StartPlanetAIResult { planet_id: 67 }) => {
                assert!(true)
            }
            _ => assert!(false),
        }
        thread::sleep(Duration::from_millis(50));

        tx_orch_in
            .send(OrchestratorToPlanet::Sunray(Sunray::default()))
            .unwrap();

        match rx_orch_out.recv_timeout(Duration::from_millis(50)) {
            Ok(PlanetToOrchestrator::SunrayAck { planet_id: 67 }) => {
                assert!(true)
            }
            _ => assert!(false),
        }
        thread::sleep(Duration::from_millis(50));

        tx_orch_in
            .send(OrchestratorToPlanet::Sunray(Sunray::default()))
            .unwrap();

        match rx_orch_out.recv_timeout(Duration::from_millis(50)) {
            Ok(PlanetToOrchestrator::SunrayAck { planet_id: 67 }) => {
                assert!(true)
            }
            _ => assert!(false),
        }
        thread::sleep(Duration::from_millis(50));

        tx_orch_in
            .send(OrchestratorToPlanet::IncomingExplorerRequest {
                explorer_id: 1,
                new_sender: tx_expl_out,
            })
            .unwrap();

        tx_expl_in.send(explo_request).unwrap();

        match rx_expl_out.recv_timeout(Duration::from_millis(100)) {
            Ok(PlanetToExplorer::GenerateResourceResponse { resource: r }) => {
                assert!(r.is_some())
            }
            _ => assert!(false),
        }
        thread::sleep(Duration::from_millis(50));
    }
    #[test]
    fn explorer_generate_carbon_no_energy() {
        let (tx_orch_in, rx_orch_in) = unbounded::<OrchestratorToPlanet>();
        let (tx_orch_out, rx_orch_out) = unbounded::<PlanetToOrchestrator>();
        let (tx_expl_in, rx_expl_in) = unbounded::<ExplorerToPlanet>();
        let (tx_expl_out, rx_expl_out) = unbounded::<PlanetToExplorer>();

        let mut dummy_planet = EnterpriseAi::create_planet(67, rx_orch_in, tx_orch_out, rx_expl_in); // Creating the planet

        let start_msg = OrchestratorToPlanet::StartPlanetAI;
        let explo_request = ExplorerToPlanet::GenerateResourceRequest {
            explorer_id: 1,
            resource: BasicResourceType::Carbon,
        };

        let _handle = thread::spawn(move || dummy_planet.run());

        tx_orch_in.send(start_msg).unwrap();

        match rx_orch_out.recv_timeout(Duration::from_millis(50)) {
            Ok(PlanetToOrchestrator::StartPlanetAIResult { planet_id: 67 }) => {
                assert!(true)
            }
            _ => assert!(false),
        }
        thread::sleep(Duration::from_millis(50));

        tx_orch_in
            .send(OrchestratorToPlanet::IncomingExplorerRequest {
                explorer_id: 1,
                new_sender: tx_expl_out,
            })
            .unwrap();

        tx_expl_in.send(explo_request).unwrap();

        match rx_expl_out.recv_timeout(Duration::from_millis(100)) {
            Ok(PlanetToExplorer::GenerateResourceResponse { resource: r }) => {
                assert!(r.is_none())
            }
            _ => assert!(false),
        }
        thread::sleep(Duration::from_millis(50));
    }
}
