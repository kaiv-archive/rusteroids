use std::{f32::consts::PI, ops::RangeInclusive};
use bevy::{app::AppExit, core_pipeline::{tonemapping::{Tonemapping, DebandDither}, bloom::{BloomCompositeMode, BloomSettings}, clear_color::ClearColorConfig}, prelude::*, render::{camera::RenderTarget, mesh::Indices, render_resource::{Extent3d, PrimitiveTopology, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages}, view::RenderLayers}, sprite::{MaterialMesh2dBundle, Mesh2dHandle}};
use bevy_egui::{egui::{self, Style, Visuals, epaint::{Shadow, CircleShape}, Color32, Rounding, Align, Stroke, FontId, load::SizedTexture, Slider, TextureId, ComboBox}, EguiContexts, EguiUserTextures};
use rand::{random, Rng};

use crate::{game::*, game::components::{ConnectProperties, ClientState}};

#[derive(Component)]
pub struct LabelAnimation;

pub fn despawn_menu(
    mut commands: Commands,
    beam_q: Query<Entity, With<Beam>>,
    label_q: Query<Entity, With<LabelAnimation>>,
    preview_camera_q: Query<Entity, With<ShipPreviewCamera>>,
    preview_ship_q: Query<Entity, With<ShipPreview>>,
){
    for b in beam_q.iter(){
        commands.entity(b).despawn();
    }
    for l in label_q.iter(){
        commands.entity(l).despawn();
    }
    for e in preview_camera_q.iter(){
        commands.entity(e).despawn();
    }
    for e in preview_ship_q.iter(){
        commands.entity(e).despawn();
    }
}

pub fn update_preview_ship(
    mut commands: Commands,
    mut prev_style: Local<u8>,
    mut prev_color: Local<Color>,
    mut ship_preview: Query<(Entity, &mut Transform), With<ShipPreview>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut cfg: ResMut<GlobalConfig>,
    clients_data: Res<ClientsData>,
    time: Res<Time>,
){
    let clientdata = clients_data.get_by_client_id(0);
    let sp = ship_preview.get_single_mut();
    match sp{
        Ok(tuple) => {
            let (e, mut t) = tuple;
            if clientdata.style != *prev_style || clientdata.color != *prev_color{
                commands.entity(e).despawn_recursive();
                let player_data = clients_data.get_by_client_id(0);
                let e = spawn_ship(true, &mut meshes, &mut materials, &mut commands, player_data, &mut cfg, &time);
                commands.entity(e).insert((ShipPreview, RenderLayers::layer(GameRenderLayers::PreviewCamera as u8), Transform::from_translation(Vec3::ZERO).with_scale(Vec3::splat(9.))));
                *prev_style = clientdata.style;
                *prev_color = clientdata.color;
            }
        },
        Err(_) => {}
    }

    
}


const PREVIEW_SIZE: u32 = 64;
pub fn setup_preview_camera(
    mut egui_user_textures: ResMut<EguiUserTextures>,
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut cfg: ResMut<GlobalConfig>,
    mut clients_data: ResMut<ClientsData>,
    time: Res<Time>,
){
    clients_data.add(ClientData{
        client_id: 0,
        object_id: 0,
        style: 0,
        entity: Entity::PLACEHOLDER,
        color: Color::WHITE,
        name: "".into(),
    });

    let size = Extent3d {
        width: PREVIEW_SIZE,
        height: PREVIEW_SIZE,
        ..default()
    };
    let mut image = Image {
        texture_descriptor: TextureDescriptor {
            label: None,
            size,
            dimension: TextureDimension::D2,
            format: TextureFormat::Bgra8UnormSrgb,
            mip_level_count: 1,
            sample_count: 1,
            usage: TextureUsages::TEXTURE_BINDING
                | TextureUsages::COPY_DST
                | TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        },
        ..default()
    };

    image.resize(size);

    let image_handle = images.add(image);
    egui_user_textures.add_image(image_handle.clone());
    commands.insert_resource(ShipPreviewImage{handle: image_handle.clone()});
    let preview_pass_layer = RenderLayers::layer(GameRenderLayers::PreviewCamera as u8);
    commands.spawn((
        Camera2dBundle {
            camera_2d: Camera2d {
                clear_color: ClearColorConfig::Custom(Color::Rgba { red: 0., green: 0., blue: 0., alpha: 1. }),
                ..default()
            },
            camera: Camera {
                // render before the "main pass" camera
                order: 1,
                hdr: true,
                target: RenderTarget::Image(image_handle.clone()),
                msaa_writeback: false,
                ..default()
            },
            tonemapping: Tonemapping::TonyMcMapface,
            deband_dither: DebandDither::Enabled,
            transform: Transform::from_scale(Vec3::splat(6.)),
            ..default()
        },
        BloomSettings{ // 3. Enable bloom for the camera
            composite_mode: BloomCompositeMode::Additive,
            intensity: 0.1,
            ..default()
        },
        PixelCamera,
        ShipPreviewCamera,
        Name::new("ShipPreviewCamera")
    )).insert(preview_pass_layer);
    let player_data = clients_data.get_by_client_id(0);
    let e = spawn_ship(true, &mut meshes, &mut materials, &mut commands, player_data, &mut cfg, &time);
    commands.entity(e).insert((ShipPreview, RenderLayers::layer(GameRenderLayers::PreviewCamera as u8), Transform::from_translation(Vec3::ZERO).with_scale(Vec3::splat(9.))));
}

