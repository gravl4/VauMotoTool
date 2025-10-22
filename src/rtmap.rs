#[allow(dead_code)] // отключаем предупреждение о неиспользуемом коде
pub mod rtmap {

    use std::iter::Iterator;
    use bincode::{Decode, Encode};
    //, time}; //convert,  , path}
    //use std::error::Error;
    //use std::path::Path;
   // use std::ffi::OsStr;
   //use egui_plot::PlotPoint;
    use serde::{Deserialize, Serialize};

    /// шаг единицы измерения времени в программе микросекунда
    ///

    #[derive(Debug, Serialize, Deserialize, Encode, Decode)]
    pub struct RtPoint {
        pub nan: bool,
        pub timestamp: i64,
        pub achanels:Vec<i16>,
        pub digital: u64,
    }
    impl Clone for RtPoint {
        fn clone(&self) -> Self {
            RtPoint {
                nan: self.nan, 
                timestamp: self.timestamp, 
                achanels: self.achanels.clone(), 
                digital: self.digital,
            }
        }
    }

    pub const ACHANELS: usize = 16; 
    pub const DCHANELS: usize = 32; 
    pub const DCHANELAMP: f64 = 1.0;

//const LEFT: [&'static str; 3] = ["Hello", "World", "!"];
    pub struct ConvertReduct {
        pub aconverter: Vec<f64>, 
        pub areduction: Vec<f64>, 
        pub areduction2: Vec<f64>,
        pub dconverter: Vec<f64>, 
    }
    impl Clone for ConvertReduct {
        fn clone(&self) -> Self {
            ConvertReduct {
                aconverter: self.aconverter.clone(), 
                areduction: self.areduction.clone(), 
                areduction2: self.areduction2.clone(),
                dconverter:self.dconverter.clone(),
            }
        }
    }

    impl ConvertReduct {
        pub fn new() -> Self {
            let aconverter:Vec<f64> = (0..ACHANELS).map(|_x| { 1.0 as f64 }).collect();
            let areduction:Vec<f64> = (0..ACHANELS).map(|_x| { 1.0 as f64 }).collect();
            let areduction2:Vec<f64> = (0..ACHANELS).map(|_x| { 1.0 as f64 }).collect();
            let dconverter:Vec<f64> = (0..DCHANELS).map(|x| { (DCHANELS-x) as f64 }).collect();
            ConvertReduct {
                aconverter, 
                areduction, 
                areduction2,
                dconverter, 
            }             
        }
    }

    // возвращает 0 или 1
    pub fn rtmap_get_dchanal( vin: u64, chanel: usize  )-> u64 {
        if chanel > 31 {
            panic!("Rtmap error chanels > 31"); 
        }
        (vin & (0x1 << chanel)) >> chanel
    }

/// rtmap - сэмплы измерений от UDP
/// gachanels вектор точек для графика 1. gachanels =  achanels[chanel]*aconverter[chanel]*areduction[chanel]
/// gachanels2 вектор точек для графика 2. gachanels2 =  achanels[chanel]*aconverter[chanel]*areduction2[chanel]
/// gachanels вектор точек для графика 1. gachanels =  achanels[chanel]*aconverter[chanel]*areduction[chanel]
/// aconverter - коэффициент пересчета в физическую величину
/// areduction - коэффициент изменения масштаба для графика
/// dconverter - смещение цифровых каналов. канал 1*32, канал 2*31 ... 
    pub struct Rtmap {
        pub rtmap: Vec<RtPoint>, 
        //pub gachanels: Vec<Vec<PlotPoint>>,
        //pub gachanels2: Vec<Vec<PlotPoint>>,
        //pub gdchanels: Vec<Vec<PlotPoint>>,  
        //pub convertrd: ConvertReduct,  
    }
    
