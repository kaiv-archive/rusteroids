use bevy::{input::{keyboard::KeyCode, Input}, ecs::system::{Local, Res, Resource}, prelude::*, utils::hashbrown::HashMap};
use bevy_egui::{egui::{epaint::Shadow, self}, EguiContexts};
use bevy_rapier2d::rapier::crossbeam::epoch::Pointable;
use bevy_renet::{renet::{*, transport::*}, RenetServerPlugin, transport::NetcodeServerPlugin};
use renet_visualizer::RenetServerVisualizer;
use rand::random;

use crate::{get_pos_to_spawn, spawn_ship, ClientData, ClientsData, GlobalConfig, Message, ObjectsDistribution, ServerChannel};

#[path = "bot_ai.rs"] pub mod bot_ai;
pub use bot_ai::*;



pub fn setup_commands_executer(
   app:  &mut App,
   is_server: bool
){
    app.insert_resource(ChatHistory::default());
    app.add_event::<CommandEvent>();
}







#[derive(Event)]
pub struct CommandEvent{command: String}



pub fn command_executer(
    mut server: ResMut<RenetServer>,
    mut reader: EventReader<CommandEvent>,
    mut clients_data: ResMut<ClientsData>,
    mut objects_distribution: ResMut<ObjectsDistribution>,
    mut chat_history: ResMut<ChatHistory>,
    mut cfg: ResMut<GlobalConfig>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    time: Res<Time>,
    mut commands: Commands,
    mut botlist: ResMut<BotList>,
){
    
    let mut log = |text: String|{
        chat_history.data.insert(0, text);
    };

    for event in reader.read(){
        if event.command.is_empty() || event.command.chars().all(|s| s == ' ') {continue;} // empty
        if event.command.starts_with("/"){
            let spawn_example = "/spawn [thing: asteroid/...] [chunk x: int] [chunk y: int]";
            let mut command = event.command.clone();
            log(format!("> {}", command));
            command.remove(0);
            let splitted: Vec<&str> = command.split_whitespace().collect();
            let head_command = splitted.get(0);
            let head_command = if head_command.is_some(){
                *head_command.unwrap()
            } else {
                log(format!("< There is no command body!"));
                continue;
            };
            match head_command {
                "help" => {log("< help command executed!".into())}
                "bot" => {// /spawn bot <style>
                    let thing = splitted.get(1);
                    let thing = if thing.is_some(){
                        *thing.unwrap()
                    } else {
                        log(format!("< spawn/list/despawn")); // todo
                        continue;
                    };
                    match thing {
                        "spawn" => {
                            let object_id = cfg.new_id();
                            let style = rand::random::<u8>();
                            let color = Color::Hsla { hue: rand::random::<f32>(), saturation: rand::random::<f32>(), lightness: rand::random::<f32>(), alpha: 1. } * 2.;
                            let name = "BEBROBOT";
                            let id = rand::random::<u64>();
                            let for_spawn_cl_data = ClientData::for_spawn(style, color, object_id);
                            let pos = get_pos_to_spawn(&mut objects_distribution, &mut cfg).extend(0.);
                            let entity = spawn_ship(false, &mut meshes, &mut materials, &mut commands, &for_spawn_cl_data, &mut cfg, &time);
                            commands.entity(entity).insert(Transform::from_translation(pos));
                            let new_client_data = ClientData { 
                                client_id: id,
                                object_id: object_id,
                                entity: entity,
                                style: style,
                                color: color, 
                                name: name.to_string() 
                            };
                            clients_data.add(new_client_data.clone());
                            println!("register new BOT with id {}", id);
                            botlist.register_bot(id);
                            let msg = Message::NewConnection {client_data: new_client_data};
                            let encoded: Vec<u8> = bincode::serialize(&msg).unwrap();
                            server.broadcast_message(ServerChannel::Garanteed, encoded);
                        }
                        "list" => {
                            let bots = botlist.get_bots_client_ids();
                            if bots.len() > 0{
                                log("List of bot ids:".into());
                                for bot_id in bots.iter(){
                                    log(format!("   {}", bot_id));
                                }
                            } else {
                                log("There is no bots".into());
                            }
                        }
                        "despawn" => {}
                        _ => {}
                    }
                }
                "set" => {
                    log(format!("< There is nothing to set!"));
                }
                "spawn" => {
                    let thing = splitted.get(1);
                    let thing = if thing.is_some(){
                        *thing.unwrap()
                    } else {
                        log(format!("< Using: {}", spawn_example));
                        continue;
                    };
                    match thing {
                        "asteroid" => {
                        }
                        _ => {
                            log(format!("< Unknown thing: {} \n  Using: {}", thing, spawn_example))
                        }
                    }
                }
                "kill" => {
                    log("< kill command executed!".into())
                }
                "kick" => {
                    log("< kick command executed!".into())
                }
                "say" => {
                    log("< say command executed!".into())
                }
                _ => {log(format!("< Unknown command: {}", head_command))}
            }



        } else { // regular message
            log(event.command.clone());
        }
        
        
    }
}

