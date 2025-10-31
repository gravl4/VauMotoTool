#[allow(dead_code)] // отключаем предупреждение о неиспользуемом коде
pub mod app {

use eframe::egui::{self, vec2};
use eframe::egui::{RichText, Color32, Modal, Id};

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::mpsc::{self, TryRecvError};
use std::ops::RangeInclusive;
use std::sync::{ RwLock, Arc}; //, Mutex
//use std::fs::File;
//use std::io::prelude::*;
use std::io::Write;
use rayon::prelude::*;
//use chrono::DateTime; //Local, 

//use crate::rtmap;
use crate::rtmap::rtmap::{RtPoint, Rtmap, ACHANELS, DCHANELS, ConvertReduct}; 
use crate::app_plot::app_plot::{chartplot, RTmapsw, SwapMod};
use crate::udp::udp::{DEF_UDP_PORT, UdpProc}; 
/// Параметры настроек каналов
///
/// AChanelName {Color} {converter} {reduction} [x] chart  {reduction2} [x] chart 2
///
/// DChanelName {Color} {dconverter} [x] dchart
/// 
/// AppSettings настройки программы и коэф. пересчета ConvertReduct сохраняются в файле настроек. 
/// Файл осциллограмм содержит  AppSettings + ConvertReduct + Rtmap
/// Перезапись AppSettings + ConvertReduct при открытии файла осциллограмм, после подтверждения. 
/// 
/// 
/// 
/// 

// время через которое обновляется экран 
pub const APPDURATION_MS: usize = 50;  // default
pub const NORTDURATION_MS: usize = 2000; // no real time duration 
// время развертки по Х по умолчанию 
pub const APPXAXISTIME: XAxisTime = XAxisTime::Minutes5; 
pub const MAX_POINTS: usize = 5*1000000; //1c на 1МГц  1мин на 1МГц - 60млн, 20минут на 1МГц - 1,2млрд

    #[derive(Debug, PartialEq)]
    pub enum ViewApp {
        OneAChart, // один аналоговыый график + пара каналов D
        DualAChart, // два аналоговых графика + пара каналов D
        MixChart, // один A и D графики
        DualMixChart, // два анлоговых и D графики
    }

    // Варианты разверки при enablert = true , включенной записи ослиллограмм
    #[derive(Debug, PartialEq)]
    pub enum XAxisTime { //разверка по оси X на весь экран
        Seconds10, 
        Seconds30,
        Minute,
        Minutes5, 
        Minutes10,
        Minutes20, 
    }
    impl XAxisTime {
        pub fn get_time_micros(&self)->i64 {
            let sc: i64 = 1000000; 
            match self {
                Self::Seconds10 => return 10*sc, 
                Self::Seconds30 => return 30*sc,
                Self::Minute  => return 60*sc,
                Self::Minutes5 => return 5*60*sc,
                Self::Minutes10 => return 10*60*sc,
                Self::Minutes20 => return 20*60*sc,
            }
        }
        pub fn get_early_timestamp(&self, lasttime:i64) ->i64 {
            let earlytime:i64;
            match self {
                XAxisTime::Seconds10 => { earlytime = lasttime-10*1000000; }, 
                XAxisTime::Seconds30 => { earlytime = lasttime-30*1000000; },
                XAxisTime::Minute => { earlytime = lasttime-60*1000000; },
                XAxisTime:: Minutes5 => { earlytime = lasttime-5*60*1000000; }, 
                XAxisTime::Minutes10 => { earlytime = lasttime-10*60*1000000; },
                XAxisTime::Minutes20 => { earlytime = lasttime-20*60*1000000; }, 
            };
            earlytime
        }
    }

    pub struct AChanelSettings {
        pub name: String, 
        pub color: Color32, 
        pub viewinchart: bool, 
        pub viewinchart2: bool,
    }
    impl AChanelSettings {
        pub fn new(name: String, color: Color32)->Self {
            AChanelSettings {
                name, 
                color, 
                viewinchart: true, 
                viewinchart2: true, 
            }
        }
    }

    pub struct DChanelSettings {
        pub name: String, 
        pub color: Color32, 
        pub viewindchart: bool,
    }
    impl DChanelSettings {
        pub fn new(name: String, color: Color32)->Self {
            DChanelSettings {
                name, 
                color, 
                viewindchart: true, 
            }
        }
    }

    pub struct AppSettings {
        add_d: bool, // добавить к аналоговым два d канала
        add_dchanles: [Option<usize>; 2], // номера каналов которые выводим дополнительно к аналоговым
        pub viewapp: ViewApp, // сколько графиков показывать и как
        pub axittime: XAxisTime, 
        pub udp_port: usize, 
        // сохраняются в файле настроек:  
        duration: usize, 
        pub achanels: Vec<AChanelSettings>, // настройки каналов Analog
        pub dchanels: Vec<DChanelSettings>, // настройки каналов Digital
        pub bignum: usize,
    }
    impl AppSettings {
        pub fn new()-> Self {
            let achanels = (0..ACHANELS).map(|x| {
                let name = format!("Chanel {}", x+11); 
                let mut r:u8 = 255;
                let mut g:u8 = 255;
                let mut b:u8 = 255; 
                if x < 5 {
                    g = g-100-10*(x as u8);
                    b = b-100-10*(x as u8);
                } else if x < 10 && x >= 5 {
                    r = r-100-10*((x as u8)-5);
                    b = b-100-10*((x as u8)-5);        
                } else {
                    r = r-100-10*((x as u8)-10);
                    g = g-100-10*((x as u8)-10);
                }

                let color = Color32::from_rgb(r, g, b);
                AChanelSettings::new(name, color)
            }).collect(); 
            let dchanels = (0..DCHANELS).map(|x| {
                let name = format!("Chanel {}", x+11); 
                let mut r:u8 = 255;
                let mut g:u8 = 255;
                let mut b:u8 = 255; 
                if x < 10 {
                    g = g-50-10*(x as u8);
                    b = b-50-10*(x as u8);
                } else if x < 20 && x >= 10 {
                    r = r-50-10*((x as u8)-10);
                    b = b-50-10*((x as u8)-10);        
                } else {
                    r = r-50-10*((x as u8)-20);
                    g = g-50-10*((x as u8)-20);
                }

                let color = Color32::from_rgb(r, g, b);
                DChanelSettings::new(name,  color)
            }).collect(); 
            AppSettings {
                add_d: true, 
                add_dchanles: [None, None],
                viewapp: ViewApp::DualAChart,
                axittime: XAxisTime::Minute,
                udp_port: DEF_UDP_PORT,
                duration: APPDURATION_MS, 
                achanels, 
                dchanels,
                bignum: 0,
            }
        }
    }

    pub struct AppState{
        ctx: Option<egui::Context>,
        duration: usize, 
        rtrender: bool, 
        rtdemo: bool, 
    }
    impl AppState {
        pub fn new(rtrender: bool) -> Self {
            Self {
                ctx: None,
                duration: APPDURATION_MS,
                rtrender, 
                rtdemo: false, 
            }
        }
        pub fn get_rt(&self)->bool {
            self.rtrender
        }
        pub fn set_rt(& mut self, new_rt: bool)->bool {
            self.rtrender = new_rt; 
            self.rtrender
        }
        pub fn get_rtdemo(&self)->bool {
            self.rtdemo
        }
        pub fn set_rtdemo(& mut self, new_rtdemo: bool)->bool {
            self.rtdemo = new_rtdemo; 
            self.rtdemo
        }
        pub fn get_duration(&self)->usize {
            self.duration 
        }  
        pub fn set_duration(& mut self, duration: usize) {
            self.duration = duration; 
        }       
    }

    /// Пустой процесс который вызывает периодическое обновление egui
    /// графики движутся в реальном времени 
    fn slow_process(state_clone: Arc<RwLock<AppState>>) {
        loop {
            let duration:u64;
            {
                let appstate = &state_clone.read().unwrap();
                duration = appstate.duration as u64;
                match &appstate.ctx {
                    Some(x) => 
                        if appstate.rtrender { x.request_repaint() },
                    None => panic!("error in Option<>"),
                }
            }
            std::thread::sleep(std::time::Duration::from_millis(duration));
        }
    }
    