    impl Rtmap {
        /// Called once before the first frame.
        /// создает массивы с 0.0 значениями от конечного timestamp с шагом времени deltatime
        /// last - самое новое значение (самое большое) timestamp
        /// gachanelx и gdchanels - пустые массивы. 
        /// После new нужно вызвать rtmap_calc_all() для расчета и заполения gachanelx и gdchanels
        pub fn new(samples:usize, timestamp: i64, deltatime: i64) -> Self {
            if samples == 0 {
                panic!("Rtmap error: samples is null"); 
            }

            let starttime = timestamp - (samples as i64)*deltatime; 
            let rtmap:Vec<RtPoint> =  (0..samples).map(|x| {  
                RtPoint {
                    nan: false, 
                    timestamp: starttime+(x as i64)*deltatime,
                    achanels: (0..ACHANELS).map(|_x| { 0 }).collect(),
                    digital: 0,
                }
            }).collect(); 

            //let zerochanel: Vec<PlotPoint> = (0..samples).map(|x| {
            //    PlotPoint{x: rtmap.get(x).expect("Rtmap error").timestamp as f64 , y: 1.0 }}).collect(); 
            //let gachanels: Vec<Vec<PlotPoint>> = (0..ACHANELS).map(|_x| { zerochanel.clone() }).collect();
            //let gachanels2: Vec<Vec<PlotPoint>> = (0..ACHANELS).map(|_x| { zerochanel.clone() }).collect();
            //let gdchanels: Vec<Vec<PlotPoint>> = (0..DCHANELS).map(|_x| { zerochanel.clone() }).collect();

            //let convertrd = ConvertReduct::new(); 
            Rtmap {
                rtmap, 
                //gachanels, 
                //gachanels2,
                //gdchanels, 
                //convertrd, 
            }
        }

        /*
        // расчет одной точки для одного канала chanel b заданного gachelx (chart)
        pub fn rtmap_calc_achanel_point(&self, rtpoint: &RtPoint, chanel: usize, chart: usize)->f64 {
            if chanel > ACHANELS { 
                panic!("Rtmap error a high number chanel"); 
            }
            let value = *rtpoint.achanels.get(chanel).expect("Rtmap error 10"); 
            let convert = &self.convertrd;
            let converter = *convert.aconverter.get(chanel).unwrap();
            let reduction: f64;
            if chart == 0 {
                reduction = *convert.areduction.get(chanel).unwrap();
            } else if chart == 1 {
                reduction = *convert.areduction2.get(chanel).unwrap();
            } else {
                panic!("Rtmap error chart");
            }
            (value as f64)*converter*reduction
        }

        // расчет одной точки для всех d каналов
        pub fn rtmap_calc_dchanels_point(&self, rtpoint: &RtPoint)->Vec<f64> {
            let mut values:Vec<f64> = vec![]; 
            for i in 0..DCHANELS {
                let res = (rtmap_get_dchanal(rtpoint.digital, i) as f64)*0.7+self.convertrd.dconverter[i]-1.0;
                values.push(res);
            }
            values
        }*/

