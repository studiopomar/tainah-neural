use hound::{SampleFormat, WavReader, WavSpec, WavWriter};
use kamafeu::drivers::{ResamplerArgs, ResamplerDriver, TainahNeuralResamplerDriver};
use std::env;
use std::error::Error;
use std::path::{Path, PathBuf};

fn note_to_hz(note: &str) -> Option<f64> {
    let mut chars = note.trim().chars();
    let letter = chars.next()?.to_ascii_uppercase();
    let mut semitone = match letter {
        'C' => 0,
        'D' => 2,
        'E' => 4,
        'F' => 5,
        'G' => 7,
        'A' => 9,
        'B' => 11,
        _ => return None,
    };
    let mut suffix: String = chars.collect();
    if let Some(rest) = suffix.strip_prefix('#') {
        semitone += 1;
        suffix = rest.to_string();
    } else if let Some(rest) = suffix.strip_prefix('b') {
        semitone -= 1;
        suffix = rest.to_string();
    }
    let octave = suffix.parse::<i32>().ok()?;
    let midi = (octave + 1) * 12 + semitone;
    Some(440.0 * 2.0_f64.powf((f64::from(midi) - 69.0) / 12.0))
}

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
        args.get(index)
            .and_then(|value| value.parse().ok())
            .unwrap_or(default)
    };
    let flags = args.get(4).cloned().unwrap_or_else(|| "g0".into());
    let pitch_freq = note_to_hz(&args[2]).ok_or("Nota inválida; use nomes como C4 ou F#3")?;
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
        pitch_freq,
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

#[cfg(test)]
mod tests {
    use super::note_to_hz;

    #[test]
    fn utau_note_names_convert_to_equal_tempered_frequency() {
        assert!((note_to_hz("A4").unwrap() - 440.0).abs() < 1e-8);
        assert!((note_to_hz("C4").unwrap() - 261.625565).abs() < 1e-5);
        assert!((note_to_hz("F#3").unwrap() - 184.997211).abs() < 1e-5);
        assert!(note_to_hz("H4").is_none());
    }
}
