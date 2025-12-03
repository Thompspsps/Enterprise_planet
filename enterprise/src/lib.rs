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

use std::{collections::HashSet, sync::mpsc, time::SystemTime};


pub struct EnterpriseAi{
    // for starting and stopping the planet ai
    is_running:bool
}

impl PlanetAI for EnterpriseAi{
    fn start(&mut self, state: &PlanetState) {
        self.is_running=true;
    }

    fn stop(&mut self,state: &PlanetState) {
        self.is_running=false;
    }

    fn handle_orchestrator_msg(
            &mut self,
            state: &mut PlanetState,
            generator: &Generator,
            combinator: &Combinator,
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
                    // let out=PlanetState::clone(state);
                    Some(PlanetToOrchestrator::InternalStateResponse { planet_id: state.id(), planet_state: PlanetState::clone(state), timestamp: SystemTime::now() })   //no clone trait for planetstate?????
                    // None
                },
            }
        //}else{
            //None
        //}
        
    }



    fn handle_asteroid(&mut self, 
        state: &mut PlanetState, 
        generator: &Generator,
        combinator: &Combinator
    ) -> Option<Rocket> {
        
        match state.take_rocket(){
            Some(rocket)=>Some(rocket),
            None=>{
                self.try_build_rocket(state);
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
        
        if self.is_running{
            match msg{
                ExplorerToPlanet::AvailableEnergyCellRequest {..}=>{
                    Some(PlanetToExplorer::AvailableEnergyCellResponse { available_cells: 1 })
                },
                ExplorerToPlanet::CombineResourceRequest { explorer_id, msg }=>{
                    // Some(PlanetToExplorer::CombineResourceResponse { complex_response: None })
                    // can reuse code below for reference
                    
                    if !self.has_charged_cells(state){
                        return Some(PlanetToExplorer::CombineResourceResponse { complex_response: None })
                    }

                    // no need to check if requested resource is available for the planet's combinator

                    match msg{
                        ComplexResourceRequest::AIPartner(r1,r2)=>{
                            if let Ok(new_aipartner)=combinator.make_aipartner(r1, r2,state.cell_mut(0)){
                                return Some(PlanetToExplorer::CombineResourceResponse { complex_response: Some(ComplexResource::AIPartner(new_aipartner)) })
                            }else{
                                return Some(PlanetToExplorer::CombineResourceResponse { complex_response: None })
                            }
                        },
                        ComplexResourceRequest::Water(r1,r2)=>{
                            if let Ok(new_water)=combinator.make_water(r1, r2,state.cell_mut(0)){
                                return Some(PlanetToExplorer::CombineResourceResponse { complex_response: Some(ComplexResource::Water(new_water)) })
                            }else{
                                return Some(PlanetToExplorer::CombineResourceResponse { complex_response: None })
                            }
                        },
                        ComplexResourceRequest::Diamond(r1,r2)=>{
                            if let Ok(new_diamond)=combinator.make_diamond(r1, r2,state.cell_mut(0)){
                                return Some(PlanetToExplorer::CombineResourceResponse { complex_response: Some(ComplexResource::Diamond(new_diamond)) })
                            }else{
                                return Some(PlanetToExplorer::CombineResourceResponse { complex_response: None })
                            }
                        }
                    }
                    
                },
                ExplorerToPlanet::GenerateResourceRequest { explorer_id, resource }=>{

                    if generator.contains(resource) && self.has_charged_cells(state){
                        //if self.use_charged_cell(state){    // problem: cell is discharged before use
                            // if let Some(cell)=
                            match resource{     // i messed up...it works but...need to remove resource match patterns(except of course carbon)
                                BasicResourceType::Oxygen=>{
                                    if let Ok(new_oxygen)=generator.make_oxygen(state.cell_mut(0)){
                                        Some(PlanetToExplorer::GenerateResourceResponse { resource: Some(BasicResource::Oxygen(new_oxygen)) })
                                    }else{
                                        Some(PlanetToExplorer::GenerateResourceResponse { resource:None })
                                    }
                                },
                                BasicResourceType::Hydrogen=>{
                                    if let Ok(new_hydrogen)=generator.make_hydrogen(state.cell_mut(0)){
                                        Some(PlanetToExplorer::GenerateResourceResponse { resource: Some(BasicResource::Hydrogen(new_hydrogen)) })
                                    }else{
                                        Some(PlanetToExplorer::GenerateResourceResponse { resource:None })
                                    }
                                },
                                BasicResourceType::Carbon=>{
                                    if let Ok(new_carbon)=generator.make_carbon(state.cell_mut(0)){
                                        Some(PlanetToExplorer::GenerateResourceResponse { resource: Some(BasicResource::Carbon(new_carbon)) })
                                    }else{
                                        Some(PlanetToExplorer::GenerateResourceResponse { resource:None })
                                    }
                                },
                                BasicResourceType::Silicon=>{
                                    if let Ok(new_silicon)=generator.make_silicon(state.cell_mut(0)){
                                        Some(PlanetToExplorer::GenerateResourceResponse { resource: Some(BasicResource::Silicon(new_silicon)) })
                                    }else{
                                        Some(PlanetToExplorer::GenerateResourceResponse { resource:None })
                                    }
                                },
                            }
                        //}else{
                            //Some(PlanetToExplorer::GenerateResourceResponse { resource:None })
                        //}
                    }else{
                        Some(PlanetToExplorer::GenerateResourceResponse { resource:None })
                    }

                },
                ExplorerToPlanet::InternalStateRequest { explorer_id }=>{
                    Some(PlanetToExplorer::InternalStateResponse { planet_state: PlanetState::from(state) })
                },
                ExplorerToPlanet::SupportedCombinationRequest {..}=>{
                    // C type planets support unbounded combination rules (up to 6)
                    let rules= combinator.all_available_recipes();
                    Some(PlanetToExplorer::CombineResourceResponse { 
                        complex_response: if rules.is_empty() {None} else {Some(rules)}        // <---- read documentation
                    })
                },
                ExplorerToPlanet::SupportedResourceRequest {..}=>{
                    // let rules=HashSet::from(BasicResourceType::Carbon);
                    // C type planets support only one generation rule
                    let mut rules=HashSet::new();
                    if let Some(&rule) = generator.all_available_recipes().iter().next(){
                        rules.insert(rule);
                    }
                    Some(PlanetToExplorer::SupportedResourceResponse { 
                        resource_list: if rules.is_empty() {None} else {Some(rules)}
                    })
                },
            }
        } else{
            None
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

    fn has_charged_cells(&self,state:&PlanetState)->bool{
        state.cell(0).is_charged()
    }

    fn use_charged_cell(&self,state:&mut PlanetState)->bool{
        match state.cell_mut(0).discharge(){
            Ok(_)=>true,
            Err(_)=>false   // maybe handle error
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
    let id=67; // huhhhhhhhhhhhhhh....do we have to agree upon id values?
    let ai=EnterpriseAi::new();
    let gen_rules=vec![BasicResourceType::Carbon];
    let comb_rules=vec![
        ComplexResourceType::Water,
        ComplexResourceType::Diamond,
        ComplexResourceType::Life,
        ComplexResourceType::Robot,
        ComplexResourceType::Dolphin,
        ComplexResourceType::AIPartner
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