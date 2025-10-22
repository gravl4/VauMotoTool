use app::app::AppState;
use eframe::egui;
//use eframe::egui::mutex::RwLock;
//use eframe::egui::Color32;
//use eframe::egui::Vec2b;
//use egui::RichText;
//use egui_plot::Cursor;
//use egui_plot::PlotBounds;
//use crate::egui::ImageSource;
//use crate::egui::TextureOptions;
//use crate::egui::ColorImage;
//use eframe::egui::{CentralPanel, Visuals};
//use crate::egui::Area;
//use eframe::CreationContext;

//use egui_plot::{Legend, Line, Plot, PlotPoints,Corner};
use rtmap::rtmap::Rtmap;
//use egui_plot::Corner::LeftTop;
//use rand::Rng;
use std::sync::{RwLock, Arc}; //, Mutex
//use std::error::Error;
//use std::iter::Iterator;
//use std::mem;
use std::include_bytes; 

//use create::rtmap::Rtmap;
pub mod rtmap; 
pub mod app; 
pub mod udp; 
pub mod app_plot;
pub mod compressor;
//use plotters::drawing::IntoDrawingArea;

use chrono::Local; //, DateTime,}


fn main() {
    /*let world:i32; // = 0x000000FF;
    let bytes = [255, 255, 255, 254];
    world = i32::from_be_bytes(bytes);  
    println!("bytes {}", world);*/
    
    /*let state = Arc::new(Mutex::new(State::new()));
    let native_options = eframe::NativeOptions::default();

    //dummy data
    let dt = Local::now();
    let mut ttimestamp: i64 = dt.timestamp_millis()-500*300;
    let mut realmap: Vec<RealPoint> = Vec::new();
    for _i in 0..300 {
         realmap.push(RealPoint {
            nan: false,
            timestamp: ttimestamp,
            chanel1: ((ttimestamp as f64)/1000.0).sin()*5.0, 
            chanel2: ((ttimestamp as f64)/1000.0).cos()*6.0,
        });
        ttimestamp += 500;
    }
    
    let mrealmap = Arc::new(Mutex::new(realmap));

    let _= eframe::run_native(
        "eframe template",
        native_options,
        Box::new(|cc| Ok(Box::new(App::new(cc, state, mrealmap)))),
    );
*/
    //создаем начальное заполнение rtmap чтобы график двигался сразу а не заполнялся а потом двигался
    let ttimestamp: i64 = Local::now().timestamp_micros();
    let screentime = app::app::APPXAXISTIME.get_time_micros();
    let rtmap = Rtmap::new(100, ttimestamp, screentime/10000); 
    let lrtmap = Arc::new(RwLock::new(rtmap));

    let native_options = eframe::NativeOptions::default();
    // создаем начальное состояние настроек каналов 
    let appstate = Arc::new(RwLock::new(AppState::new(true)));
    // запуск графического проложения
    let _= eframe::run_native(
        "www.vauag.ru   VauMotoTool   www.vauag.com",
        native_options,
        Box::new(|cc| Ok(Box::new(
            app::app::Appx::new(cc, appstate, lrtmap)))),
    );
    
}

