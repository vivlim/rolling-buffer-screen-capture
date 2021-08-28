use std::{convert::TryInto, fs::{self, File}, io::{ErrorKind::WouldBlock, Write}, path::Path, sync::{Arc, Mutex}, thread, time::{Duration, Instant}};

use bounded_vec_deque::BoundedVecDeque;
use ffmpeg_next::format::Pixel;
use hotkey::modifiers;
use libav_frame_encoder::encoder::{OutputArgs, VideoArgs};
use notify_rust::Notification;
use scrap::{Capturer, Display};
use turbojpeg::{Compressor, Image};

mod encode;

fn get_capturable_display() -> Capturer {
    for display in Display::all().unwrap() {
        match Capturer::new(display) {
            Ok(cap) => {
                return cap;
            },
            Err(e) => {
                eprintln!("Couldn't get display because of {:?}", e);
            }

        }

    }
    panic!("no capturable displays");
}

struct RollingScreenCaptures {
    frames: BoundedVecDeque<Vec<u8>>,
    encoder_args: Option<OutputArgs>,
}

fn main() {
    let target_fps  = 15;
    let frame_duration = Duration::from_secs(1) / target_fps;

    let buffer_duration = Duration::from_secs(30);

    let num_frames: usize = (buffer_duration.as_millis() / frame_duration.as_millis()).try_into().unwrap();

    let mut captures: Arc<Mutex<RollingScreenCaptures>> = Arc::new(Mutex::new(RollingScreenCaptures {
        frames: BoundedVecDeque::with_capacity(num_frames, num_frames),
        encoder_args: None,
    }));

    let mut captures_for_loop_thread = captures.clone();
    let mut captures_for_key_handler = captures.clone();

    let capture_loop = thread::spawn(move ||{
        let captures = captures_for_loop_thread;
        let mut capturer = get_capturable_display();
        let width = capturer.width();
        let height = capturer.height();

        // report every this many frames
        let report_frequency = target_fps * 5;
        let mut loop_idx = 0;
        let mut max_compressed_len = 0;
        let mut max_uncompressed_len = 0;

        let mut compressor = Compressor::new().unwrap();
        compressor.set_quality(80);
        compressor.set_subsamp(turbojpeg::Subsamp::Sub2x2);

        loop {
            let start_time = Instant::now();

            // Get capture frame if there is one.
            let current_frame = match capturer.frame() {
                Ok(captured_frame_buffer) => {
                    let stride = &captured_frame_buffer.len() / height;
                    let input_image = Image {
                        pixels: &captured_frame_buffer as &[u8],
                        width: width,
                        pitch: stride,
                        height: height,
                        format: turbojpeg::PixelFormat::BGRA,
                    };
                    match compressor.compress_to_vec(input_image) {
                        Ok(buf) => {

                            max_compressed_len = usize::max(max_compressed_len, buf.len());
                            max_uncompressed_len = usize::max(max_uncompressed_len, captured_frame_buffer.len());
                            Some(buf)
                        },
                        Err(e) => panic!("error compressing {:?}", e),
                    }
                },
                Err(error) => {
                    if error.kind() == WouldBlock {
                        // No frame right now.
                        None
                    } else {
                        return Err(error);
                    }

                }
            };

            match current_frame {
                Some(current_frame) => {
                    // Put it in the ring
                    let mut captures = captures.lock().unwrap();
                    captures.frames.push_back(current_frame);

                    if loop_idx % report_frequency == 0 { // every ~5 seconds report memory usage
                        let total_mem: usize = captures.frames.iter().map(|f| f.len()).sum();
                        println!("largest frame: {}KB -> {}KB ({:.1}% compression ratio). total mem use: {}MB for {} slots", max_uncompressed_len/1024, max_compressed_len/1024, (max_compressed_len as f32 / max_uncompressed_len as f32) * 100.0, total_mem/1024/1024, captures.frames.len());
                        max_compressed_len = 0;
                        max_uncompressed_len = 0;
                    }

                    if let None = captures.encoder_args {
                        captures.encoder_args = Some(OutputArgs::Video(VideoArgs {
                            pixel_format: Pixel::BGRA,
                            fps: target_fps,
                            width: width.try_into().unwrap(),
                            height: height.try_into().unwrap(),
                        }));
                    }
                },
                None => {
                }
            }

            let time_ran = Instant::now().duration_since(start_time);
            if time_ran < frame_duration {
                thread::sleep(Duration::max(frame_duration - time_ran, Duration::ZERO));
            } else {
                println!("falling behind");
            }
            loop_idx += 1;
        }
    });

    let mut hk = hotkey::Listener::new();
    /*
    hk.register_hotkey(modifiers::SHIFT | modifiers::SUPER, 'T' as u32, move || {
        let mut captures = captures_for_key_handler.clone();
        let dir = format!("rolling_buffer_{}", chrono::Local::now().format("%Y%m%d-%H_%M_%S"));
        let dir = Path::new(&dir);
        println!("dumping jpegs to dir; {}", &dir.display());
        fs::create_dir(&dir).unwrap();
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

            Notification::new()
                .summary(format!("Wrote {} frames as jpegs", index).as_str())
                .body(format!("To a folder \"{}\".", dir.display()).as_str())
                .show().unwrap();
        }


    }).unwrap();
*/
    hk.register_hotkey(modifiers::SHIFT | modifiers::SUPER, 'R' as u32, move || {
        let mut captures = captures_for_key_handler.clone();
        let path = format!("rolling_buffer_{}.mp4", chrono::Local::now().format("%Y%m%d-%H_%M_%S"));
        let path = Path::new(&path).to_path_buf();
        println!("dumping video to dir; {}", &path.display());

        let (encoder_args, jpegs) = {
            let mut captures = captures.lock().unwrap();

            let encoder_args = match &captures.encoder_args {
                Some(encoder_args) => encoder_args.clone(),
                None => {
                    panic!("encoder args not set, cannot do anything");
                }
            };
            let len = captures.frames.len();
            let jpegs: Vec<Vec<u8>> = captures.frames.drain(0..len).collect();
            (encoder_args, jpegs)
        };

        Notification::new()
            .summary("Beginning video encode")
            .body(format!("Writing {} frames to \"{}\".", jpegs.len(), path.display()).as_str())
            .show().unwrap();

        match encode::encode_jpegs_to_video(path.clone(), encoder_args, jpegs).join() {
            Ok(Ok(_)) => {
                Notification::new()
                    .summary("Finished video encode")
                    .body(format!("Wrote to \"{}\".", path.display()).as_str())
                    .show().unwrap();
            },
            Err(e) => {
                Notification::new()
                    .summary("Error during video encode")
                    .body(format!("{:?}", e).as_str())
                    .show().unwrap();
            },
            Ok(Err(_)) => {
                Notification::new()
                    .summary("Error during video encode")
                    .body("See console for details :(")
                    .show().unwrap();
            }
        }



    }).unwrap();

    println!("Press shift+super+R to dump the last {} seconds into a folder (in the form of jpegs for now)", buffer_duration.as_secs());

    Notification::new()
        .summary(format!("Recording last {} seconds at up to {} fps", buffer_duration.as_secs(), target_fps).as_str())
        .body("shift+super+R will export a folder of jpegs")
        .show().unwrap();
    hk.listen();

    capture_loop.join().unwrap().unwrap()
}