use hound::{SampleFormat, WavReader, WavSpec, WavWriter};
use kamafeu::drivers::{ResamplerArgs, ResamplerDriver, TainahNeuralResamplerDriver};
use std::env;
use std::error::Error;
use std::path::{Path, PathBuf};

fn read_wav(path: &Path) -> Result<(Vec<f32>, u32), Box<dyn Error>> {
    let mut reader = WavReader::open(path)?;
    let spec = reader.spec();
    let samples: Vec<f32> = match spec.sample_format {
        SampleFormat::Float => reader.samples::<f32>().collect::<Result<_, _>>()?,
        SampleFormat::Int => reader
            .samples::<i32>()
            .map(|sample| {
                sample.map(|value| value as f32 / 2.0_f32.powi(spec.bits_per_sample as i32 - 1))
            })
            .collect::<Result<_, _>>()?,
    };
    let mono = if spec.channels > 1 {
        samples
            .chunks(spec.channels as usize)
            .map(|frame| frame.iter().sum::<f32>() / frame.len() as f32)
            .collect()
    } else {
        samples
    };
    Ok((mono, spec.sample_rate))
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.len() < 3 {
        eprintln!("Uso: tainah-neural input.wav output.wav pitch [velocity flags offset duration consonant cutoff volume modulation tempo pitch_bends]");
        std::process::exit(1);
    }
    let input_path = PathBuf::from(&args[0]);
    let output_path = PathBuf::from(&args[1]);
    let parse = |index: usize, default: f64| {
        args.get(index).and_then(|value| value.parse().ok()).unwrap_or(default)
    };
    let flags = args.get(4).cloned().unwrap_or_else(|| "g0".into());
    let tempo = args
        .get(11)
        .map(|value| value.trim_start_matches('!'))
        .and_then(|value| value.parse().ok())
        .unwrap_or(120.0);
    let (samples, sample_rate) = read_wav(&input_path)?;
    let request = ResamplerArgs {
        input_wav: input_path,
        output_wav: output_path.clone(),
        pitch_name: args[2].clone(),
        pitch_freq: 0.0,
        velocity: parse(3, 100.0),
        flags,
        offset_ms: parse(5, 0.0),
        duration_ms: parse(6, 500.0),
        source_consonant_ms: parse(7, 0.0),
        consonant_ms: parse(7, 0.0),
        cutoff_ms: parse(8, 0.0),
        volume: parse(9, 100.0),
        modulation: parse(10, 0.0),
        normalization: None,
        tempo,
        pitch_bend_str: args.get(12).cloned().unwrap_or_default(),
        pitch_points: Vec::new(),
        loop_start_ms: None,
        loop_end_ms: None,
        tail_start_ms: None,
        vowel_formant: None,
    };
    let rendered = TainahNeuralResamplerDriver
        .render_sample(&samples, sample_rate, &request, None)
        .map_err(|error| format!("Renderização Tainah Neural falhou: {error}"))?;
    let mut writer = WavWriter::create(
        &output_path,
        WavSpec {
            channels: 1,
            sample_rate,
            bits_per_sample: 16,
            sample_format: SampleFormat::Int,
        },
    )?;
    for sample in rendered {
        writer.write_sample((sample.clamp(-1.0, 1.0) * i16::MAX as f32) as i16)?;
    }
    writer.finalize()?;
    Ok(())
}
