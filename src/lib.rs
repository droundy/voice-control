use std::sync::Arc;

use crate::parser::Parser;

pub mod parser;

pub mod keys;

const VAD_SAMPLES: u32 = 16 * 30; // 30 ms at 16 kHz.  10 and 20 are also options.
#[allow(non_snake_case)]
fn get_audio_input_16kHz<F: FnMut(&[i16]) + Send + 'static>(mut callback: F) -> ! {
    const REQUIRED_RATE: cpal::SampleRate = cpal::SampleRate(16000);
    const THREE_RATE: cpal::SampleRate = cpal::SampleRate(3 * REQUIRED_RATE.0);
    use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
    let use_config = move |device: cpal::Device, config: cpal::SupportedStreamConfig| -> ! {
        if config.sample_format() == cpal::SampleFormat::I16
            && config.sample_rate() == REQUIRED_RATE
        {
            let stream = device
                .build_input_stream(
                    &config.into(),
                    move |data: &[i16], _: &cpal::InputCallbackInfo| callback(data),
                    move |err| {
                        // react to errors here.
                        panic!("stream error: {}", err);
                    },
                )
                .expect("error creating stream");
            stream.play().unwrap();
        } else if config.sample_format() == cpal::SampleFormat::F32
            && config.sample_rate() == REQUIRED_RATE
        {
            let mut ints = Vec::new();
            let stream = device
                .build_input_stream(
                    &config.into(),
                    move |data: &[f32], _: &cpal::InputCallbackInfo| {
                        ints.clear();
                        ints.extend(
                            data.iter()
                                .copied()
                                .map(|f| (f * (i16::MAX as f32 - 1.0)) as i16),
                        );
                        callback(&ints);
                    },
                    move |err| {
                        // react to errors here.
                        panic!("stream error: {}", err);
                    },
                )
                .expect("error creating stream");
            stream.play().unwrap();
        } else if config.sample_format() == cpal::SampleFormat::F32
            && config.sample_rate() == THREE_RATE
        {
            let mut ints = Vec::new();
            let stream = device
                .build_input_stream(
                    &config.into(),
                    move |data: &[f32], _: &cpal::InputCallbackInfo| {
                        ints.clear();
                        ints.extend(
                            data.iter()
                                .step_by(3)
                                .copied()
                                .map(|f| (f * (i16::MAX as f32 - 1.0)) as i16),
                        );
                        callback(&ints);
                    },
                    move |err| {
                        // react to errors here.
                        panic!("stream error: {}", err);
                    },
                )
                .expect("error creating stream");
            stream.play().unwrap();
        } else {
            panic!("Unsupported configuration!");
        }
        loop {
            std::thread::sleep(std::time::Duration::from_secs_f64(1.0e3));
        }
    };
    let host = cpal::default_host();
    for device in host.input_devices().unwrap() {
        println!("\ndevice is {:?}\n", device.name());
        let supported_configs_range = device
            .supported_input_configs()
            .expect("error while querying configs");
        if let Some(supported_config_range) = supported_configs_range
            .filter(|c| c.channels() == 1)
            .filter(|c| c.sample_format() == cpal::SampleFormat::I16)
            .filter(|c| c.min_sample_rate() <= REQUIRED_RATE)
            .filter(|c| c.max_sample_rate() >= REQUIRED_RATE)
            .next()
        {
            use_config(
                device,
                supported_config_range.with_sample_rate(REQUIRED_RATE),
            );
        }
    }
    println!("No device supports i16 sampling");
    for device in host.input_devices().unwrap() {
        let supported_configs_range = device
            .supported_input_configs()
            .expect("error while querying configs");
        if let Some(supported_config_range) = supported_configs_range
            .filter(|c| c.channels() == 1)
            // .filter(|c| c.sample_format() == cpal::SampleFormat::F32)
            .filter(|c| c.min_sample_rate() <= REQUIRED_RATE)
            .filter(|c| c.max_sample_rate() >= REQUIRED_RATE)
            .next()
        {
            use_config(
                device,
                supported_config_range.with_sample_rate(REQUIRED_RATE),
            );
        }
    }
    println!("No device supports f32 sampling at 16 kHz");
    for device in host.input_devices().unwrap() {
        let supported_configs_range = device
            .supported_input_configs()
            .expect("error while querying configs");
        if let Some(supported_config_range) = supported_configs_range
            .filter(|c| c.channels() == 1)
            // .filter(|c| c.sample_format() == cpal::SampleFormat::F32)
            .filter(|c| c.min_sample_rate() <= THREE_RATE)
            .filter(|c| c.max_sample_rate() >= THREE_RATE)
            .next()
        {
            use_config(device, supported_config_range.with_sample_rate(THREE_RATE));
        }
    }
    println!("No device supports f32 sampling at 8*16 kHz");
    for device in host.input_devices().unwrap() {
        println!("\ndevice is {:?}\n", device.name());
        let supported_configs_range = device
            .supported_input_configs()
            .expect("error while querying configs");
        for scr in supported_configs_range {
            println!("   {:?}", scr);
        }
    }
    panic!("No supported audio config!");
}