pub fn egui_based_menu(
   mut egui_context: EguiContexts,
   mut exit: EventWriter<AppExit>,
    //WINDOWS
   mut play_open: Local<bool>,
   mut settings_open: Local<bool>,
   mut customize_open: Local<bool>,
   //mut errors_open: Local<bool>,
    //CONNECT
   mut adress: Local<String>,
   mut port: Local<String>,
    //STYLE
   mut ship_style: Local<(u8, bool, bool, bool, bool, bool, bool, bool, bool)>,
   mut saturation: Local<f32>,
   mut color_selector_vector: Local<egui::Vec2>,
    //OTHER
   mut settings: ResMut<GameSettings>,
   mut writer: EventWriter<ApplyCameraSettings>,
   //mut writer_init: EventWriter<InitClient>,
   //mut ship_preview: ResMut<ShipPreviewImage>,
   mut connect_properties: ResMut<ConnectProperties>,
   mut next_state: ResMut<NextState<ClientState>>,

   ship_preview_image: Res<ShipPreviewImage>,
   mut clientsdata: ResMut<ClientsData>,
){
    let ship_preview_texture_id = egui_context.image_id(&ship_preview_image.handle).unwrap();
    let ctx: &mut egui::Context = egui_context.ctx_mut();
    

    let mut fonts = egui::FontDefinitions::default();
        fonts.font_data.insert("Font".to_owned(),
        //egui::FontData::from_static(include_bytes!("../assets/fonts/F77Font-0VYv.ttf") )
        //egui::FontData::from_static(include_bytes!("../assets/fonts/TerminusTTF-4.49.3.ttf") )
        //egui::FontData::from_static(include_bytes!("../assets/fonts/unifont-15.1.04.otf") )
        //egui::FontData::from_static(include_bytes!("../assets/fonts/VCR OSD Mono Cyr.ttf") )
        //egui::FontData::from_static(include_bytes!("../assets/fonts/pixelplay.ttf") )
        //egui::FontData::from_static(include_bytes!("../assets/fonts/monocraft.ttf") )
        egui::FontData::from_static(include_bytes!("../assets/fonts/VecTerminus12Medium.otf") )
        //egui::FontData::from_static(include_bytes!("../assets/fonts/rzpix.ttf") )
        //egui::FontData::from_static(include_bytes!("../assets/fonts/CozetteVector.ttf") )
        //egui::FontData::from_static(include_bytes!("../assets/fonts/bf-mnemonika-regular-regular1.ttf") )
        
    );
    
    fonts.families.insert(egui::FontFamily::Name("Font".into()), vec!["Font".to_owned()]);
    fonts.families.get_mut(&egui::FontFamily::Proportional).unwrap()
        .insert(0, "Font".to_owned());
    fonts.families.get_mut(&egui::FontFamily::Monospace).unwrap()
        .insert(0, "Font".to_owned());

    ctx.set_fonts(fonts);

    let style = Style{
        //override_text_style: Some(egui::TextStyle::Monospace),

        //drag_value_text_style: todo!(),
        //wrap: todo!(),
        //spacing: todo!(),
        //interaction: todo!(),
        visuals: Visuals{
            //dark_mode: true,
            //override_text_color: todo!(),
            //widgets: todo!(),
            //selection: todo!(),
            //hyperlink_color: todo!(),
            //faint_bg_color: todo!(),
            window_rounding: Rounding::ZERO,
            window_shadow: Shadow::NONE,
            window_fill: Color32::from_rgba_unmultiplied(0, 0, 0, 230),
            
            
            window_stroke: Stroke{
                width: 1.,
                color: Color32::from_rgba_unmultiplied(255, 255, 255, 255)
            },
            button_frame: false,

            //menu_rounding: todo!(),
            //panel_fill: todo!(),
            //popup_shadow: todo!(),
            //resize_corner_size: todo!(),
            //text_cursor_width: todo!(),
            //text_cursor_preview: todo!(),
            //clip_rect_margin: todo!(),
            //button_frame: todo!(),
            //collapsing_header_frame: todo!(),
            //indent_has_left_vline: todo!(),
            //striped: todo!(),
            //slider_trailing_fill: todo!(),
            ..default()
        },
        animation_time: 0.,
        //debug: todo!(),
        //explanation_tooltips: todo!(),
        ..default()
    };

    ctx.set_style(style.clone());
    egui::Window::new("MENU")
        .anchor(egui::Align2([Align::Center, Align::Center]), [0., 100.])
        
        //.constrain(true)
        .resizable(false)
        //.default_height(100.0)
        .default_width(100.)
        
        .title_bar(false)
        .collapsible(false)
        
        .vscroll(false)
        .hscroll(false)
        
        //.fixed_size(bevy_egui::egui::Vec2{x: 100., y: 100.})
        .show(ctx, |ui|{
            ui.set_style(style.clone());
            
            ui.vertical_centered(|ui|{
                let mut newstyle = (*ctx.style()).clone();
                newstyle.text_styles = [
                    (egui::TextStyle::Button, FontId::new(34.0, egui::FontFamily::Monospace)),
                    (egui::TextStyle::Body, FontId::new(34.0, egui::FontFamily::Monospace))
                    ].into();
                ui.style_mut().text_styles = newstyle.text_styles;
                let play_btn = ui.add_sized(     [300., 40.], egui::Button::new("⚔PLAY⚔")).clicked();
                let customize_btn = ui.add_sized([300., 40.], egui::Button::new("✱CUSTOMIZE✱")).clicked();
                let settings_btn = ui.add_sized( [300., 40.], egui::Button::new("⛭SETTINGS⛭")).clicked();
                let exit_btn = ui.add_sized(     [300., 40.], egui::Button::new("xEXITx")).clicked();
                if exit_btn{
                    exit.send(AppExit);
                }
                if play_btn{
                    *play_open = !*play_open;
                }
                if settings_btn{
                    *settings_open = !*settings_open;
                }
                if customize_btn{
                    *customize_open = !*customize_open;
                }
            });
            //ui.allocate_space(egui::Vec2::new(1.0, 10.0));
            
            
            //ui.add(egui::Slider::new(&mut ui_state.value, 0.0..=10.0).text("value"));
        });
    let center = ctx.screen_rect().center();
    egui::Window::new("⚔PLAY⚔")
        .open(&mut *play_open)
        .title_bar(true)
        .collapsible(false)
        .hscroll(false)
        .default_height(400.)
        .vscroll(false)
        .resizable(true)
        .constrain(true)
        .default_pos(center)
        .movable(true)
        .show(ctx, |ui|{
            ui.set_style(style.clone());
            let mut newstyle = (*ctx.style()).clone();
            newstyle.text_styles = [
                    (egui::TextStyle::Body, FontId::new(20.0, egui::FontFamily::Monospace)),
                    (egui::TextStyle::Button, FontId::new(20.0, egui::FontFamily::Monospace))
            ].into();
            ui.style_mut().text_styles = newstyle.text_styles;
            ui.add(egui::TextEdit::singleline(&mut *adress).hint_text("adress"));
            if *adress == ""{ ui.add(egui::Label::new(egui::RichText::new("ADRESS IS EMPTY!").color(Color32::RED)));  };
            ui.add(egui::TextEdit::singleline(&mut *port).hint_text("port").frame(true));
            if let Ok(_) = (*port).parse::<i32>(){
                if *port == ""{
                    ui.add(egui::Label::new(egui::RichText::new("PORT IS EMPTY!").color(Color32::RED))); 
                }
            } else {
                ui.add(egui::Label::new(egui::RichText::new("PORT IS INVALID!").color(Color32::RED).italics())); 
            }
            let play_clicked = ui.add(egui::Button::new("Connect!")).clicked();
            // CHECK NAME, ADRESS, PORT
            if play_clicked{
                println!("TRYING TO CONNECT");
                if *port == "" {*port = "8567".to_owned()};
                if *adress == "" {*adress = "127.0.0.1".to_owned()};
                if let Ok(_) = (*port).parse::<i32>(){
                    connect_properties.adress = format!("{}:{}", *adress, *port).into();
                    
                    let style: u8 = (*ship_style).0 * 64 + ship_style.2 as u8 * 32 + ship_style.3 as u8* 16 + ship_style.4 as u8  * 8 + ship_style.5 as u8 * 4 + ship_style.6 as u8 * 2 + ship_style.1 as u8;
                    settings.style = style;
                    //writer_init.send(InitClient);
                    next_state.set(ClientState::InGame);
                }
            }
            
    });

    egui::Window::new("⛭SETTINGS⛭")// todo: add auto-generation settings and unfifcate "function"
        .open(&mut *settings_open)
        .title_bar(true)
        .collapsible(false)
        .hscroll(true)
        .vscroll(true)
        .resizable(true)
        .constrain(true)
        .default_pos(center + egui::Vec2{x: 400., y: 0.})
        .show(ctx, |ui| {
            ui.set_style(style.clone());
            let mut newstyle = (*ctx.style()).clone();
            newstyle.text_styles = [
                    (egui::TextStyle::Body, FontId::new(20.0, egui::FontFamily::Monospace)),
                    (egui::TextStyle::Button, FontId::new(20.0, egui::FontFamily::Monospace))
            ].into();
            ui.style_mut().text_styles = newstyle.text_styles;
            egui::CollapsingHeader::new("⏺ [ GRAPHICS SETTINGS ]")
                .default_open(true)
                .show(ui, |ui| {
                    let prev_dd = settings.deband_dither;
                    egui::ComboBox::from_label("Deband dither")
                        .selected_text(format!("{}", match settings.deband_dither { // XD
                            DebandDither::Enabled => {"Enabled"},
                            DebandDither::Disabled => {"Disabled"}
                        }))
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut settings.deband_dither, DebandDither::Enabled, "Enabled");
                            ui.selectable_value(&mut settings.deband_dither, DebandDither::Disabled, "Disabled");
                        }
                    );
                    let prev_tm = settings.tonemapping;
                    egui::ComboBox::from_label("Tonemapping")
                        .selected_text(format!("{:?}", settings.tonemapping))
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut settings.tonemapping, Tonemapping::None, "None");
                            ui.selectable_value(&mut settings.tonemapping, Tonemapping::AcesFitted, "AcesFitted");
                            ui.selectable_value(&mut settings.tonemapping, Tonemapping::AgX, "AgX");
                            ui.selectable_value(&mut settings.tonemapping, Tonemapping::BlenderFilmic, "BlenderFilmic");
                            ui.selectable_value(&mut settings.tonemapping, Tonemapping::Reinhard, "Reinhard");
                            ui.selectable_value(&mut settings.tonemapping, Tonemapping::ReinhardLuminance, "ReinhardLuminance");
                            ui.selectable_value(&mut settings.tonemapping, Tonemapping::SomewhatBoringDisplayTransform, "SomewhatBoringDisplayTransform");
                            ui.selectable_value(&mut settings.tonemapping, Tonemapping::TonyMcMapface, "TonyMcMapface");
                        }
                    );
                    let prev_cm = settings.composite_mode;
                    egui::ComboBox::from_label("Composite Mode")
                        .selected_text(format!("{}", match settings.composite_mode { // XD
                            BloomCompositeMode::Additive => {"Additive"},
                            BloomCompositeMode::EnergyConserving => {"EnergyConserving"}
                        }))
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut settings.composite_mode, BloomCompositeMode::Additive, "Additive");
                            ui.selectable_value(&mut settings.composite_mode, BloomCompositeMode::EnergyConserving, "EnergyConserving");
                        }
                    );
                    let bi = ui.add(egui::Slider::new(&mut settings.bloom_intensity , 0.0..= 1.0).text("Bloom intensity"));
                    let lfb = ui.add(egui::Slider::new(&mut settings.low_frequency_boost , 0.0..= 1.0).text("Bloom low frequency boost"));
                    let lfbc = ui.add(egui::Slider::new(&mut settings.low_frequency_boost_curvature , 0.0..= 1.0).text("Bloom low frequency boost curvature"));
                    let hpf = ui.add(egui::Slider::new(&mut settings.high_pass_frequency , 0.0..= 1.0).text("Bloom high pass frequency"));
                    let th = ui.add(egui::Slider::new(&mut settings.threshold, 0.0..= 1.0).text("Bloom threshold"));
                    let ths = ui.add(egui::Slider::new(&mut settings.threshold_softness , 0.0..= 1.0).text("Bloom threshold softness"));
                    if prev_dd != settings.deband_dither {writer.send(ApplyCameraSettings::DebandDither)};
                    if prev_tm != settings.tonemapping {writer.send(ApplyCameraSettings::Tonemapping)};
                    if prev_cm != settings.composite_mode {writer.send(ApplyCameraSettings::BloomCompositeMode)};
                    if bi.changed(){writer.send(ApplyCameraSettings::Intensity)};
                    if lfb.changed(){writer.send(ApplyCameraSettings::LowFrequencyBoost)};
                    if lfbc.changed(){writer.send(ApplyCameraSettings::LowFrequencyBoostCurvature)};
                    if hpf.changed(){writer.send(ApplyCameraSettings::HighPassFrequency)};
                    if th.changed(){writer.send(ApplyCameraSettings::Threshold)};
                    if ths.changed(){writer.send(ApplyCameraSettings::ThresholdSoftness)};
            });
            egui::CollapsingHeader::new("⏺ [ MENU SETTINGS ]")
                .default_open(true)
                .show(ui, |ui| {
                    ui.add(egui::Slider::new(&mut settings.beams_number , 0..= 5000).text("Beams number"));
                    ui.add(egui::Slider::new(&mut settings.beams_spawn_radius , 100.0..= 10000.0).text("Beams spawn radius"));
                    ui.add(egui::Slider::new(&mut settings.beams_perspective_factor, 0. ..=1. ).step_by(0.01).text("Beams perspective factor"));
                    ui.add(egui::Slider::new(&mut settings.beams_speed, 0.0..=10.0).step_by(0.01).text("Beams speed"));
            });
    });
    
    egui::Window::new("✱CUSTOMIZE✱")
        .open(&mut *customize_open)
        .title_bar(true)
        .collapsible(false)
        .hscroll(false)
        .default_width(200.)
        .default_height(800.)
        .vscroll(true)
        .resizable(true)
        .constrain(true)
        .default_pos(center + egui::Vec2{x: -400., y: 0.})
        .show(ctx, |ui|{
            ui.set_style(style.clone());
            let mut newstyle = (*ctx.style()).clone();
            newstyle.text_styles = [
                    (egui::TextStyle::Body, FontId::new(20.0, egui::FontFamily::Monospace)),
                    (egui::TextStyle::Button, FontId::new(20.0, egui::FontFamily::Monospace))
            ].into();
            ui.style_mut().text_styles = newstyle.text_styles;
            ui.add(egui::TextEdit::singleline(&mut settings.name).char_limit(24));
            /*ui.horizontal(|ui| {
                ui.label("Color");
                ui.color_edit_button_rgb(&mut settings.color);
                if settings.color[0].max(settings.color[1].max(settings.color[2])) < 0.3{
                    ui.add(egui::Label::new(egui::RichText::new("TOO DARK!").color(Color32::RED)));
                };
            });*/
            ui.label("Color");

            let to_rgb = |angle: f32, lightness: f32| -> (u8, u8, u8){

                    let hue = angle % 360. / 360.;
                    let c = (1. - ((2. * lightness - 1.) as f32).abs());
                    let x = c * (1. - ((hue * 6.) % 2. - 1.).abs());
                    let m = lightness - c / 2.;
                    let res = if hue < 1./6.{
                        (c, x, 0.)
                    } else if hue < 2./6.{
                        (x, c, 0.)
                    } else if hue < 3./6.{
                        (0., c, x)
                    } else if hue < 4./6.{
                        (0., x, c)
                    } else if hue < 5./6.{
                        (x, 0., c)
                    } else {
                        (c, 0., x)
                    };
                    (
                        ((res.0 + m) * 255.) as u8,
                        ((res.1 + m) * 255.) as u8,
                        ((res.2 + m) * 255.) as u8
                    )
                };
            egui::Frame::canvas(ui.style()).show(ui, |ui| { // todo: add caching?
                //ui.ctx().request_repaint();
                let desired_size = ui.available_width() * egui::emath::vec2(0.5, 0.5);
                let (id, rect) = ui.allocate_space(desired_size);
                
                let to_screen = egui::emath::RectTransform::from_to(egui::Rect::from_x_y_ranges(-1.0..=1.0, -1.0..=1.0), rect);

                
                let steps = 32;
                let zero = egui::Pos2{x: 0., y: 0.};
                let mut shapes = vec![];
                
                for i in 0..steps{
                    let angle1 = (i as f32 / steps as f32) * 360.;
                    let angle2 = ((i + 1) as f32 / steps as f32) * 360.;
                    
                    let color1 = to_rgb(angle1, 0.5);
                    let color2 = to_rgb(angle2, 0.5);
                    let vec1 = Vec2::from_angle(PI * 2. * (i as f32 / steps as f32));
                    let vec2 = Vec2::from_angle(PI * 2. * ((i + 1) as f32 / steps as f32));
                    shapes.push(egui::epaint::Shape::Mesh(egui::epaint::Mesh{ indices: vec![0, 1, 2], vertices: vec![
                        egui::epaint::Vertex{pos: to_screen * zero, uv: zero, color: Color32::WHITE},
                        egui::epaint::Vertex{pos: to_screen * egui::Pos2{x: vec1.x, y: vec1.y}, uv: zero, color: Color32::from_rgb(color1.0, color1.1, color1.2)},
                        egui::epaint::Vertex{pos: to_screen * egui::Pos2{x: vec2.x, y: vec2.y}, uv: zero, color: Color32::from_rgb(color2.0, color2.1, color2.2)},
                    ], ..default() }));
                }
                let response = ui.interact(rect, id, egui::Sense::drag());
                if response.dragged(){
                    //println!("{}", response.dragged());
                    let pos = response.interact_pointer_pos();
                    if pos.is_some(){
                        //println!("{:?}", response.interact_pointer_pos().unwrap());
                        let pos = pos.unwrap();
                        let mut widget_vector = ((pos - rect.left_top()) * 2. - rect.size()) / rect.size();
                        if widget_vector.length_sq() > 1.{
                            widget_vector = widget_vector.normalized();
                        }
                        //println!("{:?}", response.interact_pointer_pos().unwrap());
                        

                        
                        *color_selector_vector = widget_vector;
                        
                    }
                }
                
                shapes.push(egui::epaint::Shape::Circle(CircleShape{ 
                            center: to_screen * egui::pos2(color_selector_vector.x, color_selector_vector.y), 
                            radius: 5., 
                            fill: Color32::TRANSPARENT, 
                            stroke: Stroke { width: 1., color: Color32::from_gray((color_selector_vector.length_sq() * 255.) as u8)} 
                }));
                

                ui.painter().extend(shapes);
                
            });
            ui.label("Glow");
            if *saturation < 1.{
                *saturation = 1.;
            }
            ui.add(Slider::new(&mut *saturation, RangeInclusive::new(1., 3.)));

            let lightness  = (1. - color_selector_vector.length() * 0.5); // 1. -> 0.5
            let c = to_rgb((color_selector_vector.angle() + 2. * PI) / PI * 180., lightness);
            settings.color = [c.0 as f32 / 255. * *saturation, c.1 as f32 / 255. * *saturation, c.2 as f32 / 255. * *saturation];
            

            //ui.add(Slider::new(&mut settings.color[0], RangeInclusive::new(0., 3.)).prefix("R = ")); // todo: color circle
            //ui.add(Slider::new(&mut settings.color[1], RangeInclusive::new(0., 3.)).prefix("G = "));
            //ui.add(Slider::new(&mut settings.color[2], RangeInclusive::new(0., 3.)).prefix("B = "));
            //if settings.color[0].max(settings.color[1].max(settings.color[2])) < 0.3{
            //    ui.add(egui::Label::new(egui::RichText::new("TOO DARK!").color(Color32::RED)));
            //};
            egui::ComboBox::from_label("Colormode")
               .selected_text(format!("{}", match ship_style.1 {false => {"Full"}, true => {"Aspects"}}))
               .show_ui(ui, |ui| {
                   ui.selectable_value(&mut ship_style.1, false, "Full");
                   ui.selectable_value(&mut ship_style.1, true, "Aspects");
                   }).response.changed();
            egui::ComboBox::from_label("Base")
               .selected_text(format!("{}", match (*ship_style).0 {0 => {"Cursor"}, 1 => {"Spear"}, 2 => {"Triangle"}, 3 => {"Arrow"}, _ => {"WAT?"}}))
               .show_ui(ui, |ui| {
                   ui.selectable_value(&mut (*ship_style).0, 0, "Cursor");
                   ui.selectable_value(&mut (*ship_style).0, 1, "Spear");
                   ui.selectable_value(&mut (*ship_style).0, 2, "Triangle");
                   ui.selectable_value(&mut (*ship_style).0, 3, "Arrow");
                   }).response.changed();
            ui.add(egui::Checkbox::new(&mut ship_style.2, "Lined")).changed();
            ui.add(egui::Checkbox::new(&mut ship_style.3, "Spear")).changed();
            ui.add(egui::Checkbox::new(&mut ship_style.4, "Spikes")).changed();
            ui.add(egui::Checkbox::new(&mut ship_style.5, "Gem")).changed();
            ui.add(egui::Checkbox::new(&mut ship_style.6, "Shards")).changed();
            
            let cd = clientsdata.get_mut_by_client_id(0);
            let style: u8 = (*ship_style).0 * 64 + ship_style.2 as u8 * 32 + ship_style.3 as u8* 16 + ship_style.4 as u8  * 8 + ship_style.5 as u8 * 4 + ship_style.6 as u8 * 2 + ship_style.1 as u8;
            cd.style = style;
            cd.color = Color::from(settings.color);
            // CONVERT INTO STYLE ID
            ui.image(
                SizedTexture{
                    id: ship_preview_texture_id,
                    size: egui::vec2(ui.available_size().x, ui.available_size().x),
                }
            );
    });
}

