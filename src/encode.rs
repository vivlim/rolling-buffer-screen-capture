use std::{convert::TryInto, path::{Path, PathBuf}, thread::{self, JoinHandle}, time::Duration};

use crossbeam_channel::SendError;
use libav_frame_encoder::{encoder::{OutputArgs}, sink::{Frame, FrameData, Sink, VideoPlane}};
use turbojpeg::{Decompressor, Image, PixelFormat};

pub fn encode_jpegs_to_video(path: PathBuf, output_args: OutputArgs, jpegs: Vec<Vec<u8>>) -> JoinHandle<Result<(), ()>>{

    return thread::spawn(move || {
        let mut frame_num = 0;

        let (width, height) = match &output_args {
            OutputArgs::Video(video_args) => (video_args.width as usize, video_args.height as usize),
            _ => { panic!("unhandled"); }
        };

        let pitch = 4 * width;

        let mut encoder = JpegVideoEncoder::new(path, output_args).unwrap();
        let mut decompressor = Decompressor::new().unwrap();
        for jpeg in jpegs {
            // Decode the jpeg into raw pixel buffer
            let header = decompressor.read_header(&jpeg).unwrap();
            if width != header.width || height != header.height {
                panic!("image size has changed");
            }
            let mut pixels = vec![0; (4 * width * height).try_into().unwrap()];
            let image = Image {
                pixels: pixels.as_mut_slice(),
                width,
                pitch,
                height,
                format: PixelFormat::BGRA
            };
            decompressor.decompress_to_slice(&jpeg, image).unwrap();

            println!("pushing frame {}", frame_num);
            encoder.sink.input.send(Frame {
                data: FrameData::Video(VideoPlane {
                    data: pixels,
                    width,
                    height,
                    pitch,
                }),
                frame_number: frame_num
            }).unwrap();

            frame_num += 1;

            while encoder.sink.output.len() > 5 { // Don't put too many frames in the queue at a time, since we are expanding them out into full pixel buffers... it could use a lot of ram if it happens too fast
                thread::sleep(Duration::from_millis(100));
            }
        }

        encoder.sink.input.send(Frame {
            data: FrameData::End,
            frame_number: frame_num
        }).unwrap();

        encoder.encoder_thread.join().unwrap().unwrap();
        Ok(())
    })
}

struct JpegVideoEncoder {
    pub sink: Sink<Frame<FrameData>>,
    encoder_thread: JoinHandle<Result<(), ()>>
}

impl JpegVideoEncoder {
    pub fn new(path: PathBuf, output_args: OutputArgs) -> Result<Self, SendError<Frame<FrameData>>>{
        let sink: Sink<Frame<FrameData>> = Default::default();
        let encoder_thread = libav_frame_encoder::encoder::start_thread(sink.output.clone(), path);

        sink.input.send(Frame {
            data: FrameData::Configure(output_args),
            frame_number: 0,
        })?;

        Ok(JpegVideoEncoder {
            sink,
            encoder_thread,
        })
    }
}