use std::sync::Arc;

pub mod keys;
pub mod parser;

// pub mod keys;

pub mod desktop_control;
use desktop_control::Action;
use parser::{Error, IsParser, Parser};

const VAD_SAMPLES: u32 = 16 * 30; // 30 ms at 16 kHz.  10 and 20 are also options.
const REQUIRED_RATE: cpal::SampleRate = cpal::SampleRate(16000);
#[allow(non_snake_case)]
fn get_audio_input_16kHz<F: FnMut(&[i16]) + Send + 'static>(mut callback: F) -> ! {
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
            loop {
                std::thread::sleep(std::time::Duration::from_secs_f64(1.0e3));
            }
        } else if config.sample_format() == cpal::SampleFormat::F32
            && config.sample_rate() == REQUIRED_RATE
        {
            println!("Running with f32... at 16 kHz");
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
            loop {
                std::thread::sleep(std::time::Duration::from_secs_f64(1.0e3));
            }
        } else if config.sample_format() == cpal::SampleFormat::F32
            && config.sample_rate() == THREE_RATE
        {
            let mut ints = Vec::new();
            println!("running at higher hz");
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
            loop {
                std::thread::sleep(std::time::Duration::from_secs_f64(1.0e3));
            }
        } else {
            panic!("Unsupported configuration!");
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

pub fn voice_control(commands: impl Fn() -> Parser<Action>) {
    let mut model = coqui_stt::Model::new("english/model.tflite").expect("unable to create model");
    model
        .enable_external_scorer("english/huge-vocabulary.scorer")
        .expect("unable to read scorer");
    let model_commands = commands();
    model
        .enable_callback_scorer(move |s| {
            if let Err(Error::Wrong) = model_commands.parse(s) {
                -10.0
            } else {
                0.0
            }
        })
        .expect("unable to apply callback scorer");
    let model = Arc::new(model);

    assert_eq!(model.get_sample_rate(), REQUIRED_RATE.0 as i32);
    let vad = std::sync::Mutex::new(webrtc_vad::Vad::new_with_rate_and_mode(
        webrtc_vad::SampleRate::Rate16kHz,
        webrtc_vad::VadMode::Quality,
    ));

    let mut have_sound = false;
    let new_stream =
        move || coqui_stt::Stream::from_model(model.clone()).expect("unable to create stream?!");
    let mut stream = new_stream();
    let mut collected_data: Vec<i16> = Vec::new();

    let execute_commands = commands();
    println!("trying to get audio input...");
    get_audio_input_16kHz(move |data: &[i16]| {
        stream.feed_audio(data);
        collected_data.extend(data);
        if collected_data.len() < 2 * VAD_SAMPLES as usize {
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
                let x = std::mem::replace(&mut stream, new_stream())
                    .finish_stream_with_metadata(2)
                    .unwrap()
                    .to_owned();
                println!("Here is what we have:");
                let transcripts = x.transcripts();
                let scores: Vec<f64> = transcripts.iter().map(|c| c.confidence()).collect();
                let phrases: Vec<String> = transcripts
                    .iter()
                    .map(|c| {
                        let mut words = String::new();
                        for w in c.tokens().iter().map(|t| &t.text) {
                            words.push_str(w.as_ref());
                        }
                        words
                    })
                    .collect();
                println!(
                    "{:?} exceeds {:?} by {:?}",
                    phrases[0],
                    phrases[1],
                    scores[0] - scores[1]
                );
                if phrases[0] != "" {
                    match execute_commands.parse(&phrases[0]) {
                        Err(Error::Incomplete) => {
                            println!("    Maybe you didn't finish?");
                        }
                        Err(Error::Wrong) => {
                            println!("    This is bogus!");
                        }
                        Ok((action, "")) => {
                            println!("    Running action {action:?}");
                            action.run();
                        }
                        Ok((action, remainder)) => {
                            println!("    We had extra words: {remainder:?} after {action:?}");
                        }
                    }
                }
                // for c in x.transcripts().iter() {
                //     let action = execute_commands.parse(&words);
                //     println!("{sc:7.2}: {words:?} {action:?}");
                // }
                // let mut words = &words[..];
                // while let Some((a, rest)) = rules.lock().unwrap().parse(&words[..]) {
                //     assert!(rest.len() < words.len());
                //     words = rest;
                //     a.run();
                // }
            }
            have_sound = false;
        }
        collected_data.clear();
    });
}