#[derive(Resource, Default)]
pub struct ChatHistory{
    data: Vec<String>
}




pub fn chat_renderer(
    keys: Res<Input<KeyCode>>,
    mut egui_context: EguiContexts,
    mut current_command: Local<String>,
    mut message_history: Local<Vec<String>>,
    mut input_focused: Local<bool>
){
    let mut need_focus = false;
    if keys.just_pressed(KeyCode::T){
        need_focus = true;
    }
    let ctx: &mut egui::Context = egui_context.ctx_mut();
    let mut fonts = egui::FontDefinitions::default();
        fonts.font_data.insert("Font".to_owned(),
        egui::FontData::from_static(include_bytes!("../assets/fonts/VecTerminus12Medium.otf") )
    );
    fonts.families.insert(egui::FontFamily::Name("Font".into()), vec!["Font".to_owned()]);
    fonts.families.get_mut(&egui::FontFamily::Proportional).unwrap()
        .insert(0, "Font".to_owned());
    fonts.families.get_mut(&egui::FontFamily::Monospace).unwrap()
        .insert(0, "Font".to_owned());
    ctx.set_fonts(fonts);

    let style = egui::Style{
        visuals: egui::Visuals{
            window_rounding: egui::Rounding::ZERO,
            window_shadow: Shadow::NONE,
            window_fill: egui::Color32::TRANSPARENT,
            window_stroke: egui::Stroke::NONE,
            override_text_color: Some(egui::Color32::WHITE),
            button_frame: false,
            ..default()
        },
        animation_time: 0.,
        ..default()
    };
    ctx.set_style(style.clone());
    
    let mut focused = false;

    let size = ctx.screen_rect().size();
    let history_size = 10;
    let chat_size = egui::Vec2::from([512., 256.]);
    egui::Window::new("chat_input")
    .title_bar(false)
    .fixed_pos(ctx.screen_rect().left_bottom())
    .fixed_size([chat_size.x, 0.])
    .show(ctx, |ui| {
        ui.centered_and_justified(|ui|{
            if *input_focused || need_focus{
                let r = ui.add(egui::TextEdit::singleline(&mut *current_command));
                if need_focus {
                    r.request_focus();
                };
                if r.lost_focus() && (keys.just_pressed(KeyCode::NumpadEnter) || keys.just_pressed(KeyCode::Return)){
                    message_history.insert(0, current_command.clone());
                    while message_history.len() > history_size {
                        message_history.remove(history_size);
                    }
                    *current_command = "".into();
                    r.request_focus();
                };
                focused = r.has_focus();
                *input_focused = r.has_focus();
            }
        });
    });

    egui::Window::new("chat")
        .fixed_size(chat_size)
        .title_bar(false)
        .vscroll(false)
        .hscroll(false)
        .fixed_pos(egui::pos2(0., size.y - chat_size.y - 42.))
        .show(ctx, |ui| {
            ui.vertical(|ui| {
                ui.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| {
                egui::ScrollArea::vertical().scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden).enable_scrolling(false).show(ui, |ui| {
                        let l = message_history.len();
                        for i in 0..l{
                            ui.label(egui::RichText::new(message_history[i].clone()).background_color(egui::Color32::BLACK));
                        }
                    });
                });
            });
    });

}




