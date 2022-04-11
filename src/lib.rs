use std::sync::{Arc, Mutex};

pub fn transcription() {
    let model = coqui_stt::Model::new("english/model.tflite").expect("unable to create model");
    let desired_rate = cpal::SampleRate(model.get_sample_rate() as u32);
    let streaming = Arc::new(Mutex::new(
        model.into_streaming().expect("trouble streaming the model"),
    ));

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
    let config = supported_config.into();
    let streaming_copy = streaming.clone();
    let stream = device
        .build_input_stream(
            &config,
            move |data: &[i16], _: &cpal::InputCallbackInfo| {
                // react to stream events and read or write stream data here.
                streaming_copy.lock().unwrap().feed_audio(data);
            },
            move |err| {
                // react to errors here.
                panic!("stream error: {}", err)
            },
        )
        .expect("error creating stream");
    stream.play().unwrap();

    loop {
        if let Ok(s) = streaming.lock().unwrap().intermediate_decode() {
            if s.len() > 0 {
                println!("Said: {s}");
            }
        }
        std::thread::sleep(std::time::Duration::from_secs_f64(1.0));
    }
}
