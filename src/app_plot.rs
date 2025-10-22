#[allow(dead_code)] // отключаем предупреждение о неиспользуемом коде
pub mod app_plot {

    use std::{ cell::RefCell, rc::Rc}; //arch::x86_64,
    use rayon::prelude::*;

    use eframe::{egui::{self, Ui, Vec2b}, wgpu::naga::diagnostic_filter::FilterableTriggeringRule};
    //use egui_plot::{Corner::{self, LeftTop}, Cursor, Legend, Line, Plot, PlotBounds, PlotItem, PlotPoint, PlotPoints};
    use egui_plot::{Corner::{self}, Legend, Line, Plot, PlotPoints, PlotPoint};
    
    //use std::sync::{RwLock, Arc, Mutex};
    use chrono::DateTime;

    use crate::app::app::{Appx, ViewApp, XAxisTime};
    use crate::rtmap::rtmap::{ACHANELS, DCHANELS, ConvertReduct, RtPoint}; 
    use crate::compressor::compressor::*; 

    /*
    1. [v] отрисовка графиков
    2. [v] мгновенные значения
    2.1 [v] пересчет канала при изменении convert, reduction
    3. ПРИЕМ ПАКЕТОВ  
    4. [v] двойные точки цифровых каналов
    5. файл вид скролл - кнопки разделить
    6. короткие кнопки редукции каналов
    7. ускоритель отображения, если возможно
    8. обработать таймаут без нормальных пакетов

    Shape::line
     */

    // количество точек для отображение на графике 
    // которое стремиться поддерживать swap
    pub const PLOT_POINTS: usize = 1500; 

    struct Aminmax {
        min: i16, 
        max: i16, 
        minfirst: bool, 
    }
    impl Aminmax {
        fn new_vec()-> Vec<Aminmax> {
            let achs: Vec<Aminmax>  = (0..ACHANELS).into_iter().map(|_x|{ 
                Aminmax {
                    min: 0,
                    max: 0, 
                    minfirst: true, 
                }
             }).collect();
             achs
        }
    }
    #[derive(PartialEq, Clone)]
    pub enum SwapMod {
        Add,
        Full, 
    }
    pub struct IndPlotChanel {
        pub ppoints: Vec<PlotPoint>, 
       // pub swapind: Vec<usize>, 
    }
    pub struct RTmapsw {
        pub rtswap: Vec<RtPoint>, 
        pub gachanels: Vec<IndPlotChanel>,
        pub gachanels2: Vec<IndPlotChanel>,
        pub gdchanels: Vec<IndPlotChanel>,  
        pub convertrd: ConvertReduct,  
         first: usize, 
         begin: usize, // с чего начинать для 
         end: usize,
         xptbound: [i64; 2], // предыдущий диапазон
         wp: f32,// предыдущая ширина
         sppp: f32, // предыдущий spp
         //indstartp: usize, 
         indendp: usize, 
        pub run: bool, 
    }
    impl RTmapsw {
        pub fn new() -> Self {
            let rtswap:Vec<RtPoint> = vec![];
            let zerochanel: Vec<PlotPoint> = vec![]; 
            //let zerochanelind: Vec<usize> = vec![];
            let gachanels: Vec<IndPlotChanel> = (0..ACHANELS).map(|_x| { 
                IndPlotChanel {
                    ppoints: zerochanel.clone(),
               //     swapind: zerochanelind.clone(),
                }
            }).collect();
            let gachanels2: Vec<IndPlotChanel> = (0..ACHANELS).map(|_x| { 
                IndPlotChanel {
                    ppoints: zerochanel.clone(),
                 //   swapind: zerochanelind.clone(),
                }
            }).collect();
            let gdchanels: Vec<IndPlotChanel> = (0..DCHANELS).map(|_x| { 
                IndPlotChanel {
                    ppoints: zerochanel.clone(),
                  //  swapind: zerochanelind.clone(),
                }
            }).collect();

            let convertrd = ConvertReduct::new(); 
            let xptbound: [i64; 2] = [0, 0]; 

            RTmapsw {
                rtswap, 
                gachanels, 
                gachanels2,
                gdchanels, 
                convertrd, 
                first: 0, 
                begin: 0, 
                end: 0,
                xptbound, 
                wp: 0.0,
                sppp: 0.0, 
                //indstartp: 0, 
                indendp: 0, 
                run: false, 
            }
        }

        // выдает мгновенные значения сигналов для timestamp
        pub fn get_insta(&self, rtmap: &Vec<RtPoint>, timestamp: i64) -> Result<(Vec<f64>, Vec<u8>), &str> {
            let mut aret:Vec<f64> = (0..ACHANELS).into_iter().map(|_x| {0.0}).collect();
            let mut dret:Vec<u8> = (0..DCHANELS).into_iter().map(|_x| {0}).collect();

            let point = rtmap.par_iter().find_last(|x| {
                x.timestamp <= timestamp  
            }); 

            if point.is_some() {
                let pp= point.unwrap(); 
                (0..ACHANELS).into_iter().for_each(|x| {
                    aret[x] = (pp.achanels[x] as f64)*self.convertrd.aconverter[x];
                });
                (0..DCHANELS).into_iter().for_each(|x| {
                    dret[x] = ((pp.digital & (0x1 << x)) >> x) as u8;
                });
            }
            else {
                return Err("Point insta not found"); 
            }

            Ok((aret, dret))
        }
        