pub fn voice_control() {
    let mut model = coqui_stt::Model::new("english/model.tflite").expect("unable to create model");
    model
        .enable_external_scorer("english/huge-vocabulary.scorer")
        .expect("unable to read scorer");
    let model = Arc::new(model);
    // let streaming = Arc::new(Mutex::new(
    //     model.into_streaming().expect("trouble streaming the model"),
    // ));

    assert_eq!(model.get_sample_rate(), 16000);
    let vad = std::sync::Mutex::new(webrtc_vad::Vad::new_with_rate_and_mode(
        webrtc_vad::SampleRate::Rate16kHz,
        webrtc_vad::VadMode::Quality,
    ));

    let mut have_sound = false;
    let new_stream =
        move || coqui_stt::Stream::from_model(model.clone()).expect("unable to create stream?!");
    let mut stream = new_stream();
    let mut collected_data: Vec<i16> = Vec::new();

    let rules = Arc::new(std::sync::Mutex::new(parser::my_rules()));
    // let streaming_copy = streaming.clone();
    get_audio_input_16kHz(move |data: &[i16]| {
        collected_data.extend(data);
        if collected_data.len() < VAD_SAMPLES as usize {
            return;
        }
        let mut vad = vad.lock().unwrap();
        if collected_data
            .chunks_exact(VAD_SAMPLES as usize)
            .any(|data| vad.is_voice_segment(data).expect("wrong size data sample"))
        {
            stream.feed_audio(&collected_data);
            have_sound = true;
        } else {
            if have_sound {
                stream.feed_audio(&collected_data);
                println!("Got some audio to process...");
                let x = std::mem::replace(&mut stream, new_stream())
                    .finish_stream_with_metadata(32)
                    .unwrap()
                    .to_owned();
                let mut best = 0;
                let mut best_vec = Vec::new();
                for c in x.transcripts().iter() {
                    let mut words = String::new();
                    for w in c.tokens().iter().map(|t| &t.text) {
                        words.push_str(w.as_ref());
                    }
                    let original_words = words.split_whitespace().collect::<Vec<_>>();
                    let mut words = &original_words[..];
                    let mut goodness = 0;
                    while let Some((_, rest)) = rules.lock().unwrap().parse(words) {
                        words = rest;
                        goodness += 1;
                    }
                    println!("{goodness:2}: {original_words:?}");
                    if goodness > best {
                        best = goodness;
                        best_vec = original_words.iter().map(|w| w.to_string()).collect();
                    }
                }
                let words = best_vec.iter().map(|w| w.as_str()).collect::<Vec<_>>();
                let mut words = &words[..];
                while let Some((a, rest)) = rules.lock().unwrap().parse(&words[..]) {
                    assert!(rest.len() < words.len());
                    words = rest;
                    a.run();
                }
            }
            have_sound = false;
        }
        collected_data.clear();
    });
}