#[derive(Resource)]
pub struct ShipPreviewImage{pub handle: Handle<Image>}

#[derive(Component)]
pub struct ShipPreviewCamera;



const LABEL_RESOLUTION: u8 = 15;
const MAX_LABEL_OFFSET: f32 = 0.8;


pub fn setup_splash(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    settings: ResMut<GameSettings>,
){
    for i in 1..=LABEL_RESOLUTION{
        if i == 1{
            commands.spawn((
                SpriteBundle{
                    sprite: Sprite { color: Color::Rgba { red: 0.6, green: 0.6, blue: 0.6, alpha: 1. }, ..default() },
                    texture: asset_server.load("title.png"),
                    transform: Transform::from_translation(LABELORIGIN + Vec3::from([0., 0., 1.])).with_scale(Vec3::splat(1.)),
                    visibility: Visibility::Visible,
                    ..default()
                },
                LabelAnimation,
                Name::new("Title")
            ));
        } else {
            commands.spawn((
                SpriteBundle{
                    sprite: Sprite { color: Color::Rgba { red: 1., green: 1., blue: 1., alpha: 0.3 * 1. / LABEL_RESOLUTION as f32 }, ..default() },
                    texture: asset_server.load("title1.png"),
                    transform: Transform::from_translation(LABELORIGIN + Vec3::from([0., 0., 1. + MAX_LABEL_OFFSET * i as f32])).with_scale(Vec3::splat(1.)),
                    visibility: Visibility::Visible,
                    ..default()
                },
                LabelAnimation,
                Name::new("Title")
            ));
        }
        
    }
}
#[derive(Component)]
pub struct Beam {origin: Vec2, z: f32}

