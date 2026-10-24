//! Cyberpunk / Blade Runner soundtrack generator using ReplicantPeak DSP engine.

use peak_core::dsp::{Engine, Event};
use peak_core::params::{Patch, P};
use peak_core::preset::{parse_syx_patch, TNT_SADTOUCH_SYX};
use peak_core::wavetable::Bank;
use std::fs::File;
use std::io::Write;

fn blade_runner_brass() -> Patch {
    let mut p = Patch::default();
    p.set(P::Osc1Level, 0.85);
    p.set(P::Osc1Shape, 2.0); // Saw
    p.set(P::Osc1ShapeAmt, 0.45); // Oxford Triple-Saw detuned cluster
    p.set(P::Osc1Tune, 0.0);
    p.set(P::Osc1Fine, 0.0);

    p.set(P::Osc2Level, 0.85);
    p.set(P::Osc2Shape, 2.0); // Saw
    p.set(P::Osc2ShapeAmt, 0.40);
    p.set(P::Osc2Tune, 0.0);
    p.set(P::Osc2Fine, 14.0); // +14 cents detune

    p.set(P::Osc3Level, 0.65);
    p.set(P::Osc3Shape, 3.0); // Pulse / Square sub
    p.set(P::Osc3ShapeAmt, 0.25); // PWM
    p.set(P::Osc3Tune, -12.0); // 1 octave below
    p.set(P::Osc3Fine, -8.0);

    // OTA 24dB ladder filter with resonant analog warmth
    p.set(P::FilterType, 1.0); // LP 24
    p.set(P::Cutoff, 480.0);
    p.set(P::Reso, 0.28);
    p.set(P::Drive, 0.24);
    p.set(P::FilterEnvAmt, 0.55);
    p.set(P::Keytrack, 0.65);

    // Expressive CS-80 swelling brass envelope
    p.set(P::AmpAttack, 55.0);
    p.set(P::AmpDecay, 1200.0);
    p.set(P::AmpSustain, 0.85);
    p.set(P::AmpRelease, 750.0);

    p.set(P::FilterAttack, 160.0);
    p.set(P::FilterDecay, 1400.0);
    p.set(P::FilterSustain, 0.35);
    p.set(P::FilterRelease, 850.0);

    // LFO subtle pitch vibrato
    p.set(P::Lfo1Rate, 5.2);
    p.set(P::Lfo1Depth, 0.04);
    p.set(P::Lfo1Dest, 0.0); // Pitch
    p.set(P::Lfo1Fade, 350.0);

    // Massive cinematic Vangelis reverb and tape delay
    p.set(P::Chorus, 0.38);
    p.set(P::DelayMix, 0.35);
    p.set(P::DelayTime, 375.0);
    p.set(P::DelayFeedback, 0.50);
    p.set(P::ReverbMix, 0.48);
    p.set(P::ReverbSize, 0.88);

    p.set(P::Drift, 0.45);
    p.set(P::Diverge, 0.35);
    p.set(P::Glide, 40.0);
    p.set(P::Volume, 2.0);
    p.sanitise();
    p
}

fn cyberpunk_bass() -> Patch {
    let mut p = Patch::default();
    p.set(P::Osc1Level, 0.95);
    p.set(P::Osc1Shape, 2.0);
    p.set(P::Osc1ShapeAmt, 0.30);
    p.set(P::Osc1Tune, -12.0);

    p.set(P::Osc2Level, 0.90);
    p.set(P::Osc2Shape, 2.0);
    p.set(P::Osc2ShapeAmt, 0.30);
    p.set(P::Osc2Tune, -12.0);
    p.set(P::Osc2Fine, 12.0);

    p.set(P::Osc3Level, 0.80);
    p.set(P::Osc3Shape, 3.0); // Sub square
    p.set(P::Osc3Tune, -24.0); // 2 octaves below

    p.set(P::FilterType, 1.0); // LP 24
    p.set(P::Cutoff, 340.0);
    p.set(P::Reso, 0.22);
    p.set(P::Drive, 0.52); // Asymmetric diode drive

    p.set(P::AmpAttack, 10.0);
    p.set(P::AmpDecay, 900.0);
    p.set(P::AmpSustain, 0.90);
    p.set(P::AmpRelease, 350.0);

    p.set(P::Chorus, 0.22);
    p.set(P::Dist, 0.28);
    p.set(P::ReverbMix, 0.12);
    p.set(P::Volume, 1.5);
    p.sanitise();
    p
}

