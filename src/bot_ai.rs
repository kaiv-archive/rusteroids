use bevy::{app::App, ecs::system::{Res, ResMut, Resource}, math::Vec2, time::Time, utils::hashbrown::{HashMap, HashSet}};
use json::object;

use crate::{ClientsData, InputKeys, ObjectData};

/*
self -> movemvents 
chunk -> self
*/



#[derive(Resource)]
pub struct BotList{
    binds: HashMap<u16, u64>, // oredered_id -> "client_id"
    reverse_binds: HashMap<u64, u16>,
    responses: HashMap<u64, InputKeys>,
    world_states: HashMap<u64, Vec<ObjectData>>,
    is_world_state_fresh: HashMap<u64, bool>,
}

impl Default for BotList{
    fn default() -> Self {
        Self {
            binds: HashMap::new(),
            reverse_binds: HashMap::new(),
            responses: HashMap::new(),
            world_states: HashMap::new(),
            is_world_state_fresh: HashMap::new()
        }
    }
}

impl BotList{
    pub fn register_bot(
        &mut self,
        client_id: u64,
    ){
        let id = self.binds.len() as u16 + 1;
        self.binds.insert(id, client_id);
        self.reverse_binds.insert(client_id, id);
    }
    pub fn unregister_bot(
        &mut self,
        client_id: &u64,
    ){
        let oid = *self.reverse_binds.get(client_id).unwrap();
        self.reverse_binds.remove(client_id);
        self.binds.remove(&oid);
    }
    pub fn get_bot_id(
        &self,
        oredered_id: u16,
    ) -> Option<&u64>{
        return self.binds.get(&oredered_id);
    }

    pub fn get_bots_client_ids(&self) -> HashSet<u64>{
        return self.reverse_binds.keys().cloned().collect()
    }

    pub fn get_bot_binds(
        &self,
    ){
        //return self.binds;
    }

    pub fn get_bot_response(
        &self, id: &u64,
    ) -> Option<&InputKeys>{
        return self.responses.get(id);
    }

    pub fn set_bot_response(
        &mut self,
        id: &u64,
        inputs: InputKeys
    ){
        self.responses.insert(*id, inputs);
    }

    pub fn set_bot_world_state(
        &mut self,
        id: u64,
        state: Vec<ObjectData>
    ){
        self.world_states.insert(id, state);
        self.is_world_state_fresh.insert(id, true);
    }
    pub fn get_bot_world_state(
        &mut self,
        id: &u64
    ) -> Option<&Vec<ObjectData>>{
        return self.world_states.get(id);
    }
    pub fn is_state_updated(
        &self,
        id: &u64,
    ) -> bool{
        let res = self.is_world_state_fresh.get(id);
        if res.is_none(){return true;}
        return *res.unwrap();
    }

}


pub fn init_bots_ai(
    app: &mut App,
){
    app.insert_resource(BotList::default());
}

pub fn calculate_bots_response(
    mut bots: ResMut<BotList>,
    clients_data: Res<ClientsData>,
    time: Res<Time>,
){
    for bot_id in bots.get_bots_client_ids().iter(){
        let bot_data = clients_data.get_by_client_id(*bot_id);
        let bot_object_id = bot_data.object_id;
        let mut shooting_target_position : Option<Vec2> = None;
        let mut fly_target : Vec2 = Vec2::ZERO;
        let mut self_position : Option<Vec2> = None;
        // 
        //let mut powerups =
        if !bots.is_state_updated(bot_id){
            println!("first step passed!");
            let objects = bots.get_bot_world_state(bot_id);
            
            if objects.is_some(){
                let objects = objects.unwrap();
                println!("number of objects: {}", objects.len());
                for object_data in objects.iter(){
                    match object_data.object.object_type{
                        crate::ObjectType::Ship { style, color, shields, hp } => {
                            //fly_target = 
                            if (object_data.object.id == bot_object_id){ // define self position
                                self_position = Some(object_data.translation.truncate());
                            }
                            println!("id: {:?}", object_data.object.id);
                        }
                        _ => {}
                    }
                }
            }
        }
        


        let mut rotation_direction = fly_target;
        //if shooting_target.is_some(){fly_target =};
        bots.set_bot_response(bot_id, InputKeys { rotation_target: rotation_direction.normalize(), ..Default::default()});
        
    }
}