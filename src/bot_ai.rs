use std::cmp::Ordering;

use bevy::{app::App, ecs::system::{Res, ResMut, Resource}, math::Vec2, time::Time, utils::hashbrown::{HashMap, HashSet}};
use json::object;

use crate::{ClientsData, GlobalConfig, InputKeys, ObjectData};

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


fn world_wrapped_vec(obj1: Vec2, obj2: Vec2, world_size: Vec2) -> Vec2 { // todo: move to cfg and use in stars/dust layers
    let vector_without_looping = obj2 - obj1;
    return Vec2::from((
        [vector_without_looping.x, vector_without_looping.x - world_size.x, vector_without_looping.x + world_size.x]
            .iter()
            .min_by(|&x, &y| {
                x.abs().partial_cmp(&y.abs()).unwrap_or(Ordering::Equal)
            })
            .unwrap()
            .clone(),
        [vector_without_looping.y, vector_without_looping.y - world_size.y, vector_without_looping.y + world_size.y]
            .iter()
            .min_by(|&x, &y| {
                x.abs().partial_cmp(&y.abs()).unwrap_or(Ordering::Equal)
            })
            .unwrap()
            .clone(),
    ));
}

pub fn init_bots_ai(
    app: &mut App,
){
    app.insert_resource(BotList::default());
}

pub fn calculate_bots_response(
    mut bots: ResMut<BotList>,
    clients_data: Res<ClientsData>,
    cfg: Res<GlobalConfig>,
    time: Res<Time>,
){
    for bot_id in bots.get_bots_client_ids().iter(){
        let bot_data = clients_data.get_by_client_id(*bot_id);
        let bot_object_id = bot_data.object_id;

        let mut shooting_target : Option<&ObjectData> = None;
        let mut self_data : Option<&ObjectData> = None;

        let world_size = cfg.map_size_chunks * cfg.single_chunk_size;

        // target is 
        //                         ?
        // n. powerup -> n. player / lowest hp player -> n. asteroid

        //let mut powerups =
        if bots.is_state_updated(bot_id){
            let objects = bots.get_bot_world_state(bot_id);
            if objects.is_some(){
                let objects = objects.unwrap();
                for object_data in objects.iter(){
                    match object_data.object.object_type{
                        crate::ObjectType::Ship { style, color, shields, hp } => {
                            //fly_target = 
                            if (object_data.object.id == bot_object_id){ // define self position
                                self_data = Some(object_data);
                            } else {
                                shooting_target = Some(object_data);
                            }
                        }
                        _ => {}
                    }
                }
            }
        }

        let keep_distance: f32 = 100.;
        //shooting_target = ObjectData{ object: crate::Object::, states_and_statuses: todo!(), angular_velocity: todo!(), linear_velocity: todo!(), translation: todo!(), rotation: todo!() };
        let mut acceleration_direction = Vec2::ZERO;
        let mut rotation_direction = Vec2::ZERO;
        let mut bot_inputs = InputKeys::default();
        if self_data.is_some() {
            if shooting_target.is_some(){
                let target_vector = world_wrapped_vec(self_data.unwrap().translation.truncate(), shooting_target.unwrap().translation.truncate(), world_size);
                let target_distance_squared = target_vector.length_squared();
                acceleration_direction = target_vector.normalize();
                if target_distance_squared < keep_distance.powi(2) {
                    acceleration_direction = -acceleration_direction;
                }
                //target_distance
                if acceleration_direction.x >= 0. {bot_inputs.right = true}
                else {bot_inputs.left = true}
                if acceleration_direction.y >= 0. {bot_inputs.up = true}
                else {bot_inputs.down = true}
                rotation_direction = acceleration_direction.normalize();
                bot_inputs.rotation_target = rotation_direction;
            }
        }
        
        

        
        //if shooting_target.is_some(){fly_target =};
        bots.set_bot_response(bot_id, bot_inputs);
        
    }
}