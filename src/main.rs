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
use async_io::Timer;

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

async fn bind_tcp(addr: &str) -> std::io::Result<TcpStream> {
    let listener = TcpListener::bind(addr).await.unwrap();
    let mut incoming = listener.incoming();

    incoming.next().await.unwrap()
}

use smol::net::UdpSocket;
async fn send_pings(sock: UdpSocket, addr: &str) {
    loop {
        //println!("{:?}", sock.send(b"hades").await);
        sock.send(b"hades").await;
        Timer::after(Duration::from_millis(1000)).await;
        //println!("try again");
    }
}

async fn echo(socket: UdpSocket) {
    let mut buf = [0; 4096];
    //loop {
        let sock = socket.clone();
        println!("recv1 {:?}", socket.recv_from(&mut buf).await);
        match socket.recv_from(&mut buf).await {
            Ok((amt, src)) => {
                //smol::spawn(async move {
                //thread::spawn(move || {
                    println!("Handling connection from {}", &src);
                    let buf = &mut buf[..amt];
                    buf.reverse();
                    println!("echoing");
                    sock.send_to(&buf, &src).await.expect("error sending");
                    println!("eh?");
                //}).detach();
            },
            Err(err) => {
                eprintln!("Err: {}", err);
            }
        }
    //}
}

async fn listen_for_connection(sock: UdpSocket) {
    let mut buf = [0; 5];
    while let byte_count = sock.recv(&mut buf).await {
        //if buf[..byte_count] == b"hermes" {
        println!("count: {byte_count:?}");
        if &buf == b"hades" {
            println!("Thank Thoth!");
            break;
        }
    }
}


fn main() {
    let (sc, rc) = std::sync::mpsc::channel();
    let mut rec = VoiceRecorder::new(sc, 500);
    let mut driver = SoundRecorderDriver::new(&mut rec);

    let opts = Opt::from_args();

    /*
    println!("Starting recording");
    driver.start(44100);

    const buf_size: usize = 8946;
    let (tx, rx) = std::sync::mpsc::channel();

    let mut queue = vec![];
    let rec_relay = smol::unblock(move || {
        println!("rec relay task init");
        for chunk in rc {
            queue.extend_from_slice(&chunk);

            if queue.len() > buf_size {
                let msg: Vec<i16> = queue.drain(0..buf_size).collect();
                let ser = bincode::serialize(&msg).unwrap();
                tx.send(ser).unwrap();
            }
        }
    });
    */

    let addr = &opts.address;//"127.0.0.1:8080";

    //let sock = smol::unblock(|| UdpSocket::bind(&opts.from));

    smol::block_on(async move {
        let sock = UdpSocket::bind(&opts.from).await.unwrap();
        sock.connect(addr).await.unwrap();

        //if opts.call {
        let mut buf = [0; 8];
        println!("local: {:?}", sock.local_addr());
        println!("peer: {:?}", sock.peer_addr());
        //println!("awaiting udp packet..");

        println!("{:?}", sock.send(b"freedom").await);
        //sock.recv(&mut buf).await.unwrap();
        //println!("{buf:?}");
        //send_pings(sock.clone(), addr)
        //    .or(async { sock.recv_from(&mut buf).await.unwrap(); println!("heyo"); }).await;
        //println!("got it!");
        //} else {

        send_pings(sock.clone(), addr)
            .or(listen_for_connection(sock)).await

        //echo(sock.clone()).await
            //.and(async { println!("{:?}", sock.recv_from(&mut buf).await.unwrap()); }).await;
        //}
    });

    /*
    let mut stream = smol::block_on(async move {
        if opts.call {
            TcpStream::connect(addr).await
        } else {
            bind_tcp(addr).await
        }.unwrap()
    });

    let mut stream2 = stream.clone();

    println!("peer connected");
    let (sec, rec) = std::sync::mpsc::channel();
    //let f_deserial = smol::spawn(async move {
    let f_deserial = async move {
        println!("kek");
        loop {
            const ser_buf_size: usize = 17900;
            let mut buf = [0u8; ser_buf_size];
            stream.read_exact(&mut buf).await.unwrap();
            let decoded: Vec<i16> = bincode::deserialize(&buf).unwrap();
            //println!("sending decoded");
            sec.send(decoded).unwrap();
        }
    };
    //});

    //let write_stream = smol::unblock(|| smol::block_on(async move {
    let write_stream = async move {
        println!("write stream task init");
        for chunk in rx {
            stream2.write_all(&chunk).await.unwrap();
            //println!("wrote {} bytes", chunk.len());
        }
    };
    //}));

    let mut voice_player = VoicePlayer::new(rec, 44100);
    let mut player = SoundStreamPlayer::new(&mut voice_player);
    player.play();
    println!("playing");


    //write_stream.await
    smol::block_on(async move {
        write_stream.or(f_deserial).or(rec_relay).await
    });

    driver.stop();
    */
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