        // swap уплотнение всего вектора Rtpoint
        pub fn swap(&mut self, rtmap: &Vec<RtPoint>, xtbound: [i64; 2], w: f32, mode: SwapMod)-> Result<(),Box<dyn std::error::Error>> {
            if w <= 0.0 { Err("Rtswap w is null")?; }
            //println!("delta bounds {}", (xtbound[1] - xtbound[0]) as f32);
            let sppf = ((xtbound[1] - xtbound[0]) as f32)/w;
            if sppf > 3000000.0 || sppf < 1.0 || sppf == 0.0 { Err("Rtswap spp range")?; } 

            let sppn = if self.sppp/sppf > 1.3 || self.sppp/sppf < 0.7 {
                sppf
            } else { self.sppp };

            let spp = sppn as i64; 

            let mut inmode = mode; 

           // if self.xptbound[1] - self.xptbound[0] != xtbound[1] - xtbound[0] || self.wp != w {
            if self.sppp != sppn {
                inmode = SwapMod::Full; 
               // println!("inmode = Full p{}, {}, wp{}, w{}", self.xptbound[1] - self.xptbound[0], xtbound[1] - xtbound[0],self.wp, w ); 
            }

            if inmode == SwapMod::Add {
                if self.begin >= rtmap.len() {
                    Err("Rtswap begin range")?;
                }
            }
            else {
                self.begin = 0;
            }
        
           // if inmode == SwapMod::Full { 
          //  }      

            let mut prelast = 0;
            let mut tmppre = 0;
            let mut prefirst = 0;
            if rtmap.len() > 1000 {
                for x in (0..rtmap.len()-1).step_by(rtmap.len()/100) {
                    if rtmap[x].timestamp >= xtbound[1] {
                        prelast = tmppre;
                        break; 
                    }
                    tmppre = x;
                }
                tmppre = 0;
                for x in (0..rtmap.len()-1).step_by(rtmap.len()/100) {
                    if rtmap[x].timestamp >= xtbound[0] {
                        prefirst = tmppre;
                        break; 
                    }
                    tmppre = x;
                }
            }
                let mut last = rtmap.len()-1;
                    let pointend = rtmap[prelast..prelast+rtmap.len()/100].par_iter().enumerate().find_first(|x| {
                        x.1.timestamp >= xtbound[1] 
                    });
                    if pointend.is_some() {
                        last = pointend.unwrap().0+prelast;
                    }
                    else {
                        let pointend = rtmap[prelast..rtmap.len()].par_iter().enumerate().find_first(|x| {
                            x.1.timestamp >= xtbound[1]
                        });
                        if pointend.is_some() {
                            last = pointend.unwrap().0+prelast;
                        }
                    }
                let mut first = 0; //prefirst+rtmap.len()/100
                    let pointend = rtmap[prefirst..prefirst+rtmap.len()/100].par_iter().enumerate().find_first(|x| {
                        x.1.timestamp >= xtbound[0]-xtbound[0]/50
                    });
                    if pointend.is_some() {
                        first = pointend.unwrap().0+prefirst;
                    }
                    else {
                        let pointend = rtmap[prefirst..rtmap.len()].par_iter().enumerate().find_first(|x| {
                            x.1.timestamp >= xtbound[0]-xtbound[0]/50
                        });
                        if pointend.is_some() {
                            first = pointend.unwrap().0+prefirst;
                        }
                    }
                    //if first == 0 {
                    //    println!("first null");
                    //}
                 
            let mut left_timestamp = xtbound[0]; 
            let mut right_timestamp = xtbound[1];
            let mut left_append:usize = 0;
            let mut right_append:usize = 0;
            let mut left_delete:usize = 0;
            let mut right_delete:usize = 0;
           // println!("1{} 2{} d{} r{} l{} d{}", rtmap[first+2].timestamp, rtmap[first].timestamp, (rtmap[first+2].timestamp - rtmap[first].timestamp),
            //right_timestamp, left_timestamp, right_timestamp - left_timestamp);
            if self.run {
                let window_jump = (last - first)/4;
                let mut shifted_first = first;
                for i in first..first+(last - first)/20 { //5% диапазона проверяем на наличие разрывов 
                    if (rtmap[i+window_jump].timestamp-rtmap[i].timestamp) > (right_timestamp - left_timestamp)/4 {
                   // println!("1{} 2{} d{} r{} l{} d{}", rtmap[i+window_jump].timestamp, rtmap[i].timestamp, (rtmap[i+window_jump].timestamp - rtmap[i].timestamp),
                     //   right_timestamp, left_timestamp, (right_timestamp - left_timestamp)/10);
                        shifted_first = i;
                    }
                }
                first = shifted_first;
            }

            /*let ff = self.rtswap.first(); 
            if ff.is_none() {
                inmode = SwapMod::Full;
            }
            else {
                if ff.unwrap().timestamp >  right_timestamp {
                    inmode = SwapMod::Full;
                }
            }*/

            match inmode {
                SwapMod::Full => {                 
                    let parswap = swap_compressor(&rtmap[first..last], spp as usize)?; 
                    self.rtswap = parswap.0; 

                    self.first = first;
                    self.begin = first + parswap.1; 
                    self.end = last; // rtmap.len()-1; 
                    //println!("b {}, e {}",self.begin,  self.end );
                    self.xptbound[1] = xtbound[1];
                    self.xptbound[0] = xtbound[0];
                    self.wp = w;
                },
                SwapMod::Add => {
                    let rlast = if last > self.end { last } else { self.end };

                    //println!("s.f {} first {}, begin {}, end {}, last {}, rlast {}, rtswap {}",self.first, first, self.begin, self.end, last, rlast, self.rtswap.len()); //self.begin
                    //println!("s.f {} first {}, end {}, last {}, rlast {}",self.first, first, self.end, last, rlast); //self.begin
                    // добавляем в конце
                    if self.end < rlast {
                        let mut parswap = swap_compressor(&rtmap[self.end..rlast], spp as usize)?; 
                        if  parswap.0.len() > 2 { // 
                            // записываем остатки 
                            right_append = parswap.0.len();
                            self.rtswap.append(&mut parswap.0);
                            self.xptbound[1] = xtbound[1];
                            self.xptbound[0] = xtbound[0];
                            self.wp = w; 
                            self.end = rlast; 
                        }
                    }

                    // добавляем в начало
                    if first < self.first { 
                        let mut parswap = swap_compressor(&rtmap[first..self.first], spp as usize)?;
                        if  parswap.0.len() > 2 { // 
                            // записываем в начало
                            left_append = parswap.0.len();
                            parswap.0.append(&mut self.rtswap);
                            self.rtswap = parswap.0;
                            self.first = first; 
                        } 
                    } 

                    // удаляем в конце
                    if last < self.end {
                        self.end = last;
                        for i in 0..self.rtswap.len()-1 {
                            if self.rtswap[self.rtswap.len()-1-i].timestamp <= right_timestamp {
                                right_delete = i;
                                _= self.rtswap.split_off(self.rtswap.len()-right_delete);
                                break;
                            }
                        }
                    }

                    // удаляем в начале
                    if first > self.first { 
                        self.first = first;
                        for i in 0..self.rtswap.len()-1 {
                            if self.rtswap[i].timestamp >= left_timestamp {
                                left_delete = i;
                                self.rtswap = self.rtswap.split_off(left_delete);
                                break;
                            }
                        }
                    }
                },
            } // match     
            left_timestamp = self.rtswap.first().unwrap().timestamp; 
            right_timestamp = self.rtswap.last().unwrap().timestamp; 
            // let indendp_shift = self.indendp + left_append - left_delete; 
            if self.indendp + left_append < left_delete {
                inmode = SwapMod::Full;
            }

            match inmode {
                SwapMod::Full => {                   

                    let zerochanel: Vec<PlotPoint> = vec![]; 
                   // let zerochanelind: Vec<usize> = vec![];
                    let mut gachanels: Vec<IndPlotChanel> = (0..ACHANELS).map(|_x| { 
                        IndPlotChanel {
                            ppoints: zerochanel.clone(),
                        //    swapind: zerochanelind.clone(),
                        }
                    }).collect();
                    let mut gachanels2: Vec<IndPlotChanel> = (0..ACHANELS).map(|_x| { 
                        IndPlotChanel {
                            ppoints: zerochanel.clone(),
                         //   swapind: zerochanelind.clone(),
                        }
                    }).collect();
                    let mut gdchanels: Vec<IndPlotChanel> = (0..DCHANELS).map(|_x| { 
                        IndPlotChanel {
                            ppoints: zerochanel.clone(),
                         //   swapind: zerochanelind.clone(),
                        }
                    }).collect();      
        
                    
                    {
                        gachanels.iter_mut().enumerate().for_each(|i| {
                            let ret = self.aswap_calc(i.0, spp, 0, self.rtswap.len()-1);
                            *i.1 = ret.0;
                            gachanels2[i.0] = ret.1;
                        });
                        gdchanels.iter_mut().enumerate().for_each(|i| {
                            let ret = self.dswap_calc(i.0, spp, 0, self.rtswap.len()-1);
                            *i.1 = ret;
                        });
                    }
                    self.gachanels = gachanels; 
                    self.gachanels2 = gachanels2; 
                    self.gdchanels = gdchanels; 
                   // self.indstartp = 0; 
                    self.indendp = self.rtswap.len()-1; 
                },
                SwapMod::Add => { 
                    let indendp_shift = self.indendp + left_append - left_delete; 
                     
                    if right_append > 0 {    
                        //println!("right append {}", right_append);
                        //println!("s.is {}, istart {}, s.ie {}, iend {}", self.indstartp, indstart, self.indendp, indend); 
                        //println!("add right");
                        {
                            //добавляем точки в конце
                            for i in 0..ACHANELS {
                                let mut ret = self.aswap_calc(i, spp, indendp_shift, self.rtswap.len()-1);                          
                                while self.gachanels[i].ppoints.last().unwrap().x >= ret.0.ppoints[0].x && ret.0.ppoints.len() > 0 {
                                    ret.0.ppoints.remove(0); 
                                    ret.1.ppoints.remove(0); 
                                    //   ret.0.swapind.remove(0); 
                                    if ret.0.ppoints.len() == 0 {
                                        break;
                                    }
                                }
                                
                                if ret.0.ppoints.len() > 0
                                {
                                    self.gachanels[i].ppoints.append(&mut ret.0.ppoints);
                                    //   self.gachanels[i].swapind.append(&mut ret.0.swapind);
                                    self.gachanels2[i].ppoints.append(&mut ret.1.ppoints);
                                    //   self.gachanels2[i].swapind.append(&mut ret.1.swapind);
                                }
                            }
                            for i in 0..DCHANELS {
                                let mut ret = self.dswap_calc(i, spp, indendp_shift , self.rtswap.len()-1);
                                while self.gdchanels[i].ppoints.last().unwrap().x >= ret.ppoints[0].x && ret.ppoints.len() > 0 {
                                    ret.ppoints.remove(0); 
                                  //  ret.swapind.remove(0); 
                                    if ret.ppoints.len() == 0 {
                                        break;
                                    }
                                }     
                                if ret.ppoints.len() > 0 {
                                    self.gdchanels[i].ppoints.append(&mut ret.ppoints);
                                 //   self.gdchanels[i].swapind.append(&mut ret.swapind); 
                                }                        
                            }
                        }
                    }
                    if right_delete > 0 {
                        //удаляем точки в конце 
                        //println!("delete right");
                        for i in 0..ACHANELS {
                            let len = self.gachanels[i].ppoints.len();
                            for ii in 0..len-1 {
                                if self.gachanels[i].ppoints[len-1-ii].x <= right_timestamp as f64 {
                                    _= self.gachanels[i].ppoints.split_off(len-ii);
                                    _= self.gachanels2[i].ppoints.split_off(len-ii);
                                    break;
                                }
                            }                    
                        }      
                        for i in 0..DCHANELS {
                            let len = self.gdchanels[i].ppoints.len();
                            for ii in 0..len-1 {
                                if self.gdchanels[i].ppoints[len-1-ii].x <= right_timestamp as f64 {
                                    _= self.gdchanels[i].ppoints.split_off(len-ii);
                                    break;
                                }
                            }                           
                        }                
                    }

                    if left_append > 0 {
                        //println!("add left");
                        //println!("left append {}", left_append);
                        //println!("s.is {}, istart {}, s.ie {}, iend {}", self.indstartp, indstart, self.indendp, indend);  
                        //добавляем точки в начале 
                        {
                            for i in 0..ACHANELS {
                                let mut ret = self.aswap_calc(i, spp, 0, left_append); 
                                while self.gachanels[i].ppoints.first().unwrap().x <= ret.0.ppoints.last().unwrap().x && ret.0.ppoints.len() > 0 {
                                    ret.0.ppoints.remove(0); 
                                    ret.1.ppoints.remove(0);
                                 //  ret.0.swapind.remove(0); 
                                    if ret.0.ppoints.len() == 0 {
                                        break;
                                    }
                                }
                                if ret.0.ppoints.len() > 0 {
                                    ret.0.ppoints.append(&mut self.gachanels[i].ppoints);
                                  //  ret.0.swapind.append(&mut self.gachanels[i].swapind);
                                    ret.1.ppoints.append(&mut self.gachanels2[i].ppoints);
                                 //   ret.1.swapind.append(&mut self.gachanels2[i].swapind);
                                    self.gachanels[i]= ret.0; 
                                    self.gachanels2[i] = ret.1;
                                }
                            }
                            for i in 0..DCHANELS {
                                let mut ret = self.dswap_calc(i, spp, 0, left_append);
                                while self.gdchanels[i].ppoints.first().unwrap().x <= ret.ppoints.last().unwrap().x && ret.ppoints.len() > 0 {
                                    ret.ppoints.remove(0); 
                                 //   ret.swapind.remove(0); 
                                    if ret.ppoints.len() == 0 {
                                        break;
                                    }
                                }      
                                if ret.ppoints.len() > 0 {             
                                    ret.ppoints.append(&mut self.gdchanels[i].ppoints);
                                 //   ret.swapind.append(&mut self.gdchanels[i].swapind);
                                    self.gdchanels[i] = ret;  
                                }                          
                            }
                        }  
                    }
                    if left_delete > 0 {
                        //удаляем точки в начале
                         //println!("delete left");
                         for i in 0..ACHANELS {
                            let len = self.gachanels[i].ppoints.len();
                            for ii in 0..len-1 {
                                if self.gachanels[i].ppoints[ii].x >= left_timestamp as f64 {
                                    self.gachanels[i].ppoints = self.gachanels[i].ppoints.split_off(ii);
                                    self.gachanels2[i].ppoints = self.gachanels2[i].ppoints.split_off(ii);
                                    break;
                                }
                            }                    
                        }      
                        for i in 0..DCHANELS {
                            let len = self.gdchanels[i].ppoints.len();
                            for ii in 0..len-1 {
                                if self.gdchanels[i].ppoints[ii].x >= left_timestamp as f64 {
                                    self.gdchanels[i].ppoints = self.gdchanels[i].ppoints.split_off(ii);
                                    break;
                                }
                            }                           
                        }                          
                    }
                    self.indendp = indendp_shift + right_append - right_delete; 

                },
            } // match

            self.sppp = sppn; 

            Ok(())
        }