    pub struct Appx {
        window_open: bool, // для окна настроек
        message_modal_open: bool,
        message_modal: String, 
        settings_path: Option<String>, // файл настроек 
       // scope_path: Option<String>, // файл сохранения осциллограммы
        pub appstate: Arc<RwLock<AppState>>, // принудительный процесс обновления 
        pub rtmap: Arc<RwLock<Rtmap>>, // главные массивы данных 
        pub pcursor: Option<egui_plot::PlotPoint>, // где находится курсор
        pub pcursor_avalues: Vec<f64>, // мгновенные значения каналов 
        //pub enablert: Arc<RwLock<bool>>, // разрешить запись 
        pub appsettings: AppSettings, // настройки программы. сохраняются в файле настроек вместе ConvertReduct из rtmap
        prind: Option<usize>, // предыддуший индекс мгновенного значения
        pub udp_rx: mpsc::Receiver<Result<Vec<RtPoint>, Box<dyn std::error::Error + Send + Sync>>>,
        pub xformatter: bool, 
        pub mapsw: Rc<RefCell<RTmapsw>>,
        pub mode_swap: SwapMod, 
        pub abounds: u8, 
        pub csvtimestamp1: Option<i64>,
        pub csvtimestamp2: Option<i64>,
    } 

    impl Appx {
        pub fn new(cc: &eframe::CreationContext<'_>, appstate: Arc<RwLock<AppState>>, rtmap: Arc<RwLock<Rtmap>>) -> Self {
            //let state = Arc::new(Mutex::new(State::new()));
            appstate.write().unwrap().ctx = Some(cc.egui_ctx.clone());
            let state_clone = appstate.clone();
            let pcursor_avalues = (0..ACHANELS).map(|_x|{
                0.0 as f64
            }).collect();

            //let normal = egui::Vec2::new(10.0, 10.0).normalized().rot90(); 
            //println!("{}, {}", normal[0], normal[1]);
            
            // при создании appx создаем udp процесс
            let mut appsettings = AppSettings::new(); 
            let (port_tx, port_rx) = mpsc::channel::<Result<Vec<RtPoint>, Box<dyn std::error::Error + Send + Sync>>>();
            
            /*use chrono::Local; 
            let ttimestamp: i64 = Local::now().timestamp_micros();
            let screentime = APPXAXISTIME.get_time_micros();
            let mut rtmap2 = Rtmap::new(100, ttimestamp, screentime/100); 
            rtmap2.rtmap_calc_all();
            let lrtmap2 = Arc::new(RwLock::new(rtmap2));
            UdpProc::new_process(appsettings.udp_port, appstate.clone(), lrtmap2.clone(), port_tx.clone());*/

            UdpProc::new_process(appsettings.udp_port, appstate.clone(), port_tx.clone());
            let mapsw =  Rc::new(RefCell::new(RTmapsw::new()));

           // let strpath = "../define_settings";
            let rpath = std::env::current_exe(); 
            if rpath.is_ok() {
                let mut defpath = rpath.unwrap();//.as_os_str(); // "./define_settings";
                let mpath = defpath.as_mut_os_str();
                let mut spath = mpath.to_str().unwrap().to_string();
                spath.push_str("_define_settings"); 
           // if let Some(x) = strpath {
                let data = std::fs::read(spath);
                if data.is_ok() {
                    let _res= read_settings_from_file(data.unwrap(), &mut appsettings, 
                    rtmap.clone(), &mut mapsw.borrow_mut().convertrd);
                    let duration = appsettings.duration;
                    {
                        //appx.appstate.write().unwrap().duration = duration;
                        appstate.write().unwrap().set_duration(duration);
                    }                           
                }
         //   }
            }

            std::thread::spawn(move || {
                slow_process(state_clone);
            });
            Self {
                window_open: false,
                message_modal_open: false,
                message_modal: " ".to_string(), 
                settings_path: None,
         //       scope_path: None, 
                appstate: appstate.clone(), 
                rtmap: rtmap.clone(), 
                pcursor: None, 
                pcursor_avalues, 
              //  enablert: Arc::new(RwLock::new(true)), 
                appsettings, 
                prind: None, 
                udp_rx: port_rx, 
                xformatter: false, 
                mapsw,
                mode_swap: SwapMod::Full, 
                abounds: 1, 
                csvtimestamp1: None,
                csvtimestamp2: None,
            }
        }
    }

// основное окно 
    impl eframe::App for Appx {
        fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {

            let choke = self.udp_rx.try_recv();
            if let Err(che) = choke {
                if che == TryRecvError::Disconnected {
                    //println!("Error from udp thread lose");
                    self.appstate.write().unwrap().set_rt(false); 
                    // потока больше нет
                }
                // поток работает нормально che == TryRecvError::Empty     
            }
            else {
                //let chok = choke.unwrap().err().unwrap(); 
                //println!("Error msg from udp thread");
                if let Ok(points) = choke.unwrap() {
                    // забираем точки данных 
                    let mut rtwr = self.rtmap.write().unwrap();    
                    if self.appstate.read().unwrap().get_rt() {  
                        points.iter().for_each(|x| {
                        // println!("timestamp {}", x.timestamp);
                            rtwr.rtmap_push(x.clone());        
                        });
                    }
                    if rtwr.rtmap.len() > MAX_POINTS {
                        self.appstate.write().unwrap().set_rt(false); 
                    }
                }
                else {
                    // сообщение об ошибке из потока
                    self.message_modal_open = true;
                    self.message_modal = "UDP port fault".to_string(); 
                    self.appstate.write().unwrap().set_rt(false); 
                }
            }
            

            /*egui::CentralPanel::default().show(ctx, |ui| {
                ui.label(format!("woke up after {}ms", self.state.lock().unwrap().duration));
            });*/
            if self.window_open {
                file_window(self, ctx);
            }
            else { 

            }

            if self.message_modal_open {
                let modal = Modal::new(Id::new("Modal A")).show(ctx, |ui| {
                    ui.set_width(250.0);
                    ui.label(format!("Message: {}", self.message_modal));
                    ui.separator();

                        ui.vertical_centered(|ui| {
                            if ui.button("Cancel").clicked() {
                                self.message_modal_open = false;
                            }
                        });
                });
    
                if modal.should_close() {
                    self.message_modal_open = false;
                }
            }
    
            //let imgbytes = include_bytes!("vaulogo_g.png");
        

            egui::SidePanel::left("my_left_panel")
                .resizable(false)
                .default_width(150.0)
                .min_width(150.0)
            .show(ctx, |ui| {
                //ui.set_enabled(!self.window_open);

               // let mut texture = ui.ctx().load_texture("name", egui::ColorImage::example(), Default::default());
                /*let image = image::load_from_memory(imgbytes);
                if image.is_ok() {
                    let image = image.unwrap(); 
                    let size: [_; 2] = [image.width() as usize, image.height() as usize];
                    let image_buffer = image.to_rgba8(); 
                    let pixels = image_buffer.as_flat_samples();
                    let cimage = egui::ColorImage::from_rgba_unmultiplied(
                        size,
                        pixels.as_slice(),
                    );
                   // texture = ui.ctx().load_texture("name", cimage, Default::default());
                } 

                
                let stexture = egui::load::SizedTexture{ id: texture.id(), size: egui::vec2(64.0, 64.0) };
                ui.horizontal(|ui| {
                    ui.add(
                        egui::Image::new(egui::ImageSource::Texture( stexture))
                    );
                    ui.vertical(|ui| {
                        ui.label(format!(" "));
                        ui.label(format!("vauag.ru"));
                        ui.label(format!("vauag.com"));
                    });
                });*/


                

                ui.horizontal(|ui| {
                    if ui.button("File").clicked() {
                        self.window_open = true;
                    }
                    if ui.button("ABounds").clicked() {
                        self.abounds = 1; 
                    }
                    if ui.button("Clean").clicked() {
                        //self.appstate.write().unwrap().set_rt(false);
                        // При запуске real time, rendering надо очистить 
                        use chrono::Local; //, DateTime,}
                        let ttimestamp: i64 = Local::now().timestamp_micros();
                        let screentime = APPXAXISTIME.get_time_micros();
                        let free_rtmap = Rtmap::new(100, ttimestamp, screentime/10000); 
                        //free_rtmap.rtmap_calc_all();
                        *self.rtmap.write().unwrap() = free_rtmap;  
                        self.mode_swap = SwapMod::Full;   
                        self.abounds = 1;      
 
                    }
                });
                // включение записи RealTimeqAA
                {
                    let mut check: bool;
                    let checkpr:bool;
                    {
                        check = self.appstate.read().unwrap().get_rt();
                        checkpr = check; 
                    }
                    if ui.checkbox( &mut check, "RealTime").clicked() {
                        if check != checkpr {
                            self.appstate.write().unwrap().set_rt(check); 
                            self.mapsw.borrow_mut().run = check;
                            if check == true {
                                // при создании appx создаем udp процесс
                                let appsettings = AppSettings::new(); 
                                let (port_tx, port_rx) = mpsc::channel::<Result<Vec<RtPoint>, Box<dyn std::error::Error + Send + Sync>>>();
                                UdpProc::new_process(appsettings.udp_port, self.appstate.clone(), port_tx.clone());
                                self.udp_rx = port_rx;

                                self.appstate.write().unwrap().set_duration(self.appsettings.duration); 
                                let firsttime = self.rtmap.read().unwrap().rtmap.first().unwrap().timestamp;
                                let lasttime = self.rtmap.read().unwrap().rtmap.last().unwrap().timestamp;
                                if (lasttime-firsttime) > 20*60*1000000 {
                                    // При запуске real time, rendering надо очистить 
                                    use chrono::Local; //, DateTime,}
                                    let ttimestamp: i64 = Local::now().timestamp_micros();
                                    let screentime = APPXAXISTIME.get_time_micros();
                                    let free_rtmap = Rtmap::new(100, ttimestamp, screentime/10000);
                                    //free_rtmap.rtmap_calc_all();
                                    *self.rtmap.write().unwrap() = free_rtmap;  
                                    self.mode_swap = SwapMod::Full;     
                                }            
                            }
                            else {
                                self.appstate.write().unwrap().set_duration(NORTDURATION_MS); 
                              //  self.mode_swap = SwapMod::Full;   
                            }
                        }
                    };
                    if ui.checkbox( &mut self.xformatter, "X-Formatter").clicked() { }

                }
                {
                    /*if self.csvtimestamp1.is_some() {
                        let csvstart = self.csvtimestamp1.unwrap(); 
                        ui.label(format!("cst_s {}", csvstart));
                    }
                    else {
                        ui.label(format!("cst_s none"));
                    }
                    if self.csvtimestamp2.is_some() {
                        let csvstop = self.csvtimestamp2.unwrap(); 
                        ui.label(format!("cst_st {}", csvstop));
                    }
                    else {
                        ui.label(format!("cst_ts none"));
                    }*/
                //let rtlen = self.rtmap.read().unwrap().rtmap.len(); 
                //let swlen = self.mapsw.borrow().rtswap.len();
                //ui.label(format!("rt {}", rtlen));
                //ui.label(format!("sw {}", swlen));
                
                //if ui.button("test but").clicked() {
                //}

                /*
                let achlen = self.mapsw.borrow().gachanels[0].ppoints.len(); 
                let ach2len = self.mapsw.borrow().gachanels2[0].ppoints.len(); 
                let dchlen = self.mapsw.borrow().gdchanels[0].ppoints.len(); 
                ui.label(format!("ach {}", achlen));
                ui.label(format!("ach2 {}", ach2len));
                ui.label(format!("dch {}", dchlen));
                if ui.button("LOGG").clicked() {
                    let arr = self.mapsw.borrow().gachanels[1].ppoints.clone(); 
                    let file =  std::fs::File::create("/home/ygy1/RUST/logout");
                    if file.is_ok() {
                        let mut file = file.unwrap(); 
                        for i in 1..arr.len()-1 {
                            //if arr[i].x == arr[i-1].x {
                                let mm = format!("ind {} ga1 {} delta {}\r\n", i, arr[i].x, arr[i].x-arr[i-1].x); 
                                _= file.write(mm.as_bytes());
                                //println!("ind {} ga1 {} delta {}", i, arr[i].x, arr[i].x-arr[i-1].x);
                            //}
                        }            
                    }
                }*/
                
               // use chrono::Local; //, DateTime,}
              //  let mtimestamp: i64 = Local::now().timestamp_millis();
              //  println!("millis time: {}", mtimestamp);    
                }
                ui.separator();
                // вывод мгновенных значений

                if self.appsettings.bignum > 0 {
                    let num = self.appsettings.bignum-1;
                    let conv = self.mapsw.borrow().convertrd.aconverter[num];
                    let name = self.appsettings.achanels[num].name.clone(); 
                    let color = self.appsettings.achanels[num].color;
                    if self.appstate.read().unwrap().get_rt() == true {
                        if let Some(rtp) = self.rtmap.read().unwrap().rtmap.last() {
                            //if let Ok(pp) = self.mapsw.borrow().get_insta(&self.rtmap.read().unwrap().rtmap, tm) {
                            //}
                            let val = rtp.achanels[num]; 
                            ui.label(RichText::new(format!("{}", name)).color(color) );
                            ui.label(RichText::new(format!("{:.3}", (val as f64)*conv)).color(color).size(30.0));
                        }
                    }
                    //else {

                    //}
                }
                
                if self.pcursor.is_some() && self.appstate.read().unwrap().get_rt() == false {
                    let xtimestamp = self.pcursor.unwrap().x as i64; 

                    if let Ok(pp) = self.mapsw.borrow().get_insta(&self.rtmap.read().unwrap().rtmap, xtimestamp) {
                        if self.appsettings.bignum > 0 {
                            let num = self.appsettings.bignum-1;
                           // let conv = self.mapsw.borrow().convertrd.aconverter[num];
                            let name = self.appsettings.achanels[num].name.clone(); 
                            let color = self.appsettings.achanels[num].color;     
                            let val = pp.0[num];  
                            ui.label(RichText::new(format!("{}", name)).color(color) );
                            ui.label(RichText::new(format!("{:.3}", val)).color(color).size(30.0));                      
                        }
                        pp.0.iter().enumerate().for_each(|x| {
                            if self.appsettings.achanels[x.0].viewinchart || self.appsettings.achanels[x.0].viewinchart2 {
                                let name = self.appsettings.achanels[x.0].name.clone(); 
                                let color = self.appsettings.achanels[x.0].color; 
                                ui.label(RichText::new(format!("{} :{:.3}", name, x.1)).color(color) );
                            }                           
                        });
                        ui.separator();
                        pp.1.iter().enumerate().for_each(|x| {
                            if self.appsettings.dchanels[x.0].viewindchart {
                                let name = self.appsettings.dchanels[x.0].name.clone(); 
                                let color = self.appsettings.dchanels[x.0].color; 
                                //ui.label(RichText::new(format!("{} :{:.3}", name, x.1)).color(color) );
                                if *x.1 == 0 {
                                    ui.label(RichText::new(format!("{}: 0", name)).color(color) );
                                }
                                else {
                                    ui.label(RichText::new(format!("{}: ---1", name)).color(color) );
                                }
                            }                           
                        });
                    }
                   // ui.label(format!("Timestamp: {}", xtimestamp));
                    /*if let Ok(tvalues) = self.rtmap.read().unwrap().rtmap_get_ys_for_x(xtimestamp, self.prind)
                    {
                        tvalues.0.achanels.iter().enumerate().for_each(|x| {
                            if self.appsettings.achanels[x.0].viewinchart || self.appsettings.achanels[x.0].viewinchart2 {
                                let name = self.appsettings.achanels[x.0].name.clone(); 
                                let color = self.appsettings.achanels[x.0].color;
                                let mut aval = self.rtmap.read().unwrap().rtmap_calc_achanel_point(&tvalues.0, x.0, 0);
                                let areduct = self.rtmap.read().unwrap().convertrd.areduction[x.0]; 
                                aval = aval/areduct; // получаем физическую величину
                                ui.label(RichText::new(format!("{} :{:.3}", name, aval)).color(color) );
                            }
                        });
                        ui.separator();
                        //ui.label("D chanels");
                        if self.appstate.read().unwrap().get_rt() == false  {
                        // d каналы 
                        let dvals = self.rtmap.read().unwrap().rtmap_calc_dchanels_point(&tvalues.0);
                        (0..DCHANELS).for_each(|x| {
                            if self.appsettings.dchanels[x].viewindchart {
                                let name = self.appsettings.dchanels[x].name.clone(); 
                                let color = self.appsettings.dchanels[x].color;
                                let dconvert = self.rtmap.read().unwrap().convertrd.dconverter[x];
                                let dval = dvals[x]-dconvert+1.0; 
                                if dval < 0.5 {
                                    ui.label(RichText::new(format!("{}: 0", name)).color(color) );
                                }
                                else {
                                    ui.label(RichText::new(format!("{}: ---1", name)).color(color) );
                                }
                            }
                        });                       
                        }    
                        
                       // let values = values.unwrap(); 
                      //  self.prind = Some(values.1); 
                    }
                    else {
                        self.prind = None;  
                    }*/
                }                
    
            });
            egui::CentralPanel::default().show(ctx, |ui| {
                egui::ScrollArea::both().show(ui, |ui| { //.vscroll(modctrl)
                    chartplot(self, ui);
                }); 
            });
            //println!(".");
        }
    }