pub fn update_beams(
    mut commands: Commands,
    settings: ResMut<GameSettings>,
    asset_server: Res<AssetServer>,
    mut beams_q: Query<(&mut Beam, &mut Transform, &mut Sprite, Entity), With<Beam>>
){  

    //let perspective_factor = settings.beams_origin_offset;
    //let z = 1. - settings.beams_path_offset;

    let number = settings.beams_number as usize;
    let mut beams = beams_q.into_iter().len();
    // spawn new
    let max_len = Vec2::splat(settings.beams_spawn_radius * 0.01).extend(1000.).length();
    if beams > number {
        for (_, _, _, e) in beams_q.iter(){commands.entity(e).despawn()}
        beams = 0;
    }

    
    for _ in 0.. (number - beams){
        let pos = Vec2::from_angle(rand::thread_rng().gen_range(-PI..=PI)) * rand::thread_rng().gen_range(100. ..=settings.beams_spawn_radius);
        let rotation = Quat::from_rotation_z(Vec2::X.angle_between(pos));
        let z = rand::thread_rng().gen_range(0. ..=1000.);
        let back_dist = z * 0.001;//(max_len - (pos * 0.01).extend(1000. - z).length()) / max_len;
        let scale = Vec3{
            x: 0.1 + back_dist,
            y: 0.2 + back_dist,
            z: 0.2 + back_dist,
        };
        commands.spawn((
            SpriteBundle{
                texture: asset_server.load("beam.png"),
                transform: Transform::from_translation(pos.extend(0.)).with_rotation(rotation).with_scale(scale),
                sprite: Sprite { color: Color::Rgba { red: z * 0.0005, green: z * 0.0005, blue: z * 0.0005, alpha: 1. }, ..default() },
                ..default()
            },
            Beam{ 
                origin: pos,
                z
            }
        ));
    }
    let perspective_factor = settings.beams_perspective_factor * 1000.;
    let speed = settings.beams_speed * 10.;
    // update
    for (mut beam, mut transform, mut sprite, entity) in beams_q.iter_mut(){
        beam.z += speed;
        if beam.z > 1000.{
            beam.z = 0.;
            let pos = Vec2::from_angle(rand::thread_rng().gen_range(-PI..=PI)) * rand::thread_rng().gen_range(100. ..=settings.beams_spawn_radius);
            beam.origin = pos;
            transform.rotation = Quat::from_rotation_z(Vec2::X.angle_between(beam.origin));
        };
        transform.translation = (beam.origin * perspective_factor / (1000. - beam.z)).extend(0.);
        let back_dist = beam.z * 0.001;//(max_len - (beam.origin * 0.01).extend(1000. - beam.z).length()) / max_len;
        let scale = Vec3{
            x: 0.1 + back_dist,
            y: 0.2 + back_dist,
            z: 0.2 + back_dist,
        };
        sprite.color = Color::Rgba { red: beam.z * 0.0005, green: beam.z * 0.0005, blue: beam.z * 0.0005, alpha: 1. };
        transform.scale = scale;
        
    }
}

