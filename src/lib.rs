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

pub fn voice_control(commands: impl 'static + Fn() -> Parser<Action>) {
    let recognize_commands = load_voice_control(commands);

    let vad = std::sync::Mutex::new(webrtc_vad::Vad::new_with_rate_and_mode(
        webrtc_vad::SampleRate::Rate16kHz,
        webrtc_vad::VadMode::VeryAggressive,
    ));

    let mut have_sound = false;
    let mut silence_check: Vec<i16> = Vec::new();
    let mut all_data: Vec<i16> = Vec::new();

    let mut audio_sample = 0;

    println!("trying to get audio input...");
    get_audio_input_16kHz(move |data: &[i16]| {
        silence_check.extend(data);
        if silence_check.len() < 8 * VAD_SAMPLES as usize {
            return;
        }
        let mut vad = vad.lock().unwrap();
        if silence_check
            .chunks_exact(VAD_SAMPLES as usize)
            .any(|data| vad.is_voice_segment(data).expect("wrong size data sample"))
        {
            all_data.extend(&silence_check);
            println!("Found audio {} samples", all_data.len());
            have_sound = true;
        } else {
            if have_sound {
                // let fname = format!("audio/final-silence-{audio_sample:06}.wav");
                // println!("Final silence {} samples as {fname}", silence_check.len());
                // println!("final silence is {silence_check:?}");
                // save_data(fname.as_str(), &silence_check);
                // audio_sample += 1;
                let fname = if let Some(action) = recognize_commands(&all_data) {
                    action.run();
                    format!("audio/{audio_sample:06}-run-{action:?}.wav")
                } else {
                    format!("audio/{audio_sample:06}-unrecognized.wav")
                };
                println!("Saving {} samples as {fname}", all_data.len());
                save_data(fname.as_str(), &all_data);
                audio_sample += 1;
            } else {
                // let fname = format!("audio/silence-{audio_sample:06}.wav");
                // println!("Discarding {} samples as {fname}", silence_check.len());
                // save_data(fname.as_str(), &silence_check);
                // audio_sample += 1;
            }
            have_sound = false;
            all_data.clear();
        }
        silence_check.clear();
    });
}

pub fn load_voice_control(
    commands: impl Fn() -> Parser<Action>,
) -> impl 'static + Fn(&[i16]) -> Option<Action> {
    let mut model = coqui_stt::Model::new("english/model.tflite").expect("unable to create model");
    model
        .enable_external_scorer("english/huge-vocabulary.scorer")
        .expect("unable to read scorer");
    assert_eq!(model.get_sample_rate(), REQUIRED_RATE.0 as i32);
    let model_commands = commands();
    model
        .enable_callback_scorer(move |s| {
            if let Err(Error::Wrong) = model_commands.parse(s) {
                // println!("      bad input {:?}", s);
                -10.0
            } else {
                // println!("      good input {:?}", s);
                0.0
            }
        })
        .expect("unable to apply callback scorer");
    let model = Arc::new(model);

    let new_stream =
        move || coqui_stt::Stream::from_model(model.clone()).expect("unable to create stream?!");

    let execute_commands = commands();

    move |data: &[i16]| -> Option<Action> {
        let mut stream = new_stream();
        stream.feed_audio(data);
        let x = stream.finish_stream_with_metadata(2).unwrap().to_owned();
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
        if phrases.len() == 1 {
            if phrases[0] == "" {
                println!("You didn't say anything")
            }
        } else {
            println!(
                "{:?} exceeds {:?} by {:?}",
                phrases[0],
                phrases[1],
                scores[0] - scores[1]
            );
        }
        if phrases[0] != "" {
            match execute_commands.parse(&phrases[0]) {
                Err(Error::Incomplete) => {
                    println!("    Maybe you didn't finish?");
                    None
                }
                Err(Error::Wrong) => {
                    println!("    This is bogus!");
                    None
                }
                Ok((action, "")) => {
                    println!("    Running action {action:?}");
                    Some(action)
                }
                Ok((action, remainder)) => {
                    println!("    We had extra words: {remainder:?} after {action:?}");
                    None
                }
            }
        } else {
            None
        }
    }
}

fn save_data(fname: &str, data: &[i16]) {
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: REQUIRED_RATE.0,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = hound::WavWriter::create(fname, spec).unwrap();
    let mut writer = writer.get_i16_writer(data.len() as u32);
    for s in data.iter().copied() {
        writer.write_sample(s);
    }
    writer.flush().ok();
}

#[cfg(test)]
fn load_data(fname: &str) -> Vec<i16> {
    let reader = hound::WavReader::open(fname).unwrap();
    let len = reader.len();
    println!("len is {len}");
    reader.into_samples().map(|s| s.unwrap()).collect()
}

#[test]
fn save_load() {
    let data = (1..1000).collect::<Vec<_>>();
    let tf = tempfile::NamedTempFile::new().unwrap().into_temp_path();
    save_data(tf.to_str().unwrap(), &data);
    let new_data = load_data(tf.to_str().unwrap());
    assert_eq!(data, new_data);
}

#[test]
fn recognize_testing() {
    use parser::IntoParser;

    let parser = || {
        "testing".map(|_| Action::new("Testing!".to_string(), || println!("I am running a test!")))
    };
    let recognizer = load_voice_control(parser);
    let sound = load_data("test-audio/testing.wav");
    let result = recognizer(&sound);
    println!("Result is {result:?}");
    assert!(result.is_some());
    assert_eq!(format!("{result:?}"), r#"Some("Testing!")"#.to_string());

    let parser = || {
        parser::choose(
            "command",
            vec![
                parser::number::number().map(move |n| {
                    Action::new("{n} blind mice".to_string(), move || println!("I see {n}"))
                }),
                "testing".map(|_| {
                    Action::new("Testing!".to_string(), || println!("I am running a test!"))
                }),
            ],
        )
    };
    let recognizer = load_voice_control(parser);
    let sound = load_data("test-audio/testing.wav");
    let result = recognizer(&sound);
    println!("Result is {result:?}");
    assert!(result.is_some());
    assert_eq!(format!("{result:?}"), r#"Some("Testing!")"#.to_string());
}