        fn aswap_calc(&self, chanel:usize, spp: i64, indstart: usize, indend: usize)->
            (IndPlotChanel, IndPlotChanel) {
  
            let zerochanel: Vec<PlotPoint> = vec![]; 
            //let zerochanelind: Vec<usize> = vec![];
            let mut pch = IndPlotChanel {
                    ppoints: zerochanel.clone(),
                //    swapind: zerochanelind.clone(),
                };
            let mut pch2 = IndPlotChanel {
                ppoints: zerochanel.clone(),
               // swapind: zerochanelind.clone(),
            };
                
            let mut chrtswap:Vec<(RtPoint, f64, f64, usize)> = vec![];
            let swapslise = &self.rtswap[indstart..indend]; 
            // берем крайние точки начало конец и середину только с изменениями
            chrtswap.push((swapslise[0].clone(), 0.0, 0.0, indstart));
            swapslise.iter().enumerate().for_each(|x| {
                if chrtswap.last().unwrap().0.achanels[chanel] != x.1.achanels[chanel] ||
                chrtswap.last().unwrap().0.timestamp + spp < x.1.timestamp
                {
                    chrtswap.push((x.1.clone(), 0.0, 0.0, indstart+x.0));
                }
            });
            chrtswap.push((swapslise.last().unwrap().clone(), 0.0, 0.0, indend));
            // параллельно вычисляем
            chrtswap.par_iter_mut().for_each(|x| {
                x.1 = (x.0.achanels[chanel] as f64)*self.convertrd.aconverter[chanel]*self.convertrd.areduction[chanel];
                x.2 = (x.0.achanels[chanel] as f64)*self.convertrd.aconverter[chanel]*self.convertrd.areduction2[chanel];
            });
            //передаем в отображение
            chrtswap.iter().for_each(|x| {
                    // добавляем в хвост
                    pch.ppoints.push(PlotPoint {
                        x: x.0.timestamp as f64,
                        y: x.1,
                    });
                   // pch.swapind.push(x.3);
                    pch2.ppoints.push(PlotPoint {
                        x: x.0.timestamp as f64,
                        y: x.2,
                    });
                  //  pch2.swapind.push(x.3);
            });
            (pch, pch2)
        }

