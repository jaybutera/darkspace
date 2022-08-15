use sfml::audio::{
    SoundRecorder,
    SoundRecorderDriver,
    SoundStreamPlayer,
};

use opts::Opt;

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

fn main() {
    let (sc, rc) = std::sync::mpsc::channel();
    let mut rec = VoiceRecorder::new(sc, 500);
    let mut driver = SoundRecorderDriver::new(&mut rec);

    let opts = Opt::from_args();

    println!("Starting recording");
    driver.start(44100);

    smol::block_on(async move {
        let mut stream = TcpStream::connect("127.0.0.1:8080").await.unwrap();
        for chunk in rc {
            let ser = bincode::serialize(&chunk).unwrap();
            stream.write(&ser);
        }
    }.or(async {
        smol::Timer::after(Duration::from_secs(3)).await;
    }));
    //}.timeout(Duration::from_secs(3)));

    driver.stop();
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
