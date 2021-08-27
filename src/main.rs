use std::{convert::TryInto, fs::{self, File}, io::{BufWriter, ErrorKind::WouldBlock, Write}, path::Path, sync::{Arc, Mutex}, thread, time::{Duration, Instant, SystemTime}};


use bounded_vec_deque::BoundedVecDeque;
use hotkey::modifiers;
use scrap::{Capturer, Display};
use turbojpeg::{Compressor, Image};

fn main() {
    let target_fps  = 15;
    let frame_duration = Duration::from_secs(1) / target_fps;

    let buffer_duration = Duration::from_secs(30);

    let num_frames_to_batch = 1;

    let num_frames: usize = (buffer_duration.as_millis() / frame_duration.as_millis()).try_into().unwrap();
    let num_batches = num_frames / num_frames_to_batch;

    let mut frames = Arc::new(Mutex::new(BoundedVecDeque::with_capacity(num_batches, num_batches)));
    let mut frames_for_loop_thread = frames.clone();
    let mut frames_for_key_handler = frames.clone();

    let capture_loop = thread::spawn(move ||{
        let frames = frames_for_loop_thread;
        let display = Display::primary().expect("Couldn't find primary display.");
        let width = display.width();
        let height = display.height();
        let mut capturer = Capturer::new(display).expect("Couldn't begin capture.");


        let mut reusable_buffer: Option<Vec<u8>> = None;

        let mut raw_frame_size = None;

        let mut compressor = Compressor::new().unwrap();
        compressor.set_quality(80);
        compressor.set_subsamp(turbojpeg::Subsamp::Sub2x2);

        loop {
            let start_time = Instant::now();

            let mut destination_buffer = vec![];

            //let mut writer = snap::write::FrameEncoder::new(destination_buffer);

            for i in 0..num_frames_to_batch {

                match capturer.frame() {
                    Ok(captured_frame_buffer) => {
                        match raw_frame_size {
                            Some(_) => {
                                if let Some(raw_frame_size) = raw_frame_size {
                                    if raw_frame_size != captured_frame_buffer.len() {
                                        panic!("frame size has changed in the middle of recording {} != {}", raw_frame_size, captured_frame_buffer.len());
                                    }
                                }
                            }
                            None => {
                                raw_frame_size = Some(captured_frame_buffer.len());
                            }
                        }
                        let input_image = Image {
                            pixels: &captured_frame_buffer as &[u8],
                            width: width,
                            pitch: width * 4,
                            height: height,
                            format: turbojpeg::PixelFormat::BGRA,
                        };
                        match compressor.compress_to_vec(input_image) {
                            Ok(buf) => {
                                destination_buffer = buf;
                            },
                            Err(e) => panic!("error compressing {:?}", e),
                        }
                        //let uncompressed_bytes_written = writer.write(&captured_frame_buffer).unwrap();
                    },
                    Err(error) => {
                        if error.kind() == WouldBlock {
                            // Keep spinning until a frame is ready.
                            thread::sleep(frame_duration);
                            continue;
                        } else {
                            return Err(error);
                        }

                    }

                };

            }
            //writer.flush().unwrap();
            //destination_buffer = writer.into_inner().unwrap();

            let compressed_len = destination_buffer.len();
            let uncompressed_len = raw_frame_size.unwrap() * num_frames_to_batch;

            let mut frames = frames.lock().unwrap();
            frames.push_back(destination_buffer);

            let time_ran = Instant::now().duration_since(start_time);
            if time_ran < frame_duration {
                thread::sleep(Duration::max(frame_duration - time_ran, Duration::ZERO));
            } else {
                println!("falling behind");
            }

            // get total memory in use
            let total_mem: usize = frames.iter().map(|f| f.len()).sum();
            println!("compressed: {} -> {} ({:.1}%). total use: {}MB for {} slots", uncompressed_len, compressed_len, (compressed_len as f32 / uncompressed_len as f32) * 100.0, total_mem/1024/1024, frames.len());
        }
        Ok(())
    });

    let mut hk = hotkey::Listener::new();
    hk.register_hotkey(modifiers::SHIFT | modifiers::SUPER, 'R' as u32, move || {
        let mut frames = frames_for_key_handler.clone();
        let dir = format!("rolling_buffer_{}", chrono::Local::now().format("%Y%m%d-%H_%M_%S"));
        let dir = Path::new(&dir);
        println!("dumping jpegs to dir; {}", &dir.display());
        fs::create_dir(dir).unwrap();
        {
            let mut frames = frames.lock().unwrap();
            let frames_len = frames.len();
            let mut index = 0;
            for f in frames.drain(0..frames_len) {
                let filename = dir.join(format!("frame_{}.jpg", index));
                let mut file = File::create(filename).unwrap();
                file.write_all(&f);
                index += 1;
            }

        }


    }).unwrap();
    hk.listen();

    capture_loop.join().unwrap().unwrap()
}