        // добавляет точку RtPoint, расчет и добавление точек в gachanelx gdchanels 
        pub fn rtmap_push(&mut self, rtpoint: RtPoint) {
            let lasttime = self.rtmap.last().expect("Rtmap error").timestamp; 
            if lasttime > rtpoint.timestamp {
                return; 
               // panic!("Rtmap error: early timestamp"); 
            }
          //  let convert = &self.convertrd;
            // новая точка
            self.rtmap.push(rtpoint.clone());
            /*let timestamp = rtpoint.timestamp as f64; 
            let mut mv:Vec<f64>=Vec::new(); 
            let mut mv2:Vec<f64>=Vec::new(); 
            (0..ACHANELS).for_each(|i| {
                let value = self.rtmap_calc_achanel_point(&rtpoint, i, 0);
               // println!("avalue {}", value);
                mv.push(value);          
            });
            (0..ACHANELS).for_each(|i| {
                let value = self.rtmap_calc_achanel_point(&rtpoint, i, 1);
                mv2.push(value);          
            });

        
            if let Some(_prtime) = self.rtmap.last() {
                // было предыдущее значение добавляем ступеньку

                let mut mvpr:Vec<f64>=Vec::new(); 
                self.gdchanels.iter().for_each(|x| {
                    mvpr.push(x.last().unwrap().y);
                });    
                for i in 0..DCHANELS {
                    self.gdchanels[i].push(PlotPoint{x: timestamp-1.0, y: mvpr[i] });
                }                
            } 

            self.gachanels.iter_mut().enumerate().for_each(|x| { 
                x.1.push(PlotPoint{x: timestamp, y: *mv.get(x.0).unwrap()});});
            self.gachanels2.iter_mut().enumerate().for_each(|x| { 
                x.1.push(PlotPoint{x: timestamp, y: *mv2.get(x.0).unwrap()});});

            let dvec = self.rtmap_calc_dchanels_point(&rtpoint); 
            for i in 0..DCHANELS {
                self.gdchanels[i].push(PlotPoint{x: timestamp, y: dvec[i] });
            }*/    
        }

