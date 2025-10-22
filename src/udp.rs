#[allow(dead_code)] // отключаем предупреждение о неиспользуемом коде
pub mod udp {
   // use std::f32::DIGITS;
    use std::sync::{ RwLock, Arc}; // Mutex 
    use std::sync::mpsc;
    use chrono::Local; //, DateTime,}

    use tokio::net::UdpSocket;
    use tokio::time::{self, Duration}; //sleep,

    //use bincode::{DefaultOptions, Options};

    //use crate::{rtmap::rtmap::{Rtmap, RtPoint, ACHANELS, DCHANELS}}; //{self,  
   // use crate::rtmap::rtmap::Rtpoin;
    use crate::app::app::AppState; // DChanelSettings}
    use crate::rtmap::rtmap::{RtPoint, ACHANELS}; //, DCHANELS

    pub const DEF_UDP_PORT: usize = 55512;  // default udp port
    pub const UDP_PACK_SIZE: usize = 44;  // длинна пакета 

    pub struct UdpProc {
        handle: std::thread::JoinHandle<Result<(), Box<dyn std::error::Error + Send + Sync>>>
    }
    struct Hsync {
        synh: bool, 
        synh_tp: i64,
        udp_synh_tp: i64,
    }

    async fn udp_process(port:usize, appstate: Arc<RwLock<AppState>>, fb: mpsc::Sender<Result<Vec<RtPoint>, Box<dyn std::error::Error + Send + Sync>>>)->Result<(), Box<dyn std::error::Error+ Send + Sync>> {
        //async runtime udp приема
        //println!("0.0.0.0:{}", port);
        let sock = UdpSocket::bind(format!("0.0.0.0:{}", port)).await?;
        // let sock = UdpSocket::bind("0.0.0.0:55512").await?;
       // let mut udp_pack_count = 0;
        let mut synh = Hsync {
            synh: false, 
            synh_tp: 0, 
            udp_synh_tp: 0, 
        };
        let mut tempbuf:Vec<RtPoint> = vec![];
        let mut prtime: i64 = Local::now().timestamp_millis();
        loop {
            //sleep(Duration::from_millis(500)).await;
          //  let timeout = if  appstate.read().unwrap().get_rtdemo() { 100 } else { 150 };
            let mut buf = [0; 2000]; // буфер приема UDP пакетов
            match time::timeout(
                Duration::from_millis(200), // тайм аут приема пакетов
                sock.recv_from(&mut buf),
            )
            .await
            {
                Ok(ret) => {
                    // пакет пришел вовремя
                   // udp_pack_count += 1;    
                   // if appstate.read().unwrap().get_rt() {
                   //     println!("UDP pack: {}", udp_pack_count);
                   // }
                    
                    if appstate.read().unwrap().get_rt()== false {
                        return Ok(()); // завершаем fn
                    }

                    // парсим пакеты здесь
                    if let Ok(mut points) = parcer(&buf, ret.unwrap().0) {
                        /*if synh.synh == false {
                            use chrono::Local;
                            let timestamp: i64 = Local::now().timestamp_micros();
                            synh.synh_tp = timestamp;
                            let udp_first = points.first().unwrap().timestamp;
                            let udp_last = points.last().unwrap().timestamp;
                            synh.synh = true;
                            synh.udp_synh_tp = udp_first; 
                            points.iter_mut().for_each(|x|{ //.rev()
                               // x.timestamp = synh.synh_tp - (x.timestamp - udp_last);
                               x.timestamp = synh.synh_tp + (x.timestamp - synh.udp_synh_tp);
                            });
                            synh.synh_tp = points.last().unwrap().timestamp;
                            synh.udp_synh_tp = udp_last; 
                        }
                        else {
                            use chrono::Local;
                            let timestamp: i64 = Local::now().timestamp_micros();
                            let udp_first = points.first().unwrap().timestamp;
                            let xtimestamp = synh.synh_tp + (udp_first - synh.udp_synh_tp);
                            if timestamp-xtimestamp > 10 { // синхронизация отстала
                                synh.synh_tp = timestamp;
                                synh.udp_synh_tp = udp_first; 
                            }
                            //let udp_first = points.first().unwrap().timestamp;
                            points.iter_mut().for_each(|x|{
                                x.timestamp = synh.synh_tp + (x.timestamp - synh.udp_synh_tp);
                            });                           
                        }*/
                        use chrono::Local;
                        let timestamp: i64 = Local::now().timestamp_micros();
                        let udp_first = points.first().unwrap().timestamp;
                        //let udp_last = points.last().unwrap().timestamp;
                        for i in 0..points.len() {
                            if points[i].timestamp - synh.udp_synh_tp  > 900 {
                                synh.udp_synh_tp = points[i].timestamp;
                                points[i].timestamp = timestamp + (points[i].timestamp-udp_first);
                                synh.synh_tp = points[i].timestamp;
                            }
                            else {
                                let begin = points[i].timestamp;
                               // println!("begin {}, add {}", synh.synh_tp, (points[i].timestamp-synh.udp_synh_tp));
                                points[i].timestamp = synh.synh_tp + (points[i].timestamp-synh.udp_synh_tp);
                                synh.udp_synh_tp = begin;
                                synh.synh_tp = points[i].timestamp;
                            }      
                               
                        }
                        /*
                        points.iter_mut().for_each(|x|{
                            x.timestamp = timestamp + (x.timestamp - udp_first);
                        });*/

                        //udp_pack_count += points.len();
                       // println!("lpoints {}", udp_pack_count);
                       points.iter().for_each(|x| {
                            tempbuf.push(x.clone());
                       });
                       // _= fb.send(Ok(points)); 
                    }
                    else {
                        // обработать таймаут без нормальных пакетов
                    }
                
                    let dur =  appstate.read().unwrap().get_duration();
                    let timestamp: i64 = Local::now().timestamp_millis();
                    if (timestamp - prtime) > (dur+dur/2) as i64 {
                        prtime = timestamp;
                        _= fb.send(Ok(tempbuf));
                        tempbuf = vec![];
                    }
                }
                Err(_e) => {  
                    // таймаут прихода пакета 
                    if appstate.read().unwrap().get_rt()== false {
                        return Ok(()); // завершаем fn
                    }

                    synh.synh = false; 

                   // udp_pack_count =  0;    
                    if  appstate.read().unwrap().get_rtdemo() {
                        use chrono::Local;
                        let mut timestamp: i64 = Local::now().timestamp_micros();
                        { // 1ms here
                            let mut points: Vec<RtPoint> = vec![];
                            // 200ms 
                            let ncount = 200; //1-200ms, 200-1ms, 2000-100us, 20000-10us ,200000-1us
                            for _i in 0..ncount {
                                let dummypoint = RtPoint{
                                    nan: false, 
                                    timestamp,
                                    achanels: (0..ACHANELS).map(|x| { 
                                        let ret:i16 = if x == 0 {
                                            (((timestamp as f64)/1000000.0).sin()*10000.0) as i16
                                        } else if x == 1 {
                                            (((timestamp as f64)/1000000.0).cos()*20000.0) as i16
                                        } else {
                                            0
                                        };
                                        ret
                                    }).collect(),
                                    digital: (timestamp as u64) / 55,
                                };            
                                points.push(dummypoint);
                                timestamp += 200000/ncount; 
                            }

                            _= fb.send(Ok(points)); 
                            /*'a: loop {
                                if let Ok(mut trwr) = rtmap.write() {
                              //     trwr.rtmap_push(dummypoint);
                                   println!("ok");
                                    break 'a;
                                } 
                            }*/ 

                           
                        }
                    }
                }
            }

        }
    }