pub fn console_renderer(
    keys: Res<Input<KeyCode>>,
    mut egui_context: EguiContexts,
    mut console_window_open: Local<bool>,
    mut current_command: Local<String>,
    mut chat_history: ResMut<ChatHistory>,
    mut event_writer: EventWriter<CommandEvent>
){
    if keys.just_pressed(KeyCode::F1){
        *console_window_open = !*console_window_open;
    }
    let ctx: &mut egui::Context = egui_context.ctx_mut();
    let mut fonts = egui::FontDefinitions::default();
        fonts.font_data.insert("Font".to_owned(),
        egui::FontData::from_static(include_bytes!("../assets/fonts/VecTerminus12Medium.otf") )
    );
    fonts.families.insert(egui::FontFamily::Name("Font".into()), vec!["Font".to_owned()]);
    fonts.families.get_mut(&egui::FontFamily::Proportional).unwrap()
        .insert(0, "Font".to_owned());
    fonts.families.get_mut(&egui::FontFamily::Monospace).unwrap()
        .insert(0, "Font".to_owned());
    ctx.set_fonts(fonts);

    let style = egui::Style{
        visuals: egui::Visuals{
            window_rounding: egui::Rounding::ZERO,
            window_shadow: Shadow::NONE,
            window_fill: egui::Color32::TRANSPARENT,
            window_stroke: egui::Stroke::NONE,
            override_text_color: Some(egui::Color32::WHITE),
            button_frame: false,
            ..default()
        },
        animation_time: 0.,
        ..default()
    };
    ctx.set_style(style.clone());
    
    let mut need_focus = false;
    if keys.just_pressed(KeyCode::T){
        need_focus = true;
    }
    let ctx: &mut egui::Context = egui_context.ctx_mut();
    let mut fonts = egui::FontDefinitions::default();
        fonts.font_data.insert("Font".to_owned(),
        egui::FontData::from_static(include_bytes!("../assets/fonts/VecTerminus12Medium.otf") )
    );
    fonts.families.insert(egui::FontFamily::Name("Font".into()), vec!["Font".to_owned()]);
    fonts.families.get_mut(&egui::FontFamily::Proportional).unwrap()
        .insert(0, "Font".to_owned());
    fonts.families.get_mut(&egui::FontFamily::Monospace).unwrap()
        .insert(0, "Font".to_owned());
    ctx.set_fonts(fonts);

    let style = egui::Style{
        visuals: egui::Visuals{
            window_rounding: egui::Rounding::ZERO,
            window_shadow: Shadow::NONE,
            window_fill: egui::Color32::from_rgba_unmultiplied(0, 0, 0, 230),
            window_stroke: egui::Stroke::NONE,
            override_text_color: Some(egui::Color32::WHITE),
            button_frame: false,
            ..default()
        },
        animation_time: 0.,
        ..default()
    };
    ctx.set_style(style.clone());


    let history_size = 100;
    egui::Window::new("console")
        .show(ctx, |ui|{
            ui.vertical(|ui| {
                ui.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| {
                    let r = ui.add(egui::TextEdit::singleline(&mut *current_command));
                    if need_focus {
                        r.request_focus();
                    };
                    if r.lost_focus() && (keys.just_pressed(KeyCode::NumpadEnter) || keys.just_pressed(KeyCode::Return)){
                        while chat_history.data.len() > history_size {
                            chat_history.data.remove(history_size);
                        }
                        event_writer.send(CommandEvent{command:current_command.clone()});
                        *current_command = "".into();
                        r.request_focus();
                    };

                    egui::ScrollArea::vertical().scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden).enable_scrolling(false).show(ui, |ui| {
                            let l = chat_history.data.len();
                            for i in 0..l{
                                ui.label(egui::RichText::new(chat_history.data[i].clone()).background_color(egui::Color32::BLACK));
                            }
                        });
                });
            });
        });
    

}