const LABELORIGIN: Vec3 = Vec3{x: 0., y: 128., z: 0.};

pub fn update_menu(
    mut gamelabel: Query<(&mut Transform, &mut Sprite), With<LabelAnimation>>,
    time: Res<Time>,
){

    // BEAMS
    
    /*for (mut beam, mut transform, mut sprite, entity) in beams.iter_mut(){
        
        let pathperc = ((transform.translation.length() - settings.beams_path_offset) / settings.beams_path_len).clamp(0., 1.);
        //let colormat = materials.get_mut(&handle).unwrap();
        //println!("{:?}", local_offset);
        

        if pathperc >= 1. && random::<f32>() > 0.95{
            sprite.color.set_r(1. - random::<f32>() * 0.1);
            sprite.color.set_g(1. - random::<f32>() * 0.1);
            sprite.color.set_b(1. - random::<f32>() * 0.1);
            let newangle = random::<f32>() * PI * 2.;
            let newtarget = Vec2::from_angle(newangle);
            let newtarget = Vec3{x: newtarget.x ,y: newtarget.y, z: 0.};
            beam.velocity = newtarget;
            transform.rotation = Quat::from_rotation_z(newangle);
            beam.translation_offset = Vec3::from([(random::<f32>() - 0.5), (random::<f32>() - 0.5), 0.]) * 0.3;
            transform.translation = newtarget * settings.beams_path_offset * random::<f32>();
        };

        
        sprite.color.set_a((pathperc * PI).sin().powf(2.) * 1.);//.clamp(0., 0.5)
        
        transform.scale.x = pathperc * settings.beams_len * 0.3;
        transform.translation = transform.translation +  beam.velocity * settings.beams_speed * time.delta().as_millis() as f32  * pathperc.powf(settings.beams_path_fov);
        

       beam_count += 1;
       if beam_count > settings.beams_number{
            commands.entity(entity).despawn_recursive();
       }
    }*/
    
    /*let mut beam_count = 0;
    let delta = time.delta_seconds();
    for (mut beam, mut transform, mut sprite, entity) in beams.iter_mut(){
        beam.lifetime += delta;

        sprite.color.set_a((((beam.lifetime / settings.beams_lifetime).clamp(0., 1.) * PI)).sin().powf(2.) * 1.);

        let target_speed = beam.velocity * settings.beams_speed * beam.lifetime * 0.1;
        transform.translation += target_speed;
        transform.scale.x = 1. + target_speed.length();
        //println!("{:?}", (beam.lifetime / settings.beams_lifetime));
        if beam.lifetime > settings.beams_lifetime && random::<f32>() > 0.95{
            sprite.color.set_a(0.);
            let rotation = random::<f32>() * PI * 2.;
            let offset = random::<f32>();
            let translation_offset = Vec3::from([(random::<f32>() - 0.5), (random::<f32>() - 0.5), 0.]) * 0.3;
            let target = Vec2::from_angle(rotation);
            let target = Vec3{x: target.x, y: target.y, z: 0.};
    
           let position = settings.beams_path_offset * target + target * (settings.beams_origin_offset * offset.sqrt());
           let newtransform = Transform::from_matrix(
                        Mat4::from_rotation_translation(
                        Quat::from_rotation_z(rotation),
                        position
                    )).with_scale(Vec3{x: 0.25 * position.length().sqrt(), y: 0.25, z: 0.25}); // x: pathperc * settings.beams_len * 0.3
            transform.translation = newtransform.translation;
            transform.rotation = newtransform.rotation;
            transform.scale = newtransform.scale;
            beam.velocity = Vec3{x: target.x, y: target.y, z: 0.} * position.length().sqrt();
            beam.translation_offset = translation_offset;
            beam.lifetime = 0.;
        }
        beam_count += 1;
        if beam_count >= settings.beams_number{
            commands.entity(entity).despawn_recursive();
        }
    }
    
    
   
    if beam_count < settings.beams_number{
        //println!("{}", settings.beams_number - beam_count);
        for _ in 0..(settings.beams_number - beam_count){
            writer.send(SpawnMenuBeam{
                rotation: random::<f32>() * PI * 2.,
                offset: random::<f32>(),
                translation_offset: Vec3::from([(random::<f32>() - 0.5), (random::<f32>() - 0.5), 0.]) * 0.3,
            });
        }
    }
    */
    /////
    
    // LABEL
    
    // PULSE!!!
    let t = time.elapsed_seconds();
    let mut offset = -0.3;
    for (mut label_transform, mut sprite) in gamelabel.iter_mut(){
        label_transform.rotation.z = (1.5 * t).sin() * 0.06;
        
        //let s = 1.5 + (0.5 * t - offset).cos().abs() * 0.25 * offset * MAX_LABEL_OFFSET / LABEL_RESOLUTION as f32;
        let t_offset = 1. - ((2. * t)).cos().abs(); //.powf(0.5)

        let s = 1.5 + t_offset * offset * MAX_LABEL_OFFSET / LABEL_RESOLUTION as f32;
        label_transform.scale = Vec3::splat(s);
        if offset != -0.3{
            sprite.color.set_a(0.1 + (1. - t_offset) * 0.2);
        }
        
        //label_transform.translation.z = label_transform.scale.z;
        label_transform.translation = LABELORIGIN + (s - 1.) * Vec3{
            x: (2.5 * t /*- (offset * 10. - 1.)*/).sin() * 5.,
            y: (3.5 * t /*- (offset * 10. - 1.)*/).cos() * 5.,
            z: 1. + offset * MAX_LABEL_OFFSET
        };
        offset += 0.1;
    }



    /* SIN BASED
    let t = time.elapsed_seconds();
    let mut offset = 0.1;

    
    for mut label_transform in gamelabel.iter_mut(){
        label_transform.rotation.z = (1.5 * t).sin() * 0.06;
        
        //label_transform.rotation.z = time.elapsed_seconds().cos() * 2.;
        let s = 1.5 + (0.5 * t - offset).cos().abs() * 0.25 * offset * MAX_LABEL_OFFSET / LABEL_RESOLUTION as f32;
        label_transform.scale = Vec3::splat(s);
        //label_transform.translation.z = label_transform.scale.z;
        label_transform.translation = LABELORIGIN + (s - 1.) * Vec3{
            x: (2.5 * t - (offset * 10. - 1.)).sin() * 5.,
            y: (3.5 * t - (offset * 10. - 1.)).cos() * 5.,
            z: 1. + offset * MAX_LABEL_OFFSET
        };
        offset += 0.1;
        
    }*/
    
}   

