use common_game::components::planet;
// use common_game::components::{planet::Planet, sunray::Sunray};
#[allow(unused_imports)]
use common_game::{
    components::{
        planet::{Planet,PlanetState,PlanetAI,PlanetType},
        energy_cell::EnergyCell,
        rocket::Rocket,
        sunray::Sunray,
        resource::*,
    },
    protocols::{
        messages::{
            ExplorerToPlanet,
            PlanetToExplorer,
            OrchestratorToPlanet,
            PlanetToOrchestrator
        },
    }
};

use std::{sync::mpsc, time::SystemTime};


pub struct EnterpriseAi{
    // for starting and stopping the planet ai
    is_running:bool
}

impl PlanetAI for EnterpriseAi{
    fn start(&mut self, state: &PlanetState) {
        self.is_running=true;
    }

    fn stop(&mut self) {
        self.is_running=false;
    }

    fn handle_orchestrator_msg(
            &mut self,
            state: &mut PlanetState,
            msg: OrchestratorToPlanet,
        ) -> Option<PlanetToOrchestrator> {


        //return if self.is_running {
            match msg{
                OrchestratorToPlanet::Asteroid(_)=>{
                    if self.is_running{
                        self.try_build_rocket(state);        // maybe handle the result?
                        Some(PlanetToOrchestrator::AsteroidAck { planet_id: state.id(), rocket:state.take_rocket()})
                    }else{
                        None
                    }
                },
                OrchestratorToPlanet::StartPlanetAI(_)=>{
                    self.start(state);
                    Some(PlanetToOrchestrator::StartPlanetAIResult { planet_id: state.id(), timestamp: SystemTime::now() })
                },
                OrchestratorToPlanet::StopPlanetAI(_)=>{
                    if self.is_running{
                        self.stop();
                        Some(PlanetToOrchestrator::StopPlanetAIResult { planet_id: state.id(), timestamp: SystemTime::now() })
                    }else{
                        None
                    }   
                }
                OrchestratorToPlanet::Sunray(sunray)=>{
                    if self.is_running{
                        self.charge_energy_cell(state,sunray);
                        self.try_build_rocket(state);       // handle returned value
                        Some(PlanetToOrchestrator::SunrayAck { planet_id: state.id(), timestamp: SystemTime::now() })
                    } else {
                        None
                    }
                },
                OrchestratorToPlanet::InternalStateRequest(_)=>{
                    Some(PlanetToOrchestrator::InternalStateResponse { planet_id: state.id(), planet_state: state/* .clone*/, timestamp: SystemTime::now() })   //no clone trait for planetstate?????
                    // None
                },
            }
        //}else{
            //None
        //}
        
    }



    fn handle_asteroid(&mut self, state: &mut PlanetState) -> Option<Rocket> {
        if self.is_running{
            state.take_rocket()
        }else{
            None
        }
    }

    fn handle_explorer_msg(
            &mut self,
            state: &mut PlanetState,
            msg: ExplorerToPlanet,
        ) -> Option<PlanetToExplorer> {
        
        return match msg{
            ExplorerToPlanet::AvailableEnergyCellRequest { explorer_id }=>None,
            ExplorerToPlanet::CombineResourceRequest { explorer_id, msg }=>None,
            ExplorerToPlanet::GenerateResourceRequest { explorer_id, msg }=>None,
            ExplorerToPlanet::InternalStateRequest { explorer_id }=>None,
            ExplorerToPlanet::SupportedCombinationRequest { explorer_id }=>None,
            ExplorerToPlanet::SupportedResourceRequest { explorer_id }=>None,
        } 
    }
}

impl EnterpriseAi{
    pub fn new()->Self{
        // should be started by the orchestrator
        Self { is_running: false }
    }

    fn charge_energy_cell(&self,state:&mut PlanetState,sunray:Sunray){
        state.cell_mut(0).charge(sunray);   //has only one cell
    }

    fn try_build_rocket(&self,state:&mut PlanetState)->Result<(),String>{
        if !state.has_rocket(){
            if state.cell(0).is_charged(){
                let cell=state.cell_mut(0);
                match state.build_rocket(cell){
                    Ok(_)=>Ok(()),
                    Err(str)=>Err(str)
                }
            }else{
                Err(String::from("Energy cell already depleted"))
            }
        }else{
            Ok(())
        }
    }
}


use std::sync::mpsc::{Receiver,Sender};

pub fn create_planet(
    rx_orchestrator: Receiver<OrchestratorToPlanet>,
    tx_orchestrator: Sender<PlanetToOrchestrator>,
    rx_explorer: Receiver<ExplorerToPlanet>,
    tx_explorer: Sender<PlanetToExplorer>
)->Planet<EnterpriseAi>{
    let id=67; // huhhhhhhhhhhhhhh
    let ai=EnterpriseAi::new();
    let gen_rules=vec![BasicResourceType::Carbon];
    let comb_rules=vec![
        ComplexResourceType::Diamond,
        ComplexResourceType::Life
    ];

    match Planet::new(
        id,
        PlanetType::C,
        ai,
        gen_rules,  // basic resources
        comb_rules, // combinator
        (rx_orchestrator,tx_orchestrator),
        (rx_explorer,tx_explorer)
    ){
        Ok(planet)=>planet,
        Err(error)=>panic!("{error}")
    }
}

#[cfg(test)]
mod tests{
    use super::*;

    #[test]
    fn test_creation(){
        assert_eq!(1,1)
    }
}