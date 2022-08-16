mod opt;

use sfml::audio::{
    SoundRecorder,
    SoundRecorderDriver,
    SoundStreamPlayer,
};

use structopt::StructOpt;
use opt::Opt;

use futures_lite::*;

// Std.
use std::sync::mpsc;

// External.
use sfml::audio::SoundStream;
use sfml::system::Time;

use smol_timeout::TimeoutExt;
use smol::{
    future::FutureExt,
    net::{TcpStream, TcpListener},
    io::AsyncWriteExt,
};
//use async_io::Timer;

// Std.
use std::collections::VecDeque;
use std::time::Duration;

// Custom

pub struct VoicePlayer {
    sample_receiver: mpsc::Receiver<Vec<i16>>,
    sample_rate: u32,
    sample_chunks: VecDeque<Vec<i16>>,
    finish_chunk: Vec<i16>,
}

impl VoicePlayer {
    pub fn new(sample_receiver: mpsc::Receiver<Vec<i16>>, sample_rate: u32) -> Self {
        VoicePlayer {
            sample_receiver,
            sample_rate,
            sample_chunks: VecDeque::new(),
            finish_chunk: vec![0i16; 1],
        }
    }
}

impl SoundStream for VoicePlayer {
    /// Returns `(chunk, keep_playing)`, where `chunk` is the chunk of audio samples,
    /// and `keep_playing` tells the streaming loop whether to keep playing or to stop.
    fn get_data(&mut self) -> (&mut [i16], bool) {
        if self.sample_chunks.len() > 0 {
            self.sample_chunks.pop_front();
        }

        if self.sample_chunks.len() == 0 {
            // wait, we need data to play
            let res = self
                .sample_receiver
                .recv_timeout(Duration::from_secs(10));
            if let Err(e) = res {
                match e {
                    mpsc::RecvTimeoutError::Timeout => {
                        // finish
                        self.sample_chunks.clear();
                        return (&mut self.finish_chunk, false);
                    }
                    _ => {
                        panic!("error: {} at [{}, {}]", e, file!(), line!());
                    }
                }
            }
            self.sample_chunks.push_back(res.unwrap());

            if self.sample_chunks.back().unwrap().len() == 0 {
                // zero-sized chunk means end of voice message
                // finished
                self.sample_chunks.clear();
                return (&mut self.finish_chunk, false);
            }
        }

        (&mut self.sample_chunks[0], true)
    }
    fn seek(&mut self, _offset: Time) {
        // dont need
    }
    fn channel_count(&self) -> u32 {
        1
    }
    fn sample_rate(&self) -> u32 {
        self.sample_rate
    }
}

pub struct VoiceRecorder {
    sample_sender: mpsc::Sender<Vec<i16>>,
    microphone_volume_multiplier: f64,
}

impl VoiceRecorder {
    pub fn new(sample_sender: mpsc::Sender<Vec<i16>>, microphone_volume: i32) -> Self {
        VoiceRecorder {
            sample_sender,
            microphone_volume_multiplier: microphone_volume as f64 / 100.0,
        }
    }
}

impl SoundRecorder for VoiceRecorder {
    fn on_process_samples(&mut self, samples: &[i16]) -> bool {
        let mut sample_vec = Vec::from(samples);

        // apply microphone multiplier
        sample_vec.iter_mut().for_each(|sample| {
            let mut new_sample = *sample as f64 * self.microphone_volume_multiplier;
            if new_sample > std::i16::MAX as f64 {
                new_sample = std::i16::MAX as f64;
            } else if new_sample < std::i16::MIN as f64 {
                new_sample = std::i16::MIN as f64;
            }
            *sample = new_sample as i16;
        });

        // ignore send errors
        let _result = self.sample_sender.send(sample_vec);

        true
    }
}

/*
struct ReadBuf<T, const N: usize> {
    buf: [T; N],
    index: usize,
}

impl<T, const N: usize> ReadBuf<T, N> {
    /// Copy data into the ReadBuf, return the number of elements copied
    fn read(self, data: &[T]) -> usize {
        let size = data.len();

        self.buf[self.index..].copy_from_slice(data);
        /*
        let mut i = 0;
        while self.index < N {
            buf[idx] = data[i];
        }

        if size + self.index > buf_size {
            for i in 0..(idx - size) {
                buf[idx] = chunk[i];
                idx += 1;
            }
            tx.send(buf);

            idx = 0;
            buf[idx]
        }
        */
    }
}
*/