pub fn tab_menu(
    mut egui_context: EguiContexts,
    mut settings: ResMut<GameSettings>,
    keys: Res<Input<KeyCode>>,
){
    let ctx: &mut egui::Context = egui_context.ctx_mut();
    let mut opened = keys.pressed(KeyCode::Tab);
    let style = Style{ // todo: unificate styles and fonts!
        visuals: Visuals{
            window_rounding: Rounding::ZERO,
            window_shadow: Shadow::NONE,
            window_fill: Color32::from_rgba_unmultiplied(0, 0, 0, 230),
            
            
            window_stroke: Stroke{
                width: 1.,
                color: Color32::from_rgba_unmultiplied(255, 255, 255, 255)
            },
            button_frame: false,
            ..default()
        },
        animation_time: 0.,
        ..default()
    };
    egui::Window::new("TAB")
        .anchor(egui::Align2([Align::Center, Align::Center]), [0., 0.])
        .open(&mut opened)
        //.constrain(true)
        .resizable(false)
        //.default_height(100.0)
        .default_width(500.)
        
        .title_bar(false)
        .collapsible(false)
        
        .vscroll(false)
        .hscroll(false)
        
        //.fixed_size(bevy_egui::egui::Vec2{x: 100., y: 100.})
        .show(ctx, |ui|{
            ui.set_style(style.clone());
            
            ui.vertical_centered(|ui|{
                let mut newstyle = (*ctx.style()).clone();
                newstyle.text_styles = [
                    (egui::TextStyle::Button, FontId::new(34.0, egui::FontFamily::Monospace)),
                    (egui::TextStyle::Body, FontId::new(34.0, egui::FontFamily::Monospace))
                    ].into();
                ui.style_mut().text_styles = newstyle.text_styles;
                ui.label("TAB ITEM 1");
                ui.label("TAB ITEM 2");
                ui.label("TAB ITEM 3");
                ui.label("TAB ITEM 4");
                ui.label("TAB ITEM 5");
            });
    });
}