        fn dswap_calc(&self, chanel:usize, spp: i64, indstart: usize, indend: usize)->IndPlotChanel {

            let zerochanel: Vec<PlotPoint> = vec![]; 
            //let zerochanelind: Vec<usize> = vec![];
            let mut pch = IndPlotChanel {
                    ppoints: zerochanel.clone(),
                   // swapind: zerochanelind.clone(),
                };

            let mut chrtswap:Vec<(RtPoint, f64, usize)> = vec![]; 
            let swapslise = &self.rtswap[indstart..indend]; 
            // берем крайние точки начало конец и середину только с изменениями
            chrtswap.push((swapslise[0].clone(), 0.0, indstart));
            swapslise.iter().enumerate().for_each(|x| {  
                if ((chrtswap.last().unwrap().0.digital & (0x1 << chanel)) >> chanel) != 
                    ((x.1.digital & (0x1 << chanel)) >> chanel) ||
                chrtswap.last().unwrap().0.timestamp + spp < x.1.timestamp
                {
                    let mut prv =  chrtswap.last().unwrap().0.clone(); 
                    prv.timestamp = x.1.timestamp; 
                    chrtswap.push((prv, 0.0, x.0));
                    chrtswap.push((x.1.clone(), 0.0, x.0));
                }
            });
            chrtswap.push((swapslise.last().unwrap().clone(), 0.0, indend));
            // параллельно вычисляем
            chrtswap.par_iter_mut().for_each(|x| {
                x.1 = (((x.0.digital & (0x1 << chanel)) >> chanel) as f64)*0.7+self.convertrd.dconverter[chanel]-1.0;
            });
            //передаем в отображение
            chrtswap.iter().for_each(|x| {
                pch.ppoints.push(PlotPoint {
                    x: x.0.timestamp as f64,
                    y: x.1,
                });
             //   pch.swapind.push(x.2);
            });
            pch
        }
    }

