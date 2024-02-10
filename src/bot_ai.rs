use bevy::{app::App, ecs::system::{Res, ResMut, Resource}, math::Vec2, time::Time, utils::hashbrown::{HashMap, HashSet}};

use crate::{InputKeys, ObjectData};

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
}


pub fn init_bots_ai(
    app: &mut App,
){
    app.insert_resource(BotList::default());
}

pub fn calculate_bots_response(
    mut bots: ResMut<BotList>,
    time: Res<Time>,
){
    for bot in bots.get_bots_client_ids().iter(){
        bots.set_bot_response(bot, InputKeys { rotation_target: Vec2::from_angle(time.elapsed_seconds()), ..Default::default()});
    }
}