pub fn esc_menu(
    mut esc_open: Local<bool>,
    mut settings_open: Local<bool>,
    mut egui_context: EguiContexts,
    mut settings: ResMut<GameSettings>,
    mut next_state: ResMut<NextState<ClientState>>,
    mut writer: EventWriter<ApplyCameraSettings>,
    keys: Res<Input<KeyCode>>,
){
    let ctx: &mut egui::Context = egui_context.ctx_mut();

    if keys.just_pressed(KeyCode::Escape){
        if *esc_open{
            if *settings_open{
                *settings_open = false;
            } else {
                *esc_open = false;
            }
        } else {
            if *settings_open{
                *settings_open = false;
                *esc_open = true;
            } else {
                *esc_open = true;
            }
        }
    }





    let style = Style{
        visuals: Visuals{
            window_rounding: Rounding::ZERO,
            window_shadow: Shadow::NONE,
            window_fill: Color32::from_rgba_unmultiplied(0, 0, 0, 230),
            
            
            window_stroke: Stroke{
                width: 1.,
                color: Color32::from_rgba_unmultiplied(255, 255, 255, 255)
            },
            button_frame: false,
            ..default()
        },
        animation_time: 0.,
        ..default()
    };

    ctx.set_style(style.clone());


    egui::Window::new("MENU")
        .anchor(egui::Align2([Align::Center, Align::Center]), [0., 0.])
        .open(&mut esc_open.clone())
        //.constrain(true)
        .resizable(false)
        //.default_height(100.0)
        .default_width(100.)
        
        .title_bar(false)
        .collapsible(false)
        
        .vscroll(false)
        .hscroll(false)
        
        //.fixed_size(bevy_egui::egui::Vec2{x: 100., y: 100.})
        .show(ctx, |ui|{
            ui.set_style(style.clone());
            
            ui.vertical_centered(|ui|{
                let mut newstyle = (*ctx.style()).clone();
                newstyle.text_styles = [
                    (egui::TextStyle::Button, FontId::new(34.0, egui::FontFamily::Monospace)),
                    (egui::TextStyle::Body, FontId::new(34.0, egui::FontFamily::Monospace))
                    ].into();
                ui.style_mut().text_styles = newstyle.text_styles;
                let continue_btn = ui.add_sized(     [300., 40.], egui::Button::new("CONTINUE")).clicked();
                let settings_btn = ui.add_sized( [300., 40.], egui::Button::new("SETTINGS")).clicked();
                let exit_btn = ui.add_sized(     [300., 40.], egui::Button::new("MENU")).clicked();
                if continue_btn{
                    *settings_open = false;
                    *esc_open = false;
                }
                if settings_btn{
                    *esc_open = false;
                    *settings_open = true;
                }
                if exit_btn{
                    next_state.set(ClientState::Menu);
                }
            });
    });


    let center = ctx.screen_rect().center();
    let prev_open = settings_open.clone();
    egui::Window::new("⛭SETTINGS⛭") // todo: add auto-generation settings and unfifcate "function"
        .open(&mut *settings_open)
        .title_bar(true)
        .collapsible(false)
        .hscroll(true)
        .vscroll(true)
        .resizable(true)
        .constrain(true)
        .default_pos(center)
        .show(ctx, |ui| {
            ui.set_style(style.clone());
            let mut newstyle = (*ctx.style()).clone();
            newstyle.text_styles = [
                    (egui::TextStyle::Body, FontId::new(20.0, egui::FontFamily::Monospace)),
                    (egui::TextStyle::Button, FontId::new(20.0, egui::FontFamily::Monospace))
            ].into();
            ui.style_mut().text_styles = newstyle.text_styles;
            egui::CollapsingHeader::new("⏺ [ GRAPHICS SETTINGS ]")
                .default_open(true)
                .show(ui, |ui| {
                    let prev_dd = settings.deband_dither;
                    egui::ComboBox::from_label("Deband dither")
                        .selected_text(format!("{}", match settings.deband_dither { // XD
                            DebandDither::Enabled => {"Enabled"},
                            DebandDither::Disabled => {"Disabled"}
                        }))
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut settings.deband_dither, DebandDither::Enabled, "Enabled");
                            ui.selectable_value(&mut settings.deband_dither, DebandDither::Disabled, "Disabled");
                        }
                    );
                    let prev_tm = settings.tonemapping;
                    egui::ComboBox::from_label("Tonemapping")
                        .selected_text(format!("{:?}", settings.tonemapping))
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut settings.tonemapping, Tonemapping::None, "None");
                            ui.selectable_value(&mut settings.tonemapping, Tonemapping::AcesFitted, "AcesFitted");
                            ui.selectable_value(&mut settings.tonemapping, Tonemapping::AgX, "AgX");
                            ui.selectable_value(&mut settings.tonemapping, Tonemapping::BlenderFilmic, "BlenderFilmic");
                            ui.selectable_value(&mut settings.tonemapping, Tonemapping::Reinhard, "Reinhard");
                            ui.selectable_value(&mut settings.tonemapping, Tonemapping::ReinhardLuminance, "ReinhardLuminance");
                            ui.selectable_value(&mut settings.tonemapping, Tonemapping::SomewhatBoringDisplayTransform, "SomewhatBoringDisplayTransform");
                            ui.selectable_value(&mut settings.tonemapping, Tonemapping::TonyMcMapface, "TonyMcMapface");
                        }
                    );
                    let prev_cm = settings.composite_mode;
                    egui::ComboBox::from_label("Composite Mode")
                        .selected_text(format!("{}", match settings.composite_mode { // XD
                            BloomCompositeMode::Additive => {"Additive"},
                            BloomCompositeMode::EnergyConserving => {"EnergyConserving"}
                        }))
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut settings.composite_mode, BloomCompositeMode::Additive, "Additive");
                            ui.selectable_value(&mut settings.composite_mode, BloomCompositeMode::EnergyConserving, "EnergyConserving");
                        }
                    );
                    let bi = ui.add(egui::Slider::new(&mut settings.bloom_intensity , 0.0..= 1.0).text("Bloom intensity"));
                    let lfb = ui.add(egui::Slider::new(&mut settings.low_frequency_boost , 0.0..= 1.0).text("Bloom low frequency boost"));
                    let lfbc = ui.add(egui::Slider::new(&mut settings.low_frequency_boost_curvature , 0.0..= 1.0).text("Bloom low frequency boost curvature"));
                    let hpf = ui.add(egui::Slider::new(&mut settings.high_pass_frequency , 0.0..= 1.0).text("Bloom high pass frequency"));
                    let th = ui.add(egui::Slider::new(&mut settings.threshold, 0.0..= 1.0).text("Bloom threshold"));
                    let ths = ui.add(egui::Slider::new(&mut settings.threshold_softness , 0.0..= 1.0).text("Bloom threshold softness"));
                    if prev_dd != settings.deband_dither {writer.send(ApplyCameraSettings::DebandDither)};
                    if prev_tm != settings.tonemapping {writer.send(ApplyCameraSettings::Tonemapping)};
                    if prev_cm != settings.composite_mode {writer.send(ApplyCameraSettings::BloomCompositeMode)};
                    if bi.changed(){writer.send(ApplyCameraSettings::Intensity)};
                    if lfb.changed(){writer.send(ApplyCameraSettings::LowFrequencyBoost)};
                    if lfbc.changed(){writer.send(ApplyCameraSettings::LowFrequencyBoostCurvature)};
                    if hpf.changed(){writer.send(ApplyCameraSettings::HighPassFrequency)};
                    if th.changed(){writer.send(ApplyCameraSettings::Threshold)};
                    if ths.changed(){writer.send(ApplyCameraSettings::ThresholdSoftness)};
            });
    });
    if prev_open != *settings_open{ // handle for closing trough " X " in right up corner of window
        *esc_open = true;
    }
}