    /*fn swap_delete_edge(indstart: usize, indend: usize, pch: &mut IndPlotChanel) {
        if (indstart !=0 || indend != 0) &&  pch.swapind.len() > 1 {
            if indstart == 0 {
                // удаляем начало до indend
                while indend > pch.swapind[0] && pch.swapind.len() > 1 {
                    pch.ppoints.remove(0); 
                    pch.swapind.remove(0);
                }
            }
            if indend == 0 {
                 // удаляем конец до indstart
                 let mut last = pch.swapind.len()-1;
                 while indstart < pch.swapind[last] && pch.swapind.len() > 1 {
                    pch.ppoints.remove(last); 
                    pch.swapind.remove(last);
                    last = pch.swapind.len()-1;
                 }
            }
        }
    }*/


    /*fn micros_timestamp_to_bounds(start: i64, end: i64) { // -> RTbounds
        let startt = start; 
        // приводим к i32 диапазону для потенциала использования wgpu
        let endt = if end - start > 0xFFFFFFFF {
            end - 0xFFFFFFFF
        } else { end };
    }*/

    pub fn chartplot(appx: &mut Appx, ui: &mut Ui ) {          
        /* // первый вариант демо режима. использовался до написания Udp
        if appx.appstate.read().unwrap().get_rt() &&  appx.appstate.read().unwrap().get_rtdemo() {
            use chrono::Local;
            let timestamp: i64 = Local::now().timestamp_micros();
            {
                let dummypoint = RtPoint{
                    nan: false, 
                    timestamp,
                    achanels: (0..ACHANELS).map(|x| { 
                        let ret:f64 = if x == 0 {
                            ((timestamp as f64)/1000000.0).sin()*5.0
                        } else if x == 1 {
                            ((timestamp as f64)/1000000.0).cos()*6.0
                        } else {
                            0.0
                        };
                        ret
                    }).collect(),
                    digital: (timestamp as u64) / 55,
                };
                appx.rtmap.write().unwrap().rtmap_push(dummypoint);
            }
        }
        */


        let (scroll, _pointer_down, _modifiers) = ui.input(|i| {
            let scroll = i.events.iter().find_map(|e| match e {
                crate::egui::Event::MouseWheel {
                    unit: _,
                    delta,
                    modifiers: _,
                } => Some(*delta),
                _ => None,
            });
            (scroll, i.pointer.primary_down(), i.modifiers)
        });      

        let mut zoomctrlx = false; // разрешение зума по x
        let mut zoomctrly:Vec<bool> = (0..3).map(|_x|{false}).collect(); // разрешение зума по y
        // для графиков 0, 1, 2, 3
        let _mykey = ui.input(|key| {
            //key.modifiers.ctrl || key.modifiers.shift ||
            if key.modifiers.ctrl {
                if  key.key_down(crate::egui::Key::W) {
                    *zoomctrly.get_mut(2).unwrap() = true; 
                }
                if  key.key_down(crate::egui::Key::A) {
                    *zoomctrly.get_mut(0).unwrap() = true; 
                }
                if  key.key_down(crate::egui::Key::Z) {
                    *zoomctrly.get_mut(1).unwrap() = true; 
                }             
                let mut zoomy = false; 
                zoomctrly.iter().for_each(|x|{ if *x {zoomy = true;}});
                if !zoomy {
                    zoomctrlx = true; 
                }
            }
            else {
                if  key.key_down(crate::egui::Key::C) {
                    //println!("CCC ");
                    if appx.pcursor.is_some() { // запоминаем таймстамп курсора для CSV
                        let csvstart = appx.pcursor.unwrap().x as i64;
                        appx.csvtimestamp1 = Some(csvstart); 
                    } 
                    else {
                        appx.csvtimestamp1 = None;
                    }
                }   
                if  key.key_down(crate::egui::Key::V) {
                    //println!("CCC ");
                    if appx.pcursor.is_some() { // запоминаем таймстамп курсора для CSV
                        let csvstop = appx.pcursor.unwrap().x as i64;
                        appx.csvtimestamp2 = Some(csvstop); 
                    } 
                    else {
                        appx.csvtimestamp2 = None;
                    }
                }               
            }
            //if key.key_down( crate::egui::Key::)
        }) ; 

        //self.h = ui.available_height();
        //self.w = ui.available_width();
        let hrectui = ui.available_height();
        //let wrectui = ui.available_width();
        let mut hrectchart:Vec<f32> = vec![];
        match appx.appsettings.viewapp {
            ViewApp::OneAChart => { hrectchart.push(hrectui*1.0); hrectchart.push(hrectui*0.0); hrectchart.push(hrectui*0.0); hrectchart.push(hrectui*0.0); }, 
            ViewApp::DualAChart => { hrectchart.push(hrectui*0.5); hrectchart.push(hrectui*0.5); hrectchart.push(hrectui*0.0); hrectchart.push(hrectui*0.0); },
            ViewApp::MixChart => { hrectchart.push(hrectui*0.5); hrectchart.push(hrectui*0.0); hrectchart.push(hrectui*0.7); hrectchart.push(hrectui*0.0); },
            ViewApp::DualMixChart =>  { hrectchart.push(hrectui*0.45); hrectchart.push(hrectui*0.45); hrectchart.push(hrectui*0.7); hrectchart.push(hrectui*0.0); } ,
        }

        // Borrow slices с точками, заранее посчитанными
        // график 1
        /*let dat: std::sync::RwLockReadGuard<'_, crate::rtmap::rtmap::Rtmap>;
        'a: loop {
            let datr = appx.rtmap.read();
            if let Ok(mut trrd) = datr {
                dat
                break 'a;
            } 
        }*/ 

        let boundsxv:&mut [f64; 2] = &mut [0.0, 0.0];
        let boundsx = Rc::new(RefCell::new(boundsxv)); 
        let dat = appx.rtmap.read().unwrap();

        let lasttime = dat.rtmap.last().unwrap().timestamp; //self.realmap.lock().unwrap().last().unwrap().timestamp; 
        let earlytime: i64 = XAxisTime::get_early_timestamp(&appx.appsettings.axittime, lasttime);
       
        /*match appx.appsettings.axittime {
            XAxisTime::Seconds10 => { earlytime = lasttime-10*1000000; }, 
            XAxisTime::Seconds30 => { earlytime = lasttime-30*1000000; },
            XAxisTime::Minute => { earlytime = lasttime-60*1000000; },
            XAxisTime:: Minutes5 => { earlytime = lasttime-5*60*1000000; }, 
            XAxisTime::Minutes10 => { earlytime = lasttime-10*60*1000000; },
            XAxisTime::Minutes20 => { earlytime = lasttime-20*60*1000000; }, 
        };*/
        let w: f32; 


        { // borrow
            let swapdat = appx.mapsw.borrow(); 
            let mut vlines: Vec<Line> = vec![];
            for i in 0..ACHANELS {    
                if appx.appsettings.achanels[i].viewinchart {
                    let dds = swapdat.gachanels[i].ppoints.as_slice(); 
                    let ddline = Line::new(PlotPoints::Borrowed(dds))
                    .name( appx.appsettings.achanels[i].name.clone())
                    .color(appx.appsettings.achanels[i].color);
                    vlines.push(ddline);
                }
            } 
           /* 
        let mut vlines: Vec<Line> = vec![];
        for i in 0..ACHANELS {    
            if appx.appsettings.achanels[i].viewinchart {
                let dds = dat.gachanels[i].as_slice(); 
                let ddline = Line::new(PlotPoints::Borrowed(dds))
                .name( appx.appsettings.achanels[i].name.clone())
                .color(appx.appsettings.achanels[i].color);
                vlines.push(ddline);
            }
        } */ 
        // график 2
            let mut vlines2: Vec<Line> = vec![];
            for i in 0..ACHANELS {    
                if appx.appsettings.achanels[i].viewinchart2 {
                    let dds = &swapdat.gachanels2[i].ppoints.as_slice(); 
                    let ddline = Line::new(PlotPoints::Borrowed(dds))
                    .name( appx.appsettings.achanels[i].name.clone())
                    .color(appx.appsettings.achanels[i].color);
                    vlines2.push(ddline);
                }
            }
        /*let mut vlines2: Vec<Line> = vec![];
        for i in 0..ACHANELS {    
            if appx.appsettings.achanels[i].viewinchart2 {
                let dds = dat.gachanels2[i].as_slice(); 
                let ddline = Line::new(PlotPoints::Borrowed(dds))
                .name( appx.appsettings.achanels[i].name.clone())
                .color(appx.appsettings.achanels[i].color);
                vlines2.push(ddline);
            }
        }*/
        // график d
            let mut vlinesd: Vec<Line> = vec![];
           // let mut dchviews = 0; 
            for i in 0..DCHANELS {    
                if appx.appsettings.dchanels[i].viewindchart {
                 //   dchviews += 1;
                    let dds = &swapdat.gdchanels[i].ppoints.as_slice(); 
                    let ddline = Line::new(PlotPoints::Borrowed(dds))
                    .name( appx.appsettings.dchanels[i].name.clone())
                    .color(appx.appsettings.dchanels[i].color);
                    vlinesd.push(ddline);
                }
            }
        /*let mut vlinesd: Vec<Line> = vec![];
        for i in 0..DCHANELS {    
            if appx.appsettings.dchanels[i].viewindchart {
                let dds = dat.gdchanels[i].as_slice(); 
                let ddline = Line::new(PlotPoints::Borrowed(dds))
                .name( appx.appsettings.dchanels[i].name.clone())
                .color(appx.appsettings.dchanels[i].color);
                vlinesd.push(ddline);
            }
        }*/ 

   
        let link_group_id = ui.id().with("linked_demo1"); // группа объединения 
        let my_link_axis = crate::egui::Vec2b{ x:true, y:false}; // группа объединения курсора и оси Х

       // let firsttime = appx.rtmap.read().unwrap().rtmap.first().unwrap().timestamp as f64;

        // график 1 аналоговый
        //let prfmt: Rc<Option<f64>> = Rc::new(None);
        let my_plot = Plot::new("My Plot").legend(Legend::default()
            .position(Corner::LeftTop)
        )//.include_x(-5000)
        .x_axis_formatter(|x, rang| { 
            //let prfmt: RefCell<Option<f64>> = RefCell::new(None);
            let st = *rang.start(); 
            let en = *rang.end();
            //*boundsx.borrow_mut().get_mut(0).unwrap() = st; 
            boundsx.borrow_mut()[0] = st; 
            boundsx.borrow_mut()[1] = en; // вытаскиваем фактический диапазон по оси Х

            custom_x_axis_labels(x.value, x.step_size, appx.xformatter)

        })
        .height(hrectchart[0])
        .link_axis(link_group_id, my_link_axis)
        .link_cursor(link_group_id, my_link_axis)
        .allow_zoom(false)//zoomctrl)
        //.allow_drag(Vec2b{x:!zoomctrly, y:!zoomctrly, } )
        //.center_y_axis(true)
        .allow_scroll(false)
        .show_grid(Vec2b{x: false, y: true}
        );
        //.sense( crate::egui::Sense::hover())


        /*let mut pxytime:f64 = 0.0;
        if appx.pcursor.is_some() {
            pxytime = appx.pcursor.unwrap().x;
        }*/   
        //let xspacer  = my_plot.x_grid_spacer(spacer)
        //use crate::egui::Vec2;

        /*    let image = image::load_from_memory(image_data)?;
    let size = [image.width() as _, image.height() as _];
    let image_buffer = image.to_rgba8();
    let pixels = image_buffer.as_flat_samples();
    Ok(ColorImage::from_rgba_unmultiplied(
        size,
        pixels.as_slice(),
    ))*/
    
        //let texture = ui.ctx().load_texture("name", egui::ColorImage::example(), Default::default());
        let inner = my_plot.show( ui.into(), |plot_ui| {
            for _i in 0..vlines.len() { 
                plot_ui.line(vlines.pop().unwrap());
            }

            // /: &egui::TextureHandle
            /*let plot_image = egui_plot::PlotImage::new(
                texture.id(),
                egui_plot::PlotPoint::new(0.5, 0.5),
                egui::Vec2::new(10.0, 10.0),
            );
            plot_ui.image(plot_image);*/
      
            if appx.appstate.read().unwrap().get_rt() {
                //lot_ui.set_plot_bounds(egui_plot::PlotBounds::from_min_max([-5000.0,0.0], [0.0,1.0])); //(); 
                plot_ui.set_plot_bounds(egui_plot::PlotBounds::from_min_max([earlytime as f64, 0.0], [lasttime as f64,1.0])); //(); 
                plot_ui.set_auto_bounds(crate::egui::Vec2b { x:false, y:true}); 
            }
            else {
                if appx.abounds == 0 {
                    plot_ui.set_auto_bounds(crate::egui::Vec2b { x:false, y:false}); 
                }
                else {
                    plot_ui.set_auto_bounds(crate::egui::Vec2b { x:true, y:true}); 
                }
            }

            // получаем X координату
            appx.pcursor = plot_ui.pointer_coordinate();
            //plot_ui.set_plot_bounds(egui_plot::PlotBounds::from_min_max([0.0, -10.0], [1.0,10.0]));
            //plot_ui.set_auto_bounds(crate::egui::Vec2b { x:true, y:true}); 
            
            if let Some(mut scroll) = scroll {
            
            /*// if ctx.input(|i| i.key_down(crate::egui::Key::Num2)) {
                if reductiony == true  {
                    scroll = crate::egui::Vec2::splat(scroll.x + scroll.y);
                    if self.plot1line2max > 20.0 {
                        self.plot1line2max += (scroll.x*10.0) as f64;
                    }
                    if self.plot1line2max > 2.0 && self.plot1line2max < 20.0{
                        self.plot1line2max += scroll.x as f64;
                    }
                    if self.plot1line2max > 0.2 && self.plot1line2max < 2.0{
                        self.plot1line2max += (scroll.x/10.0) as f64;
                    }
                    if self.plot1line2max > 0.02 && self.plot1line2max < 0.2{
                        self.plot1line2max += (scroll.x/100.0) as f64;
                    }
                    if self.plot1line2max < 0.02 {
                        self.plot1line2max = 0.021;
                    }
                    
                // println!("mod1 event {}", self.plot1line2max );
                }*/

                scroll = crate::egui::Vec2::splat(scroll.x + scroll.y);
                if zoomctrlx {
                    let zoom_factor = crate::egui::Vec2::from([
                        (scroll.x * 1.0 / 10.0).exp(),
                        1.0, // (scroll.y * 1.0 / 10.0).exp(),
                    ]);
                    plot_ui.zoom_bounds_around_hovered(zoom_factor);
                }
                if *zoomctrly.get(0).unwrap() {
                    let zoom_factor = crate::egui::Vec2::from([
                        1.0, // (scroll.x * 1.0 / 10.0).exp(),
                        (scroll.y * 1.0 / 10.0).exp(),
                    ]);
                    //println!("exp {}", (scroll.y * 1.0 / 10.0).exp());
                    plot_ui.zoom_bounds_around_hovered(zoom_factor);
                }
            }
        });
        w = inner.response.rect.max.x - inner.response.rect.min.x; 
 
        // график 2 аналоговый 
        if hrectchart[1] > 0.0 {
            //let chcount = appx.rtmap.read().unwrap().convertrd
            let my_plot2 = Plot::new("My Plot2").legend(Legend::default()
                .position(Corner::LeftTop)
                )//.include_x(-5000)
                .height(hrectchart[1])
                .x_axis_formatter(|x, _rang| { 
                    custom_x_axis_labels(x.value, x.step_size, appx.xformatter)
                })
                .allow_zoom(false) //zoomctrl)
                .allow_scroll(false)
                .link_axis(link_group_id, my_link_axis)
                .link_cursor(link_group_id, my_link_axis)//.auto_bounds(abounds);
                .show_grid(Vec2b{x: false, y: true});

                let _inner2 = my_plot2.show(ui, |plot_ui| {
                    for _i in 0..vlines2.len() { 
                        plot_ui.line(vlines2.pop().unwrap());
                    }
                
                if appx.appstate.read().unwrap().get_rt() {
                    //lot_ui.set_plot_bounds(egui_plot::PlotBounds::from_min_max([-5000.0,0.0], [0.0,1.0])); //(); 
                    plot_ui.set_plot_bounds(egui_plot::PlotBounds::from_min_max([earlytime as f64, 0.0], [lasttime as f64,1.0])); //(); 
                    plot_ui.set_auto_bounds(crate::egui::Vec2b { x:false, y:true}); 
                }
                else {
                    if appx.abounds == 0 {
                        plot_ui.set_auto_bounds(crate::egui::Vec2b { x:false, y:false}); 
                    }
                    else {
                        plot_ui.set_auto_bounds(crate::egui::Vec2b { x:true, y:true}); 
                    }
                }

                if let Some(mut scroll) = scroll {
                    // if ctx.input(|i| i.key_down(crate::egui::Key::Num2)) {

                    scroll = crate::egui::Vec2::splat(scroll.x + scroll.y);
                    if zoomctrlx {
                        let zoom_factor = crate::egui::Vec2::from([
                            (scroll.x * 1.0 / 10.0).exp(),
                                1.0, // (scroll.y * 1.0 / 10.0).exp(),
                        ]);
                        plot_ui.zoom_bounds_around_hovered(zoom_factor);
                    }
                    if *zoomctrly.get(1).unwrap() == true  {
                        let zoom_factor = crate::egui::Vec2::from([
                            1.0, // (scroll.x * 1.0 / 10.0).exp(),
                            (scroll.y * 1.0 / 10.0).exp(),
                        ]);
                        plot_ui.zoom_bounds_around_hovered(zoom_factor);
                    }
                }
            });
        } // график 2

        
        // график d цифровой 
        if hrectchart[2] > 0.0 {
            let my_plotd = Plot::new("My Plotd").legend(Legend::default()
                .position(Corner::LeftTop)
                )//.include_x(-5000)
                .height(hrectchart[2])
                .x_axis_formatter(|x, _rang| { 
                    custom_x_axis_labels(x.value, x.step_size, appx.xformatter)
                })
                .allow_zoom(false) //zoomctrl)
                .allow_scroll(false)
                .link_axis(link_group_id, my_link_axis)
                .link_cursor(link_group_id, my_link_axis)
                .show_grid(Vec2b{x: false, y: true});

                let _inner2 = my_plotd.show(ui, |plot_ui| {
                    for _i in 0..vlinesd.len() { 
                        plot_ui.line(vlinesd.pop().unwrap());
                    }
                
                //let pxy = plot_ui.pointer_coordinate();
                //self.pxy = pxy;
                if appx.appstate.read().unwrap().get_rt() {
                    //lot_ui.set_plot_bounds(egui_plot::PlotBounds::from_min_max([-5000.0,0.0], [0.0,1.0])); //(); 
                    plot_ui.set_plot_bounds(egui_plot::PlotBounds::from_min_max([earlytime as f64, 0.0], [lasttime as f64, 32.0])); //(); 
                    plot_ui.set_auto_bounds(crate::egui::Vec2b { x:false, y:true}); 
                }
                else {
                    if appx.abounds == 0 {
                        plot_ui.set_auto_bounds(crate::egui::Vec2b { x:false, y:true}); 
                    }
                    else {
                        plot_ui.set_auto_bounds(crate::egui::Vec2b { x:true, y:true}); 
                    }
                }

                /*if let Some(mut scroll) = scroll {
                    // if ctx.input(|i| i.key_down(crate::egui::Key::Num2)) {

                    scroll = crate::egui::Vec2::splat(scroll.x + scroll.y);
                    if zoomctrlx {
                        let zoom_factor = crate::egui::Vec2::from([
                            (scroll.x * 1.0 / 10.0).exp(),
                                1.0, // (scroll.y * 1.0 / 10.0).exp(),
                        ]);
                        plot_ui.zoom_bounds_around_hovered(zoom_factor);
                    }
                    if *zoomctrly.get(1).unwrap() == true  {
                        let zoom_factor = crate::egui::Vec2::from([
                            1.0, // (scroll.x * 1.0 / 10.0).exp(),
                            (scroll.y * 1.0 / 10.0).exp(),
                        ]);
                        plot_ui.zoom_bounds_around_hovered(zoom_factor);
                    }
                }*/
            });
        } // график d
        } // borrow 
 
        if appx.abounds > 1 {
            if appx.abounds < 5 {
                appx.abounds += 1;
            }
            else {
                appx.abounds = 0;
            }
        }
               // println!("xmin {}, xmax {}", inner.response.rect.min.x, inner.response.rect.max.x); 
       // println!("xxmin {}, xxmax {}", inner.response.interact_rect.min.x, inner.response.rect.max.x); 
        //println!("bounds {}, {}", boundsx.borrow()[0], boundsx.borrow()[1]);  
        //let mut mapsw = RTmapsw::new(); 
        let bounds = [boundsx.borrow()[0] as i64, boundsx.borrow()[1] as i64]; 
        if appx.appstate.read().unwrap().get_rt() {
           // plot_ui.set_plot_bounds(egui_plot::PlotBounds::from_min_max([earlytime as f64, 0.0], [lasttime as f64,1.0])); //(); 
            let ret = appx.mapsw.borrow_mut().swap(&dat.rtmap, [earlytime, lasttime], w, appx.mode_swap.clone());
            if ret.is_ok() {
                appx.mode_swap = SwapMod::Add;
            }
        }
        else {
            if appx.abounds == 1 {
                let lasttime = dat.rtmap.last().unwrap().timestamp; 
                let earlytime: i64 = dat.rtmap.first().unwrap().timestamp; 
                let ret = appx.mapsw.borrow_mut().swap(&dat.rtmap, [earlytime, lasttime], w, SwapMod::Full); //appx.mode_swap.clone()
                if ret.is_ok() {
                    appx.mode_swap = SwapMod::Add;
                    if appx.abounds == 1 {
                        appx.abounds = 2;
                    };
                }  
            }
            else {
                let ret = appx.mapsw.borrow_mut().swap(&dat.rtmap, bounds, w, appx.mode_swap.clone()); //appx.mode_swap.clone()
                if ret.is_ok() {
                    appx.mode_swap = SwapMod::Add;
                }  
            }
        }
        

    }