fn main() {
    /*
    let (sc, rc) = std::sync::mpsc::channel();
    let mut rec = VoiceRecorder::new(sc, 500);
    let mut driver = SoundRecorderDriver::new(&mut rec);
    */

    let opts = Opt::from_args();

    //println!("Starting recording");
    //driver.start(44100);

    smol::block_on(async move {
        if opts.call {
            let (sc, rc) = std::sync::mpsc::channel();
            let mut rec = VoiceRecorder::new(sc, 500);
            let mut driver = SoundRecorderDriver::new(&mut rec);
            driver.start(44100);
            println!("Starting recording");

            let mut stream = TcpStream::connect("127.0.0.1:8080").await.unwrap();
            println!("Connected to peer");

            const buf_size: usize = 8946;
            let mut buf = [0; buf_size];
            //let mut idx = 0;
            let (tx, rx) = std::sync::mpsc::channel();

            let mut queue = vec![];

            smol::spawn(async move {
                for chunk in rc {
                    queue.extend_from_slice(&chunk);

                    if queue.len() > buf_size {
                        let msg: Vec<i16> = queue.drain(0..buf_size).collect();
                        let ser = bincode::serialize(&msg).unwrap();
                        tx.send(ser).unwrap();
                    }
                }
            }).detach();

            for chunk in rx {
                //let ser = bincode::serialize::<Vec<i16>>(&chunk).unwrap();
                //let dec = bincode::deserialize::<Vec<i16>>(&ser).unwrap();
                //println!("{:?}", dec);
                //let ser_len = bincode::serialize(&(ser.len() as u32)).unwrap();
                stream.write_all(&chunk).await.unwrap();
                //println!("ser len: {}", ser_len.len());

                //let byte_count = stream.write_all(&ser).await.unwrap();

                println!("wrote {} bytes", chunk.len());
            }
            /*
            }.or(async {
                smol::Timer::after(Duration::from_secs(3)).await;
            }));
            */
            driver.stop();
        }
        else {
            let listener = TcpListener::bind("127.0.0.1:8080").await.unwrap();
            let mut incoming = listener.incoming();

            while let Some(stream) = incoming.next().await {
                let mut stream = stream.unwrap();
                //let decoded = stream.map(|bc| bincode::deserialize(bc));

                println!("peer connected");
                let (sec, rec) = std::sync::mpsc::channel();
                //let msec = std::sync::Mutex::new(sec);
                //use std::sync::{Arc, Mutex};
                let msec = sec.clone();
                //let msec = Arc::new(Mutex::new(sec));//std::sync::Mutex::new(std::sync::Arc::new(sec));
                //let msec = Arc::new(sec);
                let f_deserial = smol::spawn(async move {
                    loop {
                        //let mut buf = vec![];
                        //let mut ser_len_buf = [0u8; 4];
                        //stream.read_exact(&mut ser_len_buf).await.unwrap();
                        //let ser_len = bincode::deserialize::<u32>(&ser_len_buf).unwrap();

                        //let mut buf = Vec::with_capacity(ser_len as usize);
                        const ser_buf_size: usize = 17900;
                        let mut buf = [0u8; ser_buf_size];
                        stream.read_exact(&mut buf).await.unwrap();
                        let decoded: Vec<i16> = bincode::deserialize(&buf).unwrap();
                        sec.send(decoded).unwrap();
                    //let bytes = stream.bytes().map(|bc| ;
                        //let decoded: Vec<i16> = bincode::deserialize(&buf).unwrap();
                    //while let bytes = stream.read(&mut buf).await.unwrap() {
                        //let mut buf = vec![0u8; 128];
                        //println!("read {} bytes", ser_len);
                        //println!("read {} bytes", stream.read(&mut buf).await.unwrap());
                        //println!("{decoded:?}");
                        //let decoded: Vec<i16> = bincode::deserialize(&buf).unwrap();
                        //msec.lock().unwrap().send(decoded).unwrap();
                        //msec.send(decoded).unwrap();
                    }
                });

                let mut voice_player = VoicePlayer::new(rec, 44100);
                let mut player = SoundStreamPlayer::new(&mut voice_player);
                player.play();
                println!("playing");
                f_deserial.await
            }
        }
    });

    //driver.stop();
    /*
    println!("Recorded audio");
    println!("Playing back..");

    let mut voice_player = VoicePlayer::new(rc, 44100);
    let mut player = SoundStreamPlayer::new(&mut voice_player);
    player.play();
    std::thread::sleep(Duration::from_secs(3));
    */


    //});
}
