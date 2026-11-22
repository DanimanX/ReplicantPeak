//! Start-up arguments for the standalone application.
//!
//! NIH-plug's standalone wrapper defaults to `--sample-rate 48000`. On Windows,
//! WASAPI in shared mode only accepts the rate the output device is currently
//! set to (Sound settings, or the audio interface's own control panel), so any
//! device not running at exactly 48 kHz failed with "The audio output device
//! does not support 2 audio channels at a sample rate of 48000 Hz ..." and the
//! standalone fell back to the silent dummy backend.
//!
//! When the user did not pass a sample rate, ask the output device for the rate
//! it is running at and pass that on. An explicit `-r`/`--sample-rate` always
//! wins.

/// The process arguments, with `--sample-rate <device rate>` appended when the
/// user gave no sample rate and the output device's rate can be determined.
pub fn args() -> Vec<String> {
    let mut args: Vec<String> = std::env::args().collect();
    if let Some(rate) = device_sample_rate(&args) {
        eprintln!(
            "Using the output device's current sample rate of {rate} Hz \
             (override with --sample-rate)."
        );
        args.push("--sample-rate".to_string());
        args.push(rate.to_string());
    }
    args
}

#[cfg(target_os = "windows")]
fn device_sample_rate(args: &[String]) -> Option<u32> {
    use cpal::traits::{DeviceTrait, HostTrait};

    let options = args.get(1..).unwrap_or_default();
    if option_value(options, "-r", "--sample-rate").is_some()
        || options
            .iter()
            .any(|a| matches!(a.as_str(), "-h" | "--help" | "-V" | "--version"))
    {
        return None;
    }
    // Only WASAPI (`auto` falls through to it when JACK is not running) opens a
    // cpal output device; JACK uses the server's rate and dummy has no device.
    if let Some(backend) = option_value(options, "-b", "--backend") {
        if backend != "auto" && backend != "wasapi" {
            return None;
        }
    }

    let host = cpal::host_from_id(cpal::HostId::Wasapi).ok()?;
    let device = match option_value(options, "", "--output-device") {
        Some(name) => host
            .output_devices()
            .ok()?
            .find(|d| d.name().map(|n| n == name).unwrap_or(false))?,
        None => host.default_output_device()?,
    };
    Some(device.default_output_config().ok()?.sample_rate().0)
}

#[cfg(not(target_os = "windows"))]
fn device_sample_rate(_args: &[String]) -> Option<u32> {
    None
}

/// The value of `--long <value>`, `--long=<value>`, `-s <value>` or `-s<value>`.
/// Pass an empty `short` for options without a short form.
#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
fn option_value(options: &[String], short: &str, long: &str) -> Option<String> {
    let mut iter = options.iter();
    while let Some(arg) = iter.next() {
        if arg == long || (!short.is_empty() && arg == short) {
            return iter.next().cloned();
        }
        if let Some(value) = arg.strip_prefix(long).and_then(|r| r.strip_prefix('=')) {
            return Some(value.to_string());
        }
        if !short.is_empty() && !arg.starts_with("--") {
            if let Some(value) = arg.strip_prefix(short).filter(|v| !v.is_empty()) {
                return Some(value.to_string());
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::option_value;

    fn args(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn finds_every_option_spelling() {
        for list in [
            &["--sample-rate", "44100"][..],
            &["--sample-rate=44100"],
            &["-r", "44100"],
            &["-r44100"],
            &["-b", "wasapi", "-r", "44100"],
        ] {
            assert_eq!(
                option_value(&args(list), "-r", "--sample-rate").as_deref(),
                Some("44100"),
                "{list:?}"
            );
        }
    }

    #[test]
    fn ignores_other_options() {
        let list = args(&[
            "-b",
            "dummy",
            "--period-size",
            "1024",
            "--output-device",
            "X",
        ]);
        assert_eq!(option_value(&list, "-r", "--sample-rate"), None);
        assert_eq!(
            option_value(&list, "", "--output-device").as_deref(),
            Some("X")
        );
        assert_eq!(
            option_value(&list, "-b", "--backend").as_deref(),
            Some("dummy")
        );
    }
}
