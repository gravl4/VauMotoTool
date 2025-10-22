#[allow(dead_code)] // отключаем предупреждение о неиспользуемом коде
pub mod compressor {

    use rayon::prelude::*;
    use std::sync::{ Mutex, Arc};
    use crate::rtmap::rtmap::{RtPoint, ACHANELS}; //, DCHANELS

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
    pub fn swap_compressor(inrtmap: &[RtPoint], spp: usize)-> Result<(Vec<RtPoint>, usize), &str> {
        let mut parmap: Vec<&[RtPoint]> = vec![];
        if inrtmap.len() == 0 || spp == 0 {
            return Err("compressor index error"); 
        }
        let ndiv:usize;
        if  inrtmap.len() > 10000//200000  
        {
            ndiv = 100; // количество участков разбивки
            let sldiv = inrtmap.len()/ndiv;
            // разбиваем срез на участки для параллельного вычесления
            let mut mfirst = 0; 
            for i in 0..ndiv {
                if i < ndiv -1 {
                    parmap.push(&inrtmap[mfirst..(i+1)*sldiv]);
                }
                else {
                    parmap.push(&inrtmap[mfirst..]); // добавляем хвост
                }
                //parmap.push(&inrtmap[mfirst..(i+1)*sldiv]);
                mfirst = (i+1)*sldiv;
            }
           /* if mfirst != inrtmap.len() {
                if inrtmap.len() > 1 {
                    parmap.push(&inrtmap[mfirst..inrtmap.len()-1]); // добавляем хвост
                }
                else {
                    parmap.push(&inrtmap[mfirst..]); // добавляем хвост
                }
            }*/
        }
        else {
            // точек мало просто один поток
            ndiv = 1;  
           // parmap.push(&inrtmap[0..inrtmap.len()-1]);
            parmap.push(&inrtmap[0..]); // добавляем хвост 
        }
        // результат работы 
        let parrtswap:Vec<Vec<RtPoint>> = (0..ndiv).into_iter().map(|_x| {
            let swap:Vec<RtPoint> = vec![];
            swap
        }).collect();
        let resswap = Arc::new(Mutex::new(parrtswap));
        let vreti: Vec<usize> = (0..ndiv).into_iter().map(|_x| {0}).collect(); 
        let resreti = Arc::new(Mutex::new(vreti));

        let _press = parmap.par_iter().enumerate().for_each(|px| {
           // if px.0 == 100 {
           //     println!("par {}", px.0); 
           // }

            let rtmap = *px.1;
            let mut rtswap:Vec<RtPoint> = vec![];

            let mut t: usize = 0;
            let mut achs = Aminmax::new_vec(); 
            let mut dig = rtmap[0].digital; 
            let mut digs: u64 = 0;
            let mut digf = dig; 
            let mut timef = rtmap[0].timestamp; 
            //let mut reti:usize = 0;

            (0..ACHANELS).into_iter().for_each(|j| {
                achs[j].max = rtmap[0].achanels[j]; 
                achs[j].min = rtmap[0].achanels[j]; 
                achs[j].minfirst = true; 
            });

            for i in 0..rtmap.len() {
                (0..ACHANELS).into_iter().for_each(|x| {
                    if rtmap[i].achanels[x] > achs[x].max {
                        achs[x].max = rtmap[i].achanels[x]; 
                        achs[x].minfirst = true; 
                    }
                    if rtmap[i].achanels[x] < achs[0].min {
                        achs[x].min = rtmap[i].achanels[x]; 
                        achs[x].minfirst = false;
                    }
                });
                
                if (dig ^ rtmap[i].digital) != 0 {
                    digs |= dig ^ rtmap[i].digital;
                    dig = rtmap[i].digital; 
                }

                //println!("test time {}, {}, {}" , rtmap[i].timestamp, timef, (rtmap[i].timestamp - timef)); 
                if (t >= spp) || (rtmap[i].timestamp - timef) as usize >= spp   { 
                    if spp == 1 || t == 0 || (rtmap[i].timestamp - timef) as usize >= spp {
                        rtswap.push(RtPoint {
                            nan:false, 
                            timestamp: rtmap[i].timestamp,
                            achanels: (0..ACHANELS).map(|x| { 
                                rtmap[i].achanels[x]
                                 }).collect(),
                            digital: rtmap[i].digital,
                        });
                        if  (rtmap[i].timestamp - timef) as usize >= spp {
                            rtswap.push(RtPoint {
                                nan:false, 
                                timestamp: rtmap[i].timestamp,
                                achanels: (0..ACHANELS).map(|x| { 
                                    rtmap[i].achanels[x]
                                     }).collect(),
                                digital: rtmap[i].digital,
                            });                         
                        }
                    }
                    else {
                        rtswap.push(RtPoint {
                            nan:false, 
                            timestamp: timef, //xtbound[0]+(spp*t) as i64,
                            achanels: (0..ACHANELS).map(|x| { 
                                if achs[x].minfirst {
                                    achs[x].min    
                                } else {
                                    achs[x].max
                                }
                                 }).collect(),
                            digital: digf ^ digs,
                        });
   
    
                        let timee = (rtmap[i].timestamp - timef)/2 ; 
                        rtswap.push(RtPoint {
                            nan:false, 
                            timestamp: timef+timee,
                            achanels: (0..ACHANELS).map(|x| { 
                                if achs[x].minfirst {
                                    achs[x].max    
                                } else {
                                    achs[x].min
                                }
                                 }).collect(),
                            digital: rtmap[i].digital,
                        });
                    }

                    (0..ACHANELS).into_iter().for_each(|x| {
                        achs[x].max = rtmap[i].achanels[x]; 
                        achs[x].min = rtmap[i].achanels[x]; 
                        achs[x].minfirst = true; 
                    });

                    timef = rtmap[i].timestamp; 
                    digs = 0;
                    dig = rtmap[i].digital;
                    digf = dig;

                    t = 0;
                    resreti.lock().unwrap()[px.0] = i;
                }
                else {
                    t+= 1;
                    if spp > 1 && i == rtmap.len()-1 {
                        if (t >= spp) || (rtmap[i].timestamp - timef) as usize >= spp   { 
                            resreti.lock().unwrap()[px.0] = i;
                        };
                        rtswap.push(RtPoint {
                            nan:false, 
                            timestamp: timef, //xtbound[0]+(spp*t) as i64,
                            achanels: (0..ACHANELS).map(|x| { 
                                if achs[x].minfirst {
                                    achs[x].min    
                                } else {
                                    achs[x].max
                                }
                                 }).collect(),
                            digital: digf ^ digs,
                        });
   
                        rtswap.push(RtPoint {
                            nan:false, 
                            timestamp: rtmap[i].timestamp,
                            achanels: (0..ACHANELS).map(|x| { 
                                rtmap[i].achanels[x]
                                 }).collect(),
                            digital: rtmap[i].digital,
                        });                      
                    }
                }
            }
            resswap.lock().unwrap()[px.0] = rtswap; 
        });

        /*resreti.lock().unwrap().iter().enumerate().for_each(|x| {
            println!("reti {}, {}", x.0, x.1);
        });*/

        let sldiv = inrtmap.len()/ndiv;
        let mut rtswap:Vec<RtPoint> = vec![];
        for i in 0..ndiv {
            rtswap.append(&mut resswap.lock().unwrap()[i]);
        }
        let reti = *resreti.lock().unwrap().last().unwrap() + (ndiv-1)*sldiv; 
        Ok((rtswap, reti))
    }

}//mod