
use bevy::{input::{keyboard::KeyCode, Input}, ecs::system::{Local, Res, Resource}, prelude::*, utils::hashbrown::HashMap};
use bevy_egui::{egui::{epaint::Shadow, self}, EguiContexts};
use bevy_rapier2d::rapier::crossbeam::epoch::Pointable;

use rand::random;



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
    mut reader: EventReader<CommandEvent>,
    mut chat_history: ResMut<ChatHistory>,
    mut commands: Commands
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

            let mut head_command = splitted.get(0);
            let head_command = if head_command.is_some(){
                *head_command.unwrap()
            } else {
                log(format!("< There is no command body!"));
                continue;
            };


            match head_command {
                "help" => {log("< help command executed!".into())}
                "spawn" => {
                    let thing = splitted.get(1);
                    let thing = if thing.is_some(){
                        *thing.unwrap()
                    } else {
                        log(format!("< Using: {}", spawn_example));
                        continue;
                    };
                    match thing {
                        "asteroid" => {}
                        _ => {log(format!("< Unknown thing: {} \n  Using: {}", thing, spawn_example))}
                    }
                }
                "kill" => {log("< kill command executed!".into())}
                "kick" => {log("< kick command executed!".into())}
                "say" => {log("< say command executed!".into())}
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