    fn custom_x_axis_labels(xv: f64, xs:f64, typefmt: bool )->String {
                    //let prfmt: RefCell<Option<f64>> = RefCell::new(None);
        
                    if typefmt {
                        let dt = DateTime::from_timestamp_micros(xv as i64).expect("invalid timestamp"); 
                        let dts = format!("{}:",dt.format("%M:%S%.3f")); dts 
                    }
                    else {
                        /*let mut dts = format!(":"); 
                        let mut prt = *prft; 
                        if prt.is_none() {
                            prt = Some(xv); 
                        } 
                        else {
                            //let mut dts = format!("{}:",x.value-firsttime); 
                            dts = format!("{}:", xv - prt.unwrap());
                            prt = Some(xv); 
                            let stl = dts.len(); 
                            if stl > 3 {
                                dts.insert(stl-4, '.');
                            }
                            if stl > 6 {
                                dts.insert(stl-7, '.');
                            }
                        }*/
            
                        //let mut dts = format!("{}:",x.value-firsttime); 
                        let mut dts = format!("{}:", xs); 
                        let stl = dts.len(); 
                        if stl > 3 {
                            dts.insert(stl-4, '.');
                        }
                        if stl > 6 {
                            dts.insert(stl-7, '.');
                        }                        

                        dts 
                    }
    }
}