    // окно File
    fn file_window(appx: &mut Appx, ctx: &egui::Context) {
        egui::Window::new("File")
        .open(&mut appx.window_open)
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.button("Open scope").clicked() {
                    {
                      //  appx.appstate.write().unwrap().rtrender = false;
                       appx.appstate.write().unwrap().set_rt(false);
                       appx.mode_swap = SwapMod::Full;
                    }
                    if let Some(path) = rfd::FileDialog::new().pick_file() {
                        let strpath = path.to_str();
                        if let Some(x) = strpath {
                            let data = std::fs::read(x);
                            if data.is_ok() {
                                let res= read_scope_from_file(data.unwrap(), &mut appx.appsettings, 
                                appx.rtmap.clone(), &mut appx.mapsw.borrow_mut().convertrd);
                                let duration = appx.appsettings.duration;
                                {
                                   // appx.appstate.write().unwrap().duration = duration;
                                   appx.appstate.write().unwrap().set_duration(duration);
                                }
                                if res.is_err() {
                                    // выводим ошибку парсера
                                    let er = res.err().unwrap().to_string(); 
                                    appx.message_modal = er;
                                    appx.message_modal_open = true;   
                                }  
                                else {
                                   // appx.rtmap.write().unwrap().rtmap_calc_all();
                                   appx.mode_swap = SwapMod::Full;
                                }                        
                            }
                            else {
                                // ошибка открытия файла 
                                appx.message_modal = "Can not open file".to_string();
                                appx.message_modal_open = true;                                 
                            }
                        }
                    }
                    else {
                        // нет пути 
                        appx.message_modal = "Path unknown".to_string();
                        appx.message_modal_open = true; 
                    }                          
                }
                if ui.button("Save scope as").clicked() {
                    if let Some(path) = rfd::FileDialog::new().save_file() {
                        //let test = format!("sdfsd \r\n sdfsdf \r\n 363564563\r\n"); 
                        let strpath = path.to_str();
                        if let Some(x) = strpath {
                            let file =  std::fs::File::create(x);
                            if file.is_ok() {
                                let res = write_scope_to_file(&mut file.unwrap(), &appx.appsettings, 
                                appx.rtmap.clone(),&mut appx.mapsw.borrow_mut().convertrd);
                                if res.is_err() {
                                    // выводим ошибку сохранения
                                    let er = res.err().unwrap().to_string(); 
                                    appx.message_modal = er;
                                    appx.message_modal_open = true;   
                                }
                            }
                        }
                    }    
                    else {
                        // нет пути 
                        appx.message_modal = "Path unknown".to_string();
                        appx.message_modal_open = true; 
                    }                    
                }
                ui.separator();           
                if ui.button("Save CSV").clicked() {
                    if appx.csvtimestamp1.is_some() && appx.csvtimestamp2.is_some() { 
                        if let Some(path) = rfd::FileDialog::new().save_file() {
                            //let test = format!("sdfsd \r\n sdfsdf \r\n 363564563\r\n"); 
                            let strpath = path.to_str();
                            if let Some(x) = strpath {
                                let f1 = format!("{}_1.csv", x);
                                let f2 = format!("{}_2.csv", x); 
                                let file =  std::fs::File::create(f1.as_str());
                                let file2 =  std::fs::File::create(f2.as_str());
                                if file.is_ok() {
                                    let res = write_csv_file(&mut file.unwrap(),&mut file2.unwrap(), &appx.appsettings, 
                                    appx.rtmap.clone(),&mut appx.mapsw.borrow_mut().convertrd, appx.csvtimestamp1, appx.csvtimestamp2);
                                    if res.is_err() {
                                        // выводим ошибку сохранения
                                        let er = res.err().unwrap().to_string(); 
                                        appx.message_modal = er;
                                        appx.message_modal_open = true;   
                                    }
                                }
                            }
                        }    
                        else {
                            // нет пути 
                            appx.message_modal = "Path unknown".to_string();
                            appx.message_modal_open = true; 
                        }      
                    }     
                    else {
                        appx.message_modal = "Click C and V for set CSV window".to_string();
                        appx.message_modal_open = true; 
                    }         
                }  
                if appx.csvtimestamp1.is_some() {
                    let csvstart = appx.csvtimestamp1.unwrap(); 
                    ui.label(format!("start {}", csvstart));
                }
                else {
                    ui.label(format!("start none"));
                }
                if appx.csvtimestamp2.is_some() {
                    let csvstop = appx.csvtimestamp2.unwrap(); 
                    ui.label(format!("stop {}", csvstop));
                }
                else {
                    ui.label(format!("stop none"));
                }                           
            });
            //let mut pathscope = "Settings: default".to_string();
            //if self.scope_path.is_some() {
            //    let vv =  self.scope_path.clone();
            //    pathscope = format!("Scope: {}", vv.unwrap());
            //}
            //ui.label(pathscope);  
            ui.separator();
            ui.horizontal(|ui| {
                if ui.button("Open settings").clicked() {
                    if let Some(path) = rfd::FileDialog::new().pick_file() {
                        let strpath = path.to_str();
                        if let Some(x) = strpath {
                            let data = std::fs::read(x);
                            if data.is_ok() {
                                let res= read_settings_from_file(data.unwrap(), &mut appx.appsettings, 
                                appx.rtmap.clone(), &mut appx.mapsw.borrow_mut().convertrd);
                                let duration = appx.appsettings.duration;
                                {
                                    //appx.appstate.write().unwrap().duration = duration;
                                    appx.appstate.write().unwrap().set_duration(duration);
                                }
                                if res.is_err() {
                                    // выводим ошибку парсера
                                    let er = res.err().unwrap().to_string(); 
                                    appx.message_modal = er;
                                    appx.message_modal_open = true;   
                                }
                                else {
                                    // сохраняем путь настроек
                                    let strpath = path.to_str().unwrap().to_string();
                                    appx.settings_path = Some(strpath); 
                                   // appx.rtmap.write().unwrap().rtmap_calc_all();
                                    appx.mode_swap = SwapMod::Full;
                                }                             
                            }
                            else {
                                // ошибка открытия файла 
                                appx.message_modal = "Can not open file".to_string();
                                appx.message_modal_open = true;                                 
                            }
                        }
                    }
                    else {
                        // нет пути 
                        appx.message_modal = "Path unknown".to_string();
                        appx.message_modal_open = true; 
                    }
                }
                let mut opensave = false; 
                if ui.button("Save settings").clicked() {
                    if appx.settings_path.is_some() {
                        let path = appx.settings_path.clone().unwrap().clone(); 
                        let file =  std::fs::File::create(path);
                        if file.is_ok() {
                            let res = write_settings_to_file(&mut file.unwrap(), &appx.appsettings, appx.rtmap.clone(), 
                            &mut appx.mapsw.borrow_mut().convertrd);
                            if res.is_err() {
                                // выводим ошибку сохранения
                                let er = res.err().unwrap().to_string(); 
                                appx.message_modal = er;
                                appx.message_modal_open = true;   
                            }
                        }
                    }
                    else {
                        // нет пути его надо запросить 
                        opensave = true; 
                    }
                }
                if ui.button("Save settings as ").clicked() || opensave {
                    if let Some(path) = rfd::FileDialog::new().save_file() {
                        //let test = format!("sdfsd \r\n sdfsdf \r\n 363564563\r\n"); 
                        let strpath = path.to_str();
                        if let Some(x) = strpath {
                            let file =  std::fs::File::create(x);
                            if file.is_ok() {
                                let res = write_settings_to_file(&mut file.unwrap(), &appx.appsettings, appx.rtmap.clone() ,
                                &mut appx.mapsw.borrow_mut().convertrd);
                                if res.is_err() {
                                    // выводим ошибку сохранения
                                    let er = res.err().unwrap().to_string(); 
                                    appx.message_modal = er;
                                    appx.message_modal_open = true;   
                                }
                                else {
                                    // сохраняем путь настроек
                                    let strpath = path.to_str().unwrap().to_string();
                                    appx.settings_path = Some(strpath);                                     
                                }
                            }
                        }
                    }    
                    else {
                        // нет пути 
                        appx.message_modal = "Path unknown".to_string();
                        appx.message_modal_open = true; 
                    }                
                }
                if ui.button("Save as define").clicked() {
                    let rpath = std::env::current_exe(); 
                    if rpath.is_ok() {
                        let mut defpath = rpath.unwrap();//.as_os_str(); // "./define_settings";
                        let mpath = defpath.as_mut_os_str();
                        let mut spath = mpath.to_str().unwrap().to_string();
                        spath.push_str("_define_settings"); 
                        //let spath = "/home/ygy1/RUST/defsett";
                        //println!("{}", spath); 
                        let file =  std::fs::File::create( spath);
                        if file.is_ok() {
                            let res = write_settings_to_file(&mut file.unwrap(), &appx.appsettings, appx.rtmap.clone() ,
                            &mut appx.mapsw.borrow_mut().convertrd);
                            if res.is_ok() {
                                let messg = "Save define OK"; 
                                appx.message_modal = messg.to_string();
                                appx.message_modal_open = true;   
                            }
                            else {
                                // выводим ошибку сохранения
                                let er = res.err().unwrap().to_string(); 
                                appx.message_modal = er;
                                appx.message_modal_open = true; 
                            }
                        }    
                        else {
                            // выводим ошибку сохранения
                            let er = file.err().unwrap().to_string(); 
                            appx.message_modal = er;
                            appx.message_modal_open = true;    
                        }             
                    }
                }

                ui.separator();
                //ui.horizontal(|ui| {
                ui.label("UDP port:");
                let port = &mut appx.appsettings.udp_port; 
                let prv = port.clone();
                ui.add(egui::DragValue::new(port).speed(0.01).range(RangeInclusive::new(1, 65536)));
                if prv != appx.appsettings.udp_port {
                    // задан новый порт 
                    // даем сигнал на переключение порта 
                }
                let demo = appx.appstate.read().unwrap().get_rtdemo(); 
                let mut demo_box = demo; 
                ui.checkbox( &mut demo_box, "Demo Mode");
                if demo_box != demo {
                    appx.appstate.write().unwrap().set_rtdemo(demo_box); 
                }
               // }); 
            }); 
            //let mut pathsettings = "Settings: default".to_string();
            /*if self.picked_path.is_some() {
                let vv =  self.picked_path.clone();
                pathsettings = format!("Settings: {}", vv.unwrap());
            }*/
            //ui.label(pathsettings); 
            ui.separator();
            ui.horizontal(|ui| {
                if ui.button("Reset reduction").clicked() {
                    //     self.plot1line1max = 1.0;
                    //     self.plot1line2max = 1.0; 
                }   
                ui.label("Charts view:");
                let view = &mut appx.appsettings.viewapp; 
                let _m1 = egui::ComboBox::from_label("1")
                .selected_text(format!("{view:?}"))
                .show_ui(ui, |ui| {
                    ui.selectable_value(view, ViewApp::OneAChart, "One Analog");
                    ui.selectable_value(view, ViewApp::DualAChart, "Dual Analog");
                    ui.selectable_value(view, ViewApp::MixChart, "One Mix");
                    ui.selectable_value(view, ViewApp::DualMixChart, "Dual Mix");
                });
                ui.label("X Axis Time:");
                let view2 = &mut appx.appsettings.axittime; 
                let _m2 = egui::ComboBox::from_label("2")
                .selected_text(format!("{view2:?}"))
                .show_ui(ui, |ui| {
                    ui.selectable_value(view2, XAxisTime::Seconds10, "10 seconds");
                    ui.selectable_value(view2, XAxisTime::Seconds30, "30 seconds");
                    ui.selectable_value(view2, XAxisTime::Minute, "minute");
                    ui.selectable_value(view2, XAxisTime::Minutes5, "5 minutes");
                    ui.selectable_value(view2, XAxisTime::Minutes10, "10 minutes");
                    ui.selectable_value(view2, XAxisTime::Minutes20, "20 minutes");
                });
                ui.label("Duration:");
                ui.add(egui::DragValue::new(&mut appx.appsettings.duration).speed(10)
                    .range(RangeInclusive::new(10, 2000)));
                {
                    let duration = appx.appsettings.duration;
                   // appx.appstate.write().unwrap().duration = duration;
                    appx.appstate.write().unwrap().set_duration(duration);
                }
                ui.separator();
                ui.label("Big view");
                let bignum = &mut appx.appsettings.bignum; 
                ui.add(egui::DragValue::new(bignum).speed(1).range(RangeInclusive::new(0, ACHANELS)));                

            });

            ui.separator();
            ui.label("Conv. - Conversion int to physics value. Reduc. - Reductrion value on chart");
            ui.horizontal(|ui| { 
                //AChanelName {Color} {converter} {reduction} [x] chart  {reduction2} [x] chart 2
                ui.vertical(|ui| {
                    ui.label("Name");         
                    (0..ACHANELS).for_each(|x| { 
                        ui.horizontal(|ui| { 
                            ui.label(format!("{}", x+1));
                            ui.add(egui::TextEdit::singleline(&mut appx.appsettings.achanels[x].name).desired_width(80.0));      
                        });
                    });
                });
                ui.vertical(|ui| {
                    ui.label("Color");
                    (0..ACHANELS).for_each(|x| { 
                        ui.color_edit_button_srgba(&mut appx.appsettings.achanels[x].color);
                    });
                });
                ui.vertical(|ui| {
                    ui.label("Conv.");
                    (0..ACHANELS).for_each(|x| { 
                        let prv = appx.mapsw.borrow().convertrd.aconverter[x]; 
                        ui.add(egui::DragValue::new(&mut appx.mapsw.borrow_mut().convertrd.aconverter[x]).speed(0.0001).range(RangeInclusive::new(0.0001, 100.0)));
                        if prv != appx.mapsw.borrow().convertrd.aconverter[x]{
                           appx.mode_swap = SwapMod::Full;
                        }
                    });
                });
                ui.vertical(|ui| {
                    ui.label("Chart Reduc.");
                    (0..ACHANELS).for_each(|x| { 
                    ui.horizontal(|ui| {
                        ui.checkbox(&mut  appx.appsettings.achanels[x].viewinchart, "");
                        let prv = appx.mapsw.borrow().convertrd.areduction[x]; 
                        ui.add(egui::DragValue::new(&mut appx.mapsw.borrow_mut().convertrd.areduction[x]).speed(0.0001).range(RangeInclusive::new(0.0001, 100.0)));
                        if prv != appx.mapsw.borrow().convertrd.areduction[x]{
                           appx.mode_swap = SwapMod::Full;
                        }
                    });
                    });
                });
                ui.vertical(|ui| {
                    ui.label("Chart2");
                    (0..ACHANELS).for_each(|x| { 
                    ui.horizontal(|ui| {
                        ui.checkbox(&mut  appx.appsettings.achanels[x].viewinchart2, "");
                        let prv = appx.mapsw.borrow().convertrd.areduction2[x]; 
                        ui.add(egui::DragValue::new(&mut appx.mapsw.borrow_mut().convertrd.areduction2[x]).speed(0.0001).range(RangeInclusive::new(0.0001, 100.0)));
                        if prv != appx.mapsw.borrow().convertrd.areduction2[x]{
                           appx.mode_swap = SwapMod::Full;
                        }
                    });
                    });
                });
                ui.separator();
                // DChanelName {Color} {dconverter} [x] dchart
                ui.vertical(|ui| {
                    ui.label("Name");
                    (0..DCHANELS/2).for_each(|x| { 
                        ui.horizontal(|ui| {
                            ui.label(format!("{}", x+1));
                            ui.add(egui::TextEdit::singleline(&mut appx.appsettings.dchanels[x].name).desired_width(80.0));
                        });
                    });
                });
                ui.vertical(|ui| {
                    ui.label("Color");
                    (0..DCHANELS/2).for_each(|x| { 
                        ui.color_edit_button_srgba(&mut appx.appsettings.dchanels[x].color);
                    });
                });
                let recalc = false; 
                ui.vertical(|ui| {
                    ui.label("Conv.");
                    (0..DCHANELS/2).for_each(|x| { 
                        let prv = appx.mapsw.borrow().convertrd.dconverter[x]; 
                        ui.add(egui::DragValue::new(&mut appx.mapsw.borrow_mut().convertrd.dconverter[x]).speed(0.0001).range(RangeInclusive::new(0.0001, 100.0)));
                        if prv != appx.mapsw.borrow().convertrd.dconverter[x]{
                           appx.mode_swap = SwapMod::Full;
                        }
                    });
                });
                ui.vertical(|ui| {
                    ui.label("DChart");
                    (0..DCHANELS/2).for_each(|x| { 
                        ui.checkbox(&mut  appx.appsettings.dchanels[x].viewindchart, "");
                    });
                });

                ui.separator();  
                ui.vertical(|ui| {
                    ui.label("Name");
                    (DCHANELS/2..DCHANELS).for_each(|x| { 
                        ui.horizontal(|ui| {
                            ui.label(format!("{}", x+1));
                            ui.add(egui::TextEdit::singleline(&mut appx.appsettings.dchanels[x].name).desired_width(80.0));
                        });
                    });
                });  
                ui.vertical(|ui| {
                    ui.label("Color");
                    (DCHANELS/2..DCHANELS).for_each(|x| { 
                        ui.color_edit_button_srgba(&mut appx.appsettings.dchanels[x].color);
                    });
                });
                ui.vertical(|ui| {
                    ui.label("Conv.");
                    (DCHANELS/2..DCHANELS).for_each(|x| { 
                        let prv = appx.mapsw.borrow().convertrd.dconverter[x]; 
                        ui.add(egui::DragValue::new(&mut appx.mapsw.borrow_mut().convertrd.dconverter[x]).speed(0.0001).range(RangeInclusive::new(0.0001, 100.0)));
                        if prv != appx.mapsw.borrow().convertrd.dconverter[x]{
                           appx.mode_swap = SwapMod::Full;
                        }
                    });
                });
                if recalc {
                  //  let _= &mut appx.rtmap.write().unwrap().rtmap_calc_dchanels();
                    appx.mode_swap = SwapMod::Full;
                }
                ui.vertical(|ui| {
                    ui.label("DChart");
                    (DCHANELS/2..DCHANELS).for_each(|x| { 
                        ui.checkbox(&mut  appx.appsettings.dchanels[x].viewindchart, "");
                    });
                });
            
            });         
        });
    }

    fn write_settings_to_file(file: &mut std::fs::File, appsettings: &AppSettings, _rtmap: Arc<RwLock<Rtmap>>, fmapread: &ConvertReduct) -> Result<usize, Box<dyn std::error::Error>> {
    
        // You need to take care of the conversion yourself
        // and can either try to write all data at once
        //file  .write_all(&data.as_bytes())?;
    
        // Or try to write as much as possible, but need
        // to take care of the remaining bytes yourself
        let mut remaining: usize = 0;
        let mut lines: usize = 0;
        remaining += file.write("Version:1\r\n".as_bytes())?; lines += 1; 
        appsettings.achanels.iter().enumerate().for_each(|y| {
            let x = y.1; 
            let i = y.0; 
            let fname = format!("Name{}:{}\r\n", i, x.name);
            let fcolor = format!("Color RGBA:{}:{}:{}:{}\r\n", x.color.r(), x.color.g(), x.color.b(), x.color.a());
            let fviewinchart = format!("Viewinchart:{}\r\n", x.viewinchart);
            let fviewinchart2 = format!("Viewinchart2:{}\r\n", x.viewinchart2);
            
            //let fmapread = ;//&rtmap.read().unwrap().convertrd;
            let fconverter = format!("Converter:{}\r\n", fmapread.aconverter.get(i).unwrap());
            let freduction = format!("Reduction:{}\r\n", fmapread.areduction.get(i).unwrap());
            let freduction2 = format!("Reduction2:{}\r\n", fmapread.areduction2.get(i).unwrap());

            remaining += file.write(fname.as_bytes()).expect("Write file error"); lines += 1;
            remaining += file.write(fcolor.as_bytes()).expect("Write file error"); lines += 1;
            remaining += file.write(fviewinchart.as_bytes()).expect("Write file error"); lines += 1;
            remaining += file.write(fviewinchart2.as_bytes()).expect("Write file error"); lines += 1;
            remaining += file.write(fconverter.as_bytes()).expect("Write file error"); lines += 1;
            remaining += file.write(freduction.as_bytes()).expect("Write file error"); lines += 1;
            remaining += file.write(freduction2.as_bytes()).expect("Write file error"); lines += 1;
        });

        // DChanelName {Color} {dconverter} [x] dchart
        appsettings.dchanels.iter().enumerate().for_each(|y| {
            let x = y.1; 
            let i = y.0; 
            let fname = format!("Name{}:{}\r\n", i, x.name);
            let fcolor = format!("Color RGBA:{}:{}:{}:{}\r\n", x.color.r(), x.color.g(), x.color.b(), x.color.a());
            let fviewinchart = format!("ViewinDchart:{}\r\n", x.viewindchart);
            
            //let fmapread = &rtmap.read().unwrap().convertrd;
            let fconverter = format!("Converter:{}\r\n", fmapread.dconverter.get(i).unwrap());

            remaining += file.write(fname.as_bytes()).expect("Write file error"); lines += 1;
            remaining += file.write(fcolor.as_bytes()).expect("Write file error"); lines += 1;
            remaining += file.write(fviewinchart.as_bytes()).expect("Write file error"); lines += 1;
            remaining += file.write(fconverter.as_bytes()).expect("Write file error"); lines += 1;
        });
        let duration = format!("Duration:{}\r\n", appsettings.duration );
        remaining += file.write(duration.as_bytes()).expect("Write file error"); lines += 1;
         
        while lines < 256 {
            remaining += file.write("Nop\r\n".as_bytes())?; lines += 1; 
        }
        /*while remaining < 4096 {
            let buf:Vec<u8> = vec![0];
            remaining += file.write(&buf)?; lines += 1; 
        }*/

        if remaining > 0 {
          // You need to handle the remaining bytes
        }
    
        Ok(remaining)
    }

    fn read_settings_from_file(data: Vec<u8>, appsettings: &mut AppSettings, _rtmap: Arc<RwLock<Rtmap>>, fmapread: &mut ConvertReduct) -> Result<(), Box<dyn std::error::Error>> {
       // let file =  std::fs::File::create(path)?;
       //use std::fs::read_to_string;
       // let data: Vec<u8> = std::fs::read(path)?;

        let mut slines = std::str::from_utf8(&data)?.lines();

        if let Some(sstr) = slines.next() {
            if sstr.contains("Version:") {
               // println!("sstr:{}", sstr); 
               // println!("sstr pos:{}", sstr.find(':').unwrap()); 
                let sl = &sstr[sstr.find(':').unwrap()+1..sstr.len()].parse::<u32>()?; 
                if *sl != 1 { Err("Version unknown")?; }
            } else { Err("Parser error")?; }
        }  else { Err("Parser error")?; }
        for y in appsettings.achanels.iter_mut().enumerate() {
            let x = y.1; 
            let i = y.0; 
            // name 
            if let Some(sstr) = slines.next() {
                if sstr.contains(format!("Name{}:", i).as_str()) {
                    let sl = &sstr[sstr.find(':').unwrap()+1..sstr.len()].parse::<String>()?; 
                    x.name = sl.clone(); 
                } else { Err("Parser error")?; }
            }  else { Err("Parser error")?; }
            // color
            if let Some(sstr) = slines.next() {
                if sstr.contains("Color RGBA:") {
                    let mut rc:u8 = 0;
                    let mut gc:u8 = 0;
                    let mut bc:u8 = 0;
                    let mut ac:u8 = 0;
                    let mut cnt = 0; 
                    for character in sstr.chars().enumerate() {
                        if character.1 == ':' {
                            if cnt == 0 {
                                let ind = character.0+1; 
                                let inds = ind+sstr[ind..sstr.len()].find(':').unwrap();
                                rc = sstr[ind..inds].parse::<u8>()?; 
                                cnt += 1; 
                            } else if cnt == 1 {
                                let ind = character.0+1; 
                                let inds = ind+sstr[ind..sstr.len()].find(':').unwrap();
                                gc = sstr[ind..inds].parse::<u8>()?; 
                                cnt += 1;
                            } else if  cnt == 2 {
                                let ind = character.0+1; 
                                let inds = ind+sstr[ind..sstr.len()].find(':').unwrap();
                                bc = sstr[ind..inds].parse::<u8>()?; 
                                cnt += 1; 
                            } else if  cnt == 3 {
                                let ind = character.0+1; 
                                ac = sstr[ind..sstr.len()].parse::<u8>()?; 
                                //cnt += 1; 
                            }
                        }
                    }
                    x.color = egui::Color32::from_rgba_premultiplied(rc, gc, bc, ac); 
                } else { Err("Parser error")?; }
            }  else { Err("Parser error")?; }
            // Viewinchart:
            if let Some(sstr) = slines.next() {
                if sstr.contains("Viewinchart:") {
                    let sl = &sstr[sstr.find(':').unwrap()+1..sstr.len()].parse::<bool>()?; 
                    x.viewinchart = sl.clone(); 
                } else { Err("Parser error")?; }
            }  else { Err("Parser error")?; }
            // Viewinchart2:
            if let Some(sstr) = slines.next() {
                if sstr.contains("Viewinchart2:") {
                    let sl = &sstr[sstr.find(':').unwrap()+1..sstr.len()].parse::<bool>()?; 
                    x.viewinchart2 = sl.clone(); 
                } else { Err("Parser error")?; }
            }  else { Err("Parser error")?; }           
            // Converter:
            if let Some(sstr) = slines.next() {
                if sstr.contains("Converter:") {
                    let sl = &sstr[sstr.find(':').unwrap()+1..sstr.len()].parse::<f64>()?; 
                    //let fmapread = &mut rtmap.write().unwrap().convertrd;
                    let df = fmapread.aconverter.get_mut(i).unwrap(); 
                    *df = *sl; 
                } else { Err("Parser error")?; }
            }  else { Err("Parser error")?; }
            // Reduction:
            if let Some(sstr) = slines.next() {
                if sstr.contains("Reduction:") {
                    let sl = &sstr[sstr.find(':').unwrap()+1..sstr.len()].parse::<f64>()?; 
                   // let fmapread = &mut rtmap.write().unwrap().convertrd;
                    let df = fmapread.areduction.get_mut(i).unwrap(); 
                    *df = *sl; 
                } else { Err("Parser error")?; }
            }  else { Err("Parser error")?; }           
            // Reduction2:
            if let Some(sstr) = slines.next() {
                if sstr.contains("Reduction2:") {
                    let sl = &sstr[sstr.find(':').unwrap()+1..sstr.len()].parse::<f64>()?; 
                  //  let fmapread = &mut rtmap.write().unwrap().convertrd;
                    let df = fmapread.areduction2.get_mut(i).unwrap(); 
                    *df = *sl; 
                } else { Err("Parser error")?; }
            }  else { Err("Parser error")?; }  
        }      // for achanels
        // dchanels settings 
        for y in appsettings.dchanels.iter_mut().enumerate() {
            let x = y.1; 
            let i = y.0; 
            // name 
            if let Some(sstr) = slines.next() {
                if sstr.contains(format!("Name{}:", i).as_str()) {
                    let sl = &sstr[sstr.find(':').unwrap()+1..sstr.len()].parse::<String>()?; 
                    x.name = sl.clone(); 
                } else { Err("Parser error")?; }
            }  else { Err("Parser error")?; }
            // color
            if let Some(sstr) = slines.next() {
                if sstr.contains("Color RGBA:") {
                    let mut rc:u8 = 0;
                    let mut gc:u8 = 0;
                    let mut bc:u8 = 0;
                    let mut ac:u8 = 0;
                    let mut cnt = 0; 
                    for character in sstr.chars().enumerate() {
                        if character.1 == ':' {
                            if cnt == 0 {
                                let ind = character.0+1; 
                                let inds = ind+sstr[ind..sstr.len()].find(':').unwrap();
                                rc = sstr[ind..inds].parse::<u8>()?; 
                                cnt += 1; 
                            } else if cnt == 1 {
                                let ind = character.0+1; 
                                let inds = ind+sstr[ind..sstr.len()].find(':').unwrap();
                                gc = sstr[ind..inds].parse::<u8>()?; 
                                cnt += 1;
                            } else if  cnt == 2 {
                                let ind = character.0+1; 
                                let inds = ind+sstr[ind..sstr.len()].find(':').unwrap();
                                bc = sstr[ind..inds].parse::<u8>()?; 
                                cnt += 1; 
                            } else if  cnt == 3 {
                                let ind = character.0+1; 
                                ac = sstr[ind..sstr.len()].parse::<u8>()?; 
                                //cnt += 1; 
                            }
                        }
                    }
                    x.color = egui::Color32::from_rgba_premultiplied(rc, gc, bc, ac); 
                } else { Err("Parser error")?; }
            }  else { Err("Parser error")?; }
            // ViewinDchart:
            if let Some(sstr) = slines.next() {
                if sstr.contains("ViewinDchart:") {
                    let sl = &sstr[sstr.find(':').unwrap()+1..sstr.len()].parse::<bool>()?; 
                    x.viewindchart = sl.clone(); 
                } else { Err("Parser error")?; }
            }  else { Err("Parser error")?; }
            // Converter:
            if let Some(sstr) = slines.next() {
                if sstr.contains("Converter:") {
                    let sl = &sstr[sstr.find(':').unwrap()+1..sstr.len()].parse::<f64>()?; 
                    //let fmapread = &mut rtmap.write().unwrap().convertrd;
                    let df = fmapread.dconverter.get_mut(i).unwrap(); 
                    *df = *sl; 
                } else { Err("Parser error")?; }
            }  else { Err("Parser error")?; }
        }
        // Duration:
        if let Some(sstr) = slines.next() {
            if sstr.contains("Duration:") {
                let sl = &sstr[sstr.find(':').unwrap()+1..sstr.len()].parse::<usize>()?; 
                appsettings.duration = *sl;
            } else { Err("Parser error")?; }
        }  else { Err("Parser error")?; }       

        Ok(())
    }

    fn write_scope_to_file(file: &mut std::fs::File, appsettings: &AppSettings, rtmap: Arc<RwLock<Rtmap>>, fmapread: &mut ConvertReduct) -> Result<usize, Box<dyn std::error::Error>> {
        let mut remaining: usize;
        remaining = write_settings_to_file(file, appsettings, rtmap.clone(), fmapread )?;
        /*while remaining < 4096 {
            let buf:Vec<u8> = vec![0];
            remaining += file.write(&buf)?; lines += 1; 
        }*/
        let buf:Vec<u8> = vec![0]; 
        remaining += file.write(&buf)?; // разделитель на символьные настройки и
        // бинарные данные 
        let sl = rtmap.read().unwrap(); 

        for i in 0..sl.rtmap.len() {
            let rtpoint = sl.rtmap.get(i).unwrap(); 

            let mut bytes = bincode::encode_to_vec(
                &rtpoint,
                bincode::config::standard()
            ).unwrap();

        //println!("num:{}, {}, savea0: {}", i, rtpoint.timestamp, rtpoint.achanels[0] ); 
            bytes.insert(0, bytes.len() as u8);
            remaining += file.write(&bytes)?;
           // println!("{:?}", bytes);
          // println!("bytes {}", remaining-old);
        }

        /* 
        let myrt:RtPoint = DefaultOptions::new()
        .with_varint_encoding()
        .deserialize(&bytes)?;
        println!("{}", myrt.timestamp);
        */

        Ok(remaining)
    }

    fn read_scope_from_file(data: Vec<u8>, appsettings: &mut AppSettings, rtmap: Arc<RwLock<Rtmap>>, fmapread: &mut ConvertReduct ) -> Result<(), Box<dyn std::error::Error>> {
        let mut inull: usize = 0; 
        // ищем разделитель на символьные настройки и бинарные данные 
        for i in 0..data.len() {
            if data[i] == 0 {
                inull = i;   
                break; 
            }
        }
        //println!("{}", inull);

        let sldata = data[0..inull].to_vec();
        read_settings_from_file(sldata, appsettings, rtmap.clone(), fmapread)?;
        inull += 1; // на +1 начинаются данные
        
        let bdata = &data[inull..data.len()];

        // не знаем сколько RtPoint записанный в файл занимает места
        // узнаем формируя одну точку
        let mut sl = rtmap.write().unwrap();
        /*let rtpoint = sl.rtmap.get_mut(0).unwrap();
        let bytes = DefaultOptions::new()
            .with_varint_encoding()
            .serialize(&rtpoint)?;
        let rtpointsize = bytes.len();*/

        let mut new_rtmap: Vec<RtPoint> = Vec::new(); 

        let mut ind:usize = 0;  
        while ind < bdata.len() {
            let blen = bdata[ind]; 
            if blen < 20 || blen > 70 { 
                Err("parser error")?; 
            }
            ind = ind+1;
            let bytes = &bdata[ind..ind+blen as usize];

            /*let new_rt:RtPoint = DefaultOptions::new()
            .with_varint_encoding()
            .deserialize(bytes)?;*/
            let tempret:Result<(RtPoint, usize), _> = bincode::decode_from_slice(bytes, bincode::config::standard());
            if tempret.is_ok()  {
                new_rtmap.push(tempret.unwrap().0);
            }
            //else { break; } 

            //new_rtmap.push(new_rt);
            ind += blen as usize; 
        }

        sl.rtmap = new_rtmap;

        Ok(())
    }

    fn write_csv_file(file: &mut std::fs::File, file2: &mut std::fs::File, appsettings: &AppSettings, rtmap: Arc<RwLock<Rtmap>>, fmapread: &mut ConvertReduct, start: Option<i64>, stop: Option<i64>) -> Result<usize, Box<dyn std::error::Error>> {
        let mut remaining: usize = 0;
        let sl = rtmap.read().unwrap();
        let rt = &sl.rtmap; 

        //let buf:Vec<u8> = vec![0]; 
        //remaining += file.write(&buf)?; // разделитель на символьные настройки и
        // бинарные данные 
       // let fname = format!("Name{}:{}\r\n", i, x.name);
       // remaining += file.write(fname.as_bytes()).expect("Write file error"); lines += 1;
        if start.is_none() || stop.is_none() {
            Err("no have twist CSV1 CSV2 timestamps")?;
        }
        let csvstart = start.unwrap(); 
        let csvstop = stop.unwrap();
        if csvstop < csvstart {
            Err("CSV2 timestamp must be better then CVS1")?;
        }

        let pointstart = rt.iter().position(|x| x.timestamp > csvstart);
        let pointstop = rt.iter().position(|x| x.timestamp > csvstop);

        if pointstart.is_some() && pointstop.is_some() {
            let ps = pointstart.unwrap();
            let pt = pointstop.unwrap(); 
            let mut chvec1 = Vec::<usize>::new(); 
            let mut chvec2 = Vec::<usize>::new(); 
            for i in 0..ACHANELS {
                if appsettings.achanels[i].viewinchart {
                    chvec1.push(i);
                }
                if appsettings.achanels[i].viewinchart2 {
                    chvec2.push(i);
                }
            }
            //let fist_ts = rt[0].timestamp; 
            if chvec1.len() > 0 {
                let mut st_out = format!("");
                st_out.push_str(&format!("Timestamp,"));
                for j in 0..chvec1.len()-1 {
                    st_out.push_str(&format!("{},", appsettings.achanels[chvec1[j]].name));
                }
                st_out.push_str(&format!("{}\r\n", appsettings.achanels[*chvec1.last().unwrap()].name));
                remaining += file.write(st_out.as_bytes())?; 
                for i in ps..pt  {
                    let mut st_out = format!("");
                    let ts = rt[i].timestamp; 
                    st_out.push_str(&format!("{},", ts));
                    for j in 0..chvec1.len()-1 {
                        let ach = chvec1[j];
                        let val = (rt[i].achanels[ach] as f64)*fmapread.aconverter[ach]*fmapread.areduction[ach];
                        st_out.push_str(&format!("{:.3},", val));
                    }
                    let ach = *chvec1.last().unwrap();
                    let val = (rt[i].achanels[ach] as f64)*fmapread.aconverter[ach]*fmapread.areduction[ach];
                    st_out.push_str(&format!("{:.3}\r\n", val));                    
                    remaining += file.write(st_out.as_bytes())?; 
                }
            }
            if chvec2.len() > 0 {
                let mut st_out = format!("");
                st_out.push_str(&format!("Timestamp,"));
                for j in 0..chvec2.len()-1 {
                    st_out.push_str(&format!("{},", appsettings.achanels[chvec2[j]].name));
                }
                st_out.push_str(&format!("{}\r\n", appsettings.achanels[*chvec2.last().unwrap()].name));
                remaining += file2.write(st_out.as_bytes())?; 
                for i in ps..pt  {
                    let mut st_out = format!("");
                    let ts = rt[i].timestamp; 
                    st_out.push_str(&format!("{},", ts));
                    for j in 0..chvec2.len()-1 {
                        let ach = chvec2[j];
                        let val = (rt[i].achanels[ach] as f64)*fmapread.aconverter[ach]*fmapread.areduction2[ach];
                        st_out.push_str(&format!("{:.3},", val));
                    }
                    let ach = *chvec2.last().unwrap();
                    let val = (rt[i].achanels[ach] as f64)*fmapread.aconverter[ach]*fmapread.areduction2[ach];
                    st_out.push_str(&format!("{:.3}\r\n", val));                    
                    remaining += file2.write(st_out.as_bytes())?; 
                }
            }
        }
        else {
            Err("no have data")?;
        }

        Ok(remaining)
    }
}



/*
use bincode::{DefaultOptions, Options};
use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
struct MyStruct {
    bytes: Vec<u8>,
}

fn main() {
    let s = MyStruct {
        bytes: [0u8, 1u8, 2u8, 3u8].to_vec(),
    };

    let bytes = DefaultOptions::new()
        .with_varint_encoding()
        .serialize(&s);

    println!("{:?}", bytes);
}

    fn get_index(v: Vec<u8>, occurrence: usize, value: u8) -> Option<usize> {
    v.iter()
        .enumerate()
        .filter(|(_, &v)| v == value)
        .map(|(i, _)| i)
        .nth(occurrence - 1)

        Мы берем итератор нашего вектора v.
Мы enumerate этот итератор, преобразуя его в [(0, 3), (1, 2), (2, 1), ...]. Таким образом, мы можем сохранить индекс элемента после фильтрации.
Мы фильтруем наше перечисление в зависимости от того, равно ли значение (правый элемент в кортеже) искомому элементу.
Мы сопоставляем и удаляем часть со значением, так как она больше не актуальна (но сохраняем индекс).
Мы берём occurrence-й элемент, который является опцией, которая может содержать индекс (поскольку он может не существовать).
}
*/