        /*
        // удаляет все точки до указанного earlytimestamp
        pub fn rtmap_remove(&mut self, earlytimestamp: i64) {
            let lasttime = self.rtmap.last().expect("Rtmap error").timestamp; 
            if lasttime < earlytimestamp {
                panic!("Rtmap error: early timestamp"); 
            }
            //  удаляем старые точки до нужного значения 
            while  self.rtmap.get(0).expect("Rtmap error").timestamp < earlytimestamp {
                self.rtmap.remove(0); 
                self.gachanels.iter_mut().for_each(|x| { x.remove(0);});
                self.gachanels2.iter_mut().for_each(|x| { x.remove(0);});
                self.gdchanels.iter_mut().for_each(|x| { x.remove(0);});
            }
            //  удаляем старые точки до нужного значения, в d каналах их больше, из-за формирования ступенек
            //let y =  self.gdchanels.get(0).expect("Rtmap error")[0].x; 
            while  (self.gdchanels.get(0).expect("Rtmap error")[0].x as i64)  < earlytimestamp {
                self.gdchanels.iter_mut().for_each(|x| { x.remove(0);});
            }           
        }   

        // пересчет аналогового канала chanel
        pub fn rtmap_calc_achanel(&mut self, chanel: usize) {
            if chanel > ACHANELS || self.rtmap.len() != self.gachanels[chanel].len() ||
            self.rtmap.len() != self.gachanels2[chanel].len()  {
                panic!("Rtmap error calc achanel"); 
            }
            for i in 0..self.rtmap.len() {
                let res = self.rtmap_calc_achanel_point(&self.rtmap[i], chanel, 0);
                self.gachanels[chanel][i].y = res;
            }
        } 
        pub fn rtmap_calc_achanel2(&mut self, chanel: usize) {
            if chanel > ACHANELS || self.rtmap.len() != self.gachanels[chanel].len() ||
            self.rtmap.len() != self.gachanels2[chanel].len()  {
                panic!("Rtmap error calc achanel"); 
            }
            for i in 0..self.rtmap.len() {
                let res = self.rtmap_calc_achanel_point(&self.rtmap[i], chanel, 1);
                self.gachanels2[chanel][i].y = res;
            }
        } 

        // пересчет всех цифровых каналов 
        pub fn rtmap_calc_dchanels(&mut self) {
            if DCHANELS != self.gdchanels.len() ||
            self.rtmap.len() != self.gdchanels[0].len()  {
                panic!("Rtmap error calc achanel"); 
            }

            let zerochanel: Vec<PlotPoint> = vec![];
            let mut gdchanels: Vec<Vec<PlotPoint>> = (0..DCHANELS).map(|_x| { zerochanel.clone() }).collect();

            for i in 0..self.rtmap.len() {
                let rtpoint = self.rtmap.get(i).unwrap();
                let dvec = self.rtmap_calc_dchanels_point(&rtpoint); 
                for i in 0..DCHANELS {
                    let timestamp = rtpoint.timestamp as f64; 
                    gdchanels[i].push(PlotPoint{x: timestamp, y: dvec[i] });
                }
            }            
            self.gdchanels = gdchanels; 
        }

        /// стирает массивы отображения. Рассчитывает и заполняет заново
        pub fn rtmap_calc_all(&mut self) {
            let zerochanel: Vec<PlotPoint> = vec![];
            let mut gachanels: Vec<Vec<PlotPoint>> = (0..ACHANELS).map(|_x| { zerochanel.clone() }).collect();
            let mut gachanels2: Vec<Vec<PlotPoint>> = (0..ACHANELS).map(|_x| { zerochanel.clone() }).collect();
            let mut gdchanels: Vec<Vec<PlotPoint>> = (0..DCHANELS).map(|_x| { zerochanel.clone() }).collect();

            for i in 0..self.rtmap.len() {
                let rtpoint = self.rtmap.get(i).unwrap();
                // новая точка
                let timestamp = rtpoint.timestamp as f64; 
                
                let mut mv:Vec<f64>=Vec::new(); 
                let mut mv2:Vec<f64>=Vec::new(); 
                (0..ACHANELS).for_each(|i| {
                    let value = self.rtmap_calc_achanel_point(&rtpoint, i, 0);
                    mv.push(value);          
                });
                (0..ACHANELS).for_each(|i| {
                    let value = self.rtmap_calc_achanel_point(&rtpoint, i, 1);
                    mv2.push(value);          
                });
               // println!("--num {}, {}, a0: {}", i, timestamp, rtpoint.achanels[0]); 

                for i in 0..ACHANELS {
                    gachanels[i].push(PlotPoint{x: timestamp, y: *mv.get(i).unwrap()});
                    gachanels2[i].push(PlotPoint{x: timestamp, y: *mv2.get(i).unwrap()});
                }
                let dvec = self.rtmap_calc_dchanels_point(&rtpoint); 
                for i in 0..DCHANELS {
                    gdchanels[i].push(PlotPoint{x: timestamp, y: dvec[i] });
                }
            }
            self.gachanels = gachanels; 
            self.gachanels2 = gachanels2;
            self.gdchanels = gdchanels; 

        }

        // x - timestamp us
        // prind - откуда начинать поиск
        pub fn rtmap_get_ys_for_x(&self, x: i64, prind:Option<usize>)->Result<(RtPoint, usize), Box<dyn std::error::Error>> {
            let firsttime = self.rtmap.first().unwrap().timestamp;
            let lasttime = self.rtmap.last().unwrap().timestamp;
            let indlen = self.rtmap.len(); 
            
            let mut ind:usize = 0;   
            if prind.is_none()  {
                // рассчитываем примерное значение исходя из линейного преобразования
                if x > firsttime {
                    if x < lasttime {
                        let k = (indlen as f64)/(lasttime as f64-firsttime as f64);
                        ind = (k*((x-firsttime) as f64)) as usize; 
                    }
                    else {
                        //indlen
                        Err("No have rtmap index")?;
                    }
                }
                else {
                    //0
                    Err("No have rtmap index")?;
                }
            }
            else {
                ind = prind.unwrap();
            };
            let mut indw = ind; 
            let rtp = self.rtmap.get(indw).ok_or("No index")?;  
            if  rtp.timestamp > x { 
                while self.rtmap.get(indw).ok_or("No index")?.timestamp > x {
                    indw -= 1;
                }
            } else { 
                while self.rtmap.get(indw).ok_or("No index")?.timestamp < x {
                    indw += 1;
                }
            };
            let rtpoint = self.rtmap.get(indw).ok_or("No index")?; 
    
            Ok((rtpoint.clone(), indw))
        }  
        */

    }
}