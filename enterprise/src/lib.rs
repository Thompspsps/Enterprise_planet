use common_game::components::{planet::Planet, sunray::Sunray};
#[allow(unused_imports)]
use common_game::{
    components::{
        planet::{PlanetState,PlanetAI},
        energy_cell::EnergyCell,
        rocket::Rocket,
        resource,
    },
    protocols::{
        messages::{ExplorerToPlanet,PlanetToExplorer,OrchestratorToPlanet,PlanetToOrchestrator},
    }
};

use std::time::SystemTime;


pub struct Enterprise_AI{
    // for starting and stopping the planet ai
    is_running:bool
}

impl PlanetAI for Enterprise_AI{
    fn start(&mut self, state: &PlanetState) {
        self.is_running=true;
        
        todo!();
    }

    fn stop(&mut self) {
        self.is_running=false;

        todo!();
    }

    fn handle_orchestrator_msg(
            &mut self,
            state: &mut PlanetState,
            msg: OrchestratorToPlanet,
        ) -> Option<PlanetToOrchestrator> {

        return match msg{
            OrchestratorToPlanet::Asteroid(_)=>{
                // self.handle_asteroid(state);
                None
            },
            OrchestratorToPlanet::StartPlanetAI(_)=>{
                Some(PlanetToOrchestrator::StartPlanetAIResult { planet_id: state.id(), timestamp: SystemTime::now() })
            },
            OrchestratorToPlanet::StopPlanetAI(_)=>{
                Some(PlanetToOrchestrator::StopPlanetAIResult { planet_id: state.id(), timestamp: SystemTime::now() })
            }
            OrchestratorToPlanet::Sunray(sunray)=>{
                self.charge_energy_cell(state, sunray);
                // self.try_build_rocket(state);
                Some(PlanetToOrchestrator::SunrayAck { planet_id: state.id(), timestamp: SystemTime::now() })
            },
            OrchestratorToPlanet::InternalStateRequest(_)=>{
                // Some(PlanetToOrchestrator::InternalStateResponse { planet_id: state.id(), planet_state: state/* .clone*/, timestamp: SystemTime::now() })   //not clone trait for planetstate?????
                None
            },
            _=>None
        };
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
            _=>None
        } 
    }
}

impl Enterprise_AI{
    pub fn new()->Self{
        // should be started by the orchestrator
        Self { is_running: false }
    }

    fn charge_energy_cell(&self,state:&mut PlanetState,sunray:Sunray){
        state.cell_mut(0).charge(sunray);   //has only one cell
    }

    // fn try_build_rocket(&self,state:&mut PlanetState)->bool{
    //     if !state.has_rocket() {
    //         if state.cell(0).is_charged() {
    //             let cell=state.cell_mut(0);
    //             if let Ok(_) = state.build_rocket(cell) {
    //                 return true;
    //             }
    //         }

    //     }
    //     false
    // }
}


use std::sync::mpsc::{Receiver,Sender};

pub fn planet_genesis(
    id:u32,
    orchestratore_msg_channels:(Receiver<OrchestratorToPlanet>,Sender<PlanetToOrchestrator>),
    explorer_msg_channels:(Receiver<ExplorerToPlanet>,Sender<PlanetToExplorer>)
)->Result<Planet<Enterprise_AI>,String>{
    Err("todo".to_string())
}

#[cfg(test)]
mod tests{
    use super::*;

    #[test]
    fn test_creation(){
        assert_eq!(1,1)
    }
}