fn cyberpunk_arp() -> Patch {
    let mut p = Patch::default();
    p.set(P::Osc1Level, 0.75);
    p.set(P::Osc1Shape, 2.0);
    p.set(P::Osc1ShapeAmt, 0.20);

    p.set(P::Osc2Level, 0.70);
    p.set(P::Osc2Shape, 3.0);
    p.set(P::Osc2Fine, -8.0);

    p.set(P::FilterType, 1.0);
    p.set(P::Cutoff, 750.0);
    p.set(P::Reso, 0.42);
    p.set(P::FilterEnvAmt, 0.65);
    p.set(P::Keytrack, 0.50);

    p.set(P::FilterAttack, 2.0);
    p.set(P::FilterDecay, 180.0);
    p.set(P::FilterSustain, 0.05);
    p.set(P::FilterRelease, 120.0);

    p.set(P::AmpAttack, 2.0);
    p.set(P::AmpDecay, 200.0);
    p.set(P::AmpSustain, 0.15);
    p.set(P::AmpRelease, 120.0);

    p.set(P::Chorus, 0.30);
    p.set(P::DelayMix, 0.42);
    p.set(P::DelayTime, 250.0);
    p.set(P::DelayFeedback, 0.55);
    p.set(P::ReverbMix, 0.28);
    p.set(P::Volume, 0.0);
    p.sanitise();
    p
}

fn main() {
    let sample_rate = 48_000.0;
    let bpm = 112.0;
    let beat_samples = (60.0 / bpm * sample_rate) as usize;
    let sixteenth_samples = beat_samples / 4;

    // Total duration: 46 seconds (no más de 1 minuto)
    let total_secs = 46.0;
    let total_samples = (total_secs * sample_rate) as usize;

    println!("Initializing ReplicantPeak DSP engines for multi-track composition...");
    let bank = Bank::generate(2049); // Cyberpunk seed 2049

    let mut engine_pad = Engine::new(sample_rate);
    engine_pad.set_bank(bank.clone());
    let (_, pad_patch) = parse_syx_patch(TNT_SADTOUCH_SYX).expect("parse Sadtouch");

    let mut engine_lead = Engine::new(sample_rate);
    engine_lead.set_bank(bank.clone());
    let lead_patch = blade_runner_brass();

    let mut engine_bass = Engine::new(sample_rate);
    engine_bass.set_bank(bank.clone());
    let bass_patch = cyberpunk_bass();

    let mut engine_arp = Engine::new(sample_rate);
    engine_arp.set_bank(bank);
    let arp_patch = cyberpunk_arp();

    let mut left_master = vec![0.0f32; total_samples];
    let mut right_master = vec![0.0f32; total_samples];

    // Track buffers
    let mut pad_l = vec![0.0f32; total_samples];
    let mut pad_r = vec![0.0f32; total_samples];
    let mut lead_l = vec![0.0f32; total_samples];
    let mut lead_r = vec![0.0f32; total_samples];
    let mut bass_l = vec![0.0f32; total_samples];
    let mut bass_r = vec![0.0f32; total_samples];
    let mut arp_l = vec![0.0f32; total_samples];
    let mut arp_r = vec![0.0f32; total_samples];

    // 1. CHORD PROGRESSION (Atmospheric Pad - TnT Sadtouch)
    // C minor 9 -> Ab maj 7 -> Fm 9 -> G sus4 -> C minor
    let chords: &[(f32, f32, &[u8])] = &[
        (0.5, 9.5, &[48, 55, 60, 63, 70]),   // Cm9
        (9.5, 18.5, &[44, 51, 60, 63, 68]),  // Ab maj7
        (18.5, 27.5, &[41, 48, 56, 60, 65]), // Fm9
        (27.5, 36.5, &[43, 50, 55, 62, 67]), // Gsus4 / G7
        (36.5, 45.0, &[36, 48, 55, 60, 63]), // Cm resolution deep
    ];

    let mut pad_events: Vec<(usize, Event)> = Vec::new();
    for &(start_s, end_s, notes) in chords {
        let start_sample = (start_s * sample_rate) as usize;
        let end_sample = (end_s * sample_rate) as usize;
        for &n in notes {
            pad_events.push((
                start_sample,
                Event::NoteOn {
                    note: n,
                    velocity: 0.75,
                },
            ));
            pad_events.push((end_sample, Event::NoteOff { note: n }));
        }
    }
    pad_events.sort_by_key(|e| e.0);

    // 2. BASS DRONE & CYBERPUNK BASSLINE
    let bass_notes: &[(f32, f32, u8)] = &[
        (0.5, 9.5, 36),   // C1
        (9.5, 18.5, 32),  // Ab0
        (18.5, 27.5, 29), // F0
        (27.5, 36.5, 31), // G0
        (36.5, 45.0, 36), // C1
    ];
    let mut bass_events: Vec<(usize, Event)> = Vec::new();
    for &(start_s, end_s, n) in bass_notes {
        let start_sample = (start_s * sample_rate) as usize;
        let end_sample = (end_s * sample_rate) as usize;
        bass_events.push((
            start_sample,
            Event::NoteOn {
                note: n,
                velocity: 0.90,
            },
        ));
        bass_events.push((end_sample, Event::NoteOff { note: n }));
    }
    bass_events.sort_by_key(|e| e.0);

    // 3. BLADE RUNNER BRASS LEAD MELODY (Enters at ~9s with iconic expressive phrasing)
    let lead_notes: &[(f32, f32, u8, f32)] = &[
        // Phrase 1 (The Vangelis Theme)
        (9.8, 12.0, 62, 0.85),  // D4
        (12.0, 13.5, 63, 0.90), // D#4
        (13.5, 15.2, 65, 0.95), // F4
        (15.2, 16.5, 63, 0.88), // D#4
        (16.5, 18.0, 62, 0.85), // D4
        (18.0, 21.0, 60, 0.92), // C4 (held long, swelling)
        (21.2, 22.8, 58, 0.82), // A#3
        (22.8, 26.5, 60, 0.95), // C4
        // Phrase 2 (Climax with soaring octave)
        (27.5, 29.5, 67, 0.92), // G4
        (29.5, 31.0, 70, 0.95), // A#4
        (31.0, 33.5, 72, 1.00), // C5 (high cry)
        (33.5, 35.0, 74, 0.95), // D5
        (35.0, 37.0, 72, 0.92), // C5
        (37.0, 42.0, 63, 0.90), // D#4 (haunting final fall)
        (42.0, 44.5, 60, 0.85), // C4 fade
    ];
    let mut lead_events: Vec<(usize, Event)> = Vec::new();
    for &(start_s, end_s, n, vel) in lead_notes {
        let start_sample = (start_s * sample_rate) as usize;
        let end_sample = (end_s * sample_rate) as usize;
        lead_events.push((
            start_sample,
            Event::NoteOn {
                note: n,
                velocity: vel,
            },
        ));
        lead_events.push((end_sample, Event::NoteOff { note: n }));
    }
    lead_events.sort_by_key(|e| e.0);

    // 4. CYBERPUNK 16TH-NOTE ARPEGGIO (Enters at 18.5s during middle section)
    let arp_pattern = [
        48, 60, 63, 55, 48, 63, 67, 70, 51, 63, 67, 58, 48, 60, 63, 72,
    ];
    let mut arp_events: Vec<(usize, Event)> = Vec::new();
    let arp_start_sample = (18.5 * sample_rate) as usize;
    let arp_end_sample = (37.0 * sample_rate) as usize;

    let mut curr_sample = arp_start_sample;
    let mut pat_idx = 0;
    while curr_sample < arp_end_sample {
        let note = arp_pattern[pat_idx % arp_pattern.len()];
        let dur = sixteenth_samples * 8 / 10;
        arp_events.push((
            curr_sample,
            Event::NoteOn {
                note,
                velocity: 0.78,
            },
        ));
        arp_events.push((curr_sample + dur, Event::NoteOff { note }));
        curr_sample += sixteenth_samples;
        pat_idx += 1;
    }
    arp_events.sort_by_key(|e| e.0);

    // Render all tracks in 128-sample blocks
    let block_size = 128;
    println!("Rendering audio timeline (46 seconds @ 48 kHz)...");

    let mut pad_idx = 0;
    let mut lead_idx = 0;
    let mut bass_idx = 0;
    let mut arp_idx = 0;

    for i in (0..total_samples).step_by(block_size) {
        let end = (i + block_size).min(total_samples);

        // Gather block events for pad
        let mut blk_pad = Vec::new();
        while pad_idx < pad_events.len() && pad_events[pad_idx].0 < end {
            blk_pad.push(pad_events[pad_idx].1);
            pad_idx += 1;
        }
        engine_pad.process(&pad_patch, &blk_pad, &mut pad_l[i..end], &mut pad_r[i..end]);

        // Gather block events for bass
        let mut blk_bass = Vec::new();
        while bass_idx < bass_events.len() && bass_events[bass_idx].0 < end {
            blk_bass.push(bass_events[bass_idx].1);
            bass_idx += 1;
        }
        engine_bass.process(
            &bass_patch,
            &blk_bass,
            &mut bass_l[i..end],
            &mut bass_r[i..end],
        );

        // Gather block events for lead
        let mut blk_lead = Vec::new();
        while lead_idx < lead_events.len() && lead_events[lead_idx].0 < end {
            blk_lead.push(lead_events[lead_idx].1);
            lead_idx += 1;
        }
        engine_lead.process(
            &lead_patch,
            &blk_lead,
            &mut lead_l[i..end],
            &mut lead_r[i..end],
        );

        // Gather block events for arp
        let mut blk_arp = Vec::new();
        while arp_idx < arp_events.len() && arp_events[arp_idx].0 < end {
            blk_arp.push(arp_events[arp_idx].1);
            arp_idx += 1;
        }
        engine_arp.process(&arp_patch, &blk_arp, &mut arp_l[i..end], &mut arp_r[i..end]);
    }

    // Mix tracks together with cinematic mastering
    println!("Mastering & mixing cyberpunk soundscape...");
    for j in 0..total_samples {
        // Fade in first 0.5s, fade out last 2.0s
        let t = j as f32 / sample_rate;
        let fade_in = (t / 0.5).min(1.0);
        let fade_out = ((total_secs - t) / 2.0).clamp(0.0, 1.0);
        let env = fade_in * fade_out;

        let l = (pad_l[j] * 0.70 + bass_l[j] * 0.85 + lead_l[j] * 0.90 + arp_l[j] * 0.55) * env;
        let r = (pad_r[j] * 0.70 + bass_r[j] * 0.85 + lead_r[j] * 0.90 + arp_r[j] * 0.55) * env;

        // Soft analog diode saturation to glue mix together
        left_master[j] = (l / (1.0 + 0.20 * l.abs())).clamp(-0.98, 0.98);
        right_master[j] = (r / (1.0 + 0.20 * r.abs())).clamp(-0.98, 0.98);
    }

    // Write WAV file
    let wav_path = "assets/cyberpunk_blade_runner.wav";
    let mut f = File::create(wav_path).expect("create wav");
    let num_samples = total_samples as u32;
    let byte_rate = 48000u32 * 2 * 2;
    let block_align = 4u16;
    let bits_per_sample = 16u16;
    let data_len = num_samples * 4;

    f.write_all(b"RIFF").unwrap();
    f.write_all(&(36 + data_len).to_le_bytes()).unwrap();
    f.write_all(b"WAVEfmt ").unwrap();
    f.write_all(&16u32.to_le_bytes()).unwrap();
    f.write_all(&1u16.to_le_bytes()).unwrap(); // PCM
    f.write_all(&2u16.to_le_bytes()).unwrap(); // Stereo
    f.write_all(&48000u32.to_le_bytes()).unwrap();
    f.write_all(&byte_rate.to_le_bytes()).unwrap();
    f.write_all(&block_align.to_le_bytes()).unwrap();
    f.write_all(&bits_per_sample.to_le_bytes()).unwrap();
    f.write_all(b"data").unwrap();
    f.write_all(&data_len.to_le_bytes()).unwrap();

    for j in 0..total_samples {
        let sl = (left_master[j] * 32767.0) as i16;
        let sr = (right_master[j] * 32767.0) as i16;
        f.write_all(&sl.to_le_bytes()).unwrap();
        f.write_all(&sr.to_le_bytes()).unwrap();
    }

    println!("Audio rendered successfully to {wav_path} (duration: {total_secs:.1}s)!");
}
