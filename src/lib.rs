use std::sync::Arc;

use cpal::StreamConfig;

pub fn transcription() {
    let mut model = coqui_stt::Model::new("english/model.tflite").expect("unable to create model");
    model
        .enable_external_scorer("english/huge-vocabulary.scorer")
        .expect("unable to read scorer");
    let model = Arc::new(model);
    let desired_rate = cpal::SampleRate(model.get_sample_rate() as u32);
    // let streaming = Arc::new(Mutex::new(
    //     model.into_streaming().expect("trouble streaming the model"),
    // ));

    use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
    let host = cpal::default_host();
    let device = host
        .default_input_device()
        .expect("no input device available");
    let supported_configs_range = device
        .supported_output_configs()
        .expect("error while querying configs");
    let supported_config = supported_configs_range
        .filter(|c| c.channels() == 1)
        .filter(|c| c.sample_format() == cpal::SampleFormat::I16)
        .filter(|c| c.min_sample_rate() <= desired_rate)
        .filter(|c| c.max_sample_rate() >= desired_rate)
        .next()
        .expect("No input at desired sample rate")
        .with_sample_rate(desired_rate);
    let mut config: StreamConfig = supported_config.into();
    let mut silent_buffers = 0;
    let mut last_length = 0;
    let latency_samples = 320 * desired_rate.0 as usize / 1000;
    config.buffer_size = cpal::BufferSize::Fixed(latency_samples as u32);
    let mut samples_taken = 0;
    let mut streaming: Option<coqui_stt::Stream> = None;

    // let streaming_copy = streaming.clone();
    let stream = device
        .build_input_stream(
            &config,
            move |data: &[i16], _: &cpal::InputCallbackInfo| {
                if streaming.is_none() {
                    streaming = Some(
                        coqui_stt::Stream::from_model(model.clone())
                            .expect("unable to create stream?!"),
                    );
                }
                let mut finished_phrase = false;
                if let Some(stream) = &mut streaming {
                    samples_taken += data.len();
                    stream.feed_audio(data);
                    if samples_taken > latency_samples {
                        samples_taken -= latency_samples;
                        if let Ok(s) = stream.intermediate_decode() {
                            if s.len() > 1 {
                                if s.len() != last_length {
                                    last_length = s.len();
                                    silent_buffers = 0;
                                } else {
                                    silent_buffers += 1;
                                    if silent_buffers > 3 {
                                        finished_phrase = true;
                                        println!("Said: {s}");
                                        // for (n, t) in m.transcripts().iter().enumerate() {
                                        //     let confidence = t.confidence();
                                        //     let mut v = String::new();
                                        //     let mut time = 0.0;
                                        //     for s in t.tokens().iter() {
                                        //         v.push_str(s.text().as_ref());
                                        //         time = s.start_time();
                                        //     }
                                        //     println!("{n:2}: {confidence:5.3} {v:?} at {time:.2}");
                                        // }
                                    }
                                }
                            }
                        }

                        if finished_phrase {
                            silent_buffers = 0;
                            last_length = 0;
                            let old = std::mem::replace(&mut streaming, None);
                            drop(old);
                        }
                    }
                }
            },
            move |err| {
                // react to errors here.
                panic!("stream error: {}", err);
            },
        )
        .expect("error creating stream");
    stream.play().unwrap();
    loop {
        std::thread::sleep(std::time::Duration::from_secs_f64(1.0e3));
    }

    // loop {
    //     if let Ok(m) = streaming
    //         .lock()
    //         .unwrap()
    //         .intermediate_decode_with_metadata(10)
    //     {
    //         println!();
    //         for (n, t) in m.transcripts().iter().enumerate() {
    //             let confidence = t.confidence();
    //             let mut v = String::new();
    //             let mut time = 0.0;
    //             for s in t.tokens().iter() {
    //                 v.push_str(s.text().as_ref());
    //                 time = s.start_time();
    //             }
    //             println!("{n:2}: {confidence:5.3} {v:?} at {time:.2}");
    //         }
    //         // if s.len() > 0 {
    //         //     println!("Said: {s}");
    //         // }
    //     }
    //     std::thread::sleep(std::time::Duration::from_secs_f64(1.0));
    // }
}