    impl UdpProc {
        pub fn get_finish(&self) -> bool {
            self.handle.is_finished()
        }

        //portin: mpsc::Receiver<usize>, fb: mpsc::Sender<Result<(),&str>> ,
        pub fn new_process(port:usize, appstate: Arc<RwLock<AppState>>, fb: mpsc::Sender<Result<Vec<RtPoint>, Box<dyn std::error::Error + Send + Sync>>>) {
           // let (port_tx, port_rx) = mpsc::channel::<usize>();

            /*let port:usize;
            if let Ok(read) = portin.try_recv() {
                port = read;
            } else { port = DEF_UDP_PORT; };
             JoinHandle<Result<(), Box<dyn std::error::Error + Send + Sync>>>
             */

            let _handle = std::thread::spawn(move || { //-> Result<(), Box<dyn std::error::Error + Send + Sync>> 
                //let mut i = 0; 
                loop {
                    let asf = tokio::runtime::Builder::new_multi_thread()
                    .enable_all()
                    .build() //?
                    .unwrap()
                    .block_on( udp_process(port, appstate.clone(), fb.clone()) );

                    if asf.is_err() {
                       //println!("udp error");
                       // сообщаем что поток упал
                       let der = asf.err().unwrap();
                       _= fb.send(Err(der));
                       break;// Err(der);
                    }
                    else {
                        break; // просто закрываем поток
                    }
                }
              //  println!("udp thread exit");
              //  std::process::exit(0x0);
            }); 
            //println!("udp thread create");
        }
        
    }

    fn parcer(buf: &[u8], blen: usize)-> Result<Vec<RtPoint>, ()> {
        let mut points: Vec<RtPoint> = vec![];
        //let rcv = ret.unwrap();  rcv.0
        if  blen % UDP_PACK_SIZE == 0 { // данные кратно пакетам 
            let mut ind:usize = 0;  
            while ind <= blen-UDP_PACK_SIZE { // перебираем пакеты по 44 байта
                let bytes = &buf[ind..ind+UDP_PACK_SIZE];

                let mut achanels:Vec<i16> = vec![]; 

                if bytes[0] != 0x55 && bytes[1] != 0x33  {
                    break; // заголовок не верный
                }    
                let mut indin:usize  = 2;

            
                // timestamp
                let timestamp: i64;
                //let lebytes:&[u8] = &[bytes[indin+3], bytes[indin+2], bytes[indin+1], bytes[indin]]; 
                let lebytes:[u8; 8] = [0,0,0,0,bytes[indin],bytes[indin+1],bytes[indin+2],bytes[indin+3]];
                timestamp = i64::from_be_bytes(lebytes);
                indin += 4; 
                /*let tempret:Result<(i64, usize), _> = bincode::decode_from_slice(lebytes, bincode::config::standard());
                if tempret.is_ok()  
                {
                    timestamp = tempret.unwrap().0;
                  //  println!("udp timestamp {}",  timestamp);
                    indin += 4; 
                }
                else { 
                    println!("error timestamp");
                    break; 
                }*/

                // аналаговые каналы
                for _i in 0..ACHANELS {
                    let lebytes:[u8; 2] = [bytes[indin],bytes[indin+1]];
                    let tempret = i16::from_be_bytes(lebytes);
                    achanels.push(tempret);
                    indin += 2; 
                    /*let lebytes:&[u8] = &[bytes[indin+1], bytes[indin]];
                    println!("lebytes {} {}", lebytes[1], lebytes[0]);
                    let tempret:Result<(i16, usize), _> = bincode::decode_from_slice(lebytes, bincode::config::standard());
                    if tempret.is_ok() 
                    { 
                        achanels.push(tempret.unwrap().0);
                        indin += 2; 
                    }
                    else { 
                        println!("error achanel {}", i);
                        break; 
                    }*/
                }
                if achanels.len() != ACHANELS { break; }

                // d каналы
                let lebytes:[u8; 8] = [0,0,0,0,bytes[indin],bytes[indin+1],bytes[indin+2],bytes[indin+3]];
                let digital = u64::from_be_bytes(lebytes);
                //indin += 4; 
                /*let lebytes:&[u8] = &[bytes[indin+3], bytes[indin+2], bytes[indin+1], bytes[indin]];
                let tempret:Result<(u32, usize), _> = bincode::decode_from_slice(lebytes, bincode::config::standard());
                if tempret.is_ok()  {
                    digital = tempret.unwrap().0 as u64; 
                   // indin += 4; 
                }
                else { 
                    println!("error dchanels ");
                    break; 
                } */            

                points.push(RtPoint {
                    nan: false,
                    timestamp,
                    achanels, 
                    digital, 
                });

                ind += UDP_PACK_SIZE; 
            }
            if ind == blen && ind != 0 {
                // парсер сработал нормально 
                return Ok(points); 
            }
        }
        Err(())
    }

}


/*
use std::error::Error;
use std::thread;

fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    let udp_thread = thread::spawn(udp);
    
    udp_thread.join().unwrap()?;

    Ok(())
}

fn udp() -> Result<(), Box<dyn Error + Send + Sync>> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;

    rt.block_on(async {
        Err("test".into())
    })
}

вариант 2
use std::thread;
use std::time::Duration;
//use std::process;
use std::error::Error;

async fn f() -> Result<(), Box<dyn Error>> {
    loop {
        println!("hello");
        thread::sleep(Duration::from_secs(1));
        Err::<(), &str>("sf")?;
    }
}

fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    let handle_udp = thread::spawn(|| -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        loop {
            tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .unwrap()
                .block_on(f());
        }
    });

    let handle_ui = thread::spawn(|| -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        loop {
            //thread::sleep(Duration::from_millis(1));
            thread::sleep(Duration::from_secs(5));
            Err("dfd")?;
        }
    });
    handle_ui.join().unwrap()?;
    handle_udp.join().unwrap()?;

    Ok(())
}
*/