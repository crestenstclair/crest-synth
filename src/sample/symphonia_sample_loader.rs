// path: src/sample/symphonia_sample_loader.rs

//! Adapter: `SymphoniaSampleLoader`.
//!
//! Implements [`crate::sample::sample_loader::SampleLoader`] on top of the
//! `symphonia` crate for WAV decoding and a small hand-rolled RIFF/SF2 chunk
//! reader for SoundFont banks (symphonia has no SF2 demuxer).
//!
//! This adapter is a non-real-time boundary: it performs blocking file I/O
//! and heap allocation, exactly like the port it implements documents. It
//! must only ever be driven from the UI/loader thread. Nothing here reaches
//! the audio thread directly — callers hand the resulting `SampleSet`s to
//! the audio thread via the `ParameterBridge`/`EventRing` seam, never this
//! adapter.

use std::collections::HashMap;
use std::io::Read;
use std::path::Path;

use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::{DecoderOptions, CODEC_TYPE_NULL};
use symphonia::core::errors::Error as SymphoniaError;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::{MediaSourceStream, MediaSourceStreamOptions};
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

use crate::sample::sample_loader::{LoadError, SampleData, SampleLoader, SampleSet};

/// SF2 generator operator IDs we care about. See the SoundFont 2.04 spec,
/// section 8.1.2, for the full list; every other operator is irrelevant to
/// building a flat sample list and is intentionally ignored.
const GEN_INSTRUMENT: u16 = 41;
const GEN_KEY_RANGE: u16 = 43;
const GEN_VEL_RANGE: u16 = 44;
const GEN_SAMPLE_ID: u16 = 53;
const GEN_OVERRIDING_ROOT_KEY: u16 = 58;

/// Loads sample data from WAV files (via `symphonia`) and SF2 sound banks
/// (via a purpose-built RIFF chunk reader). Holds no state and no
/// dependencies — it is a pure translation from file bytes to the port's
/// value types, so it needs no injected collaborators.
#[derive(Debug, Default, Clone, Copy)]
pub struct SymphoniaSampleLoader;

impl SymphoniaSampleLoader {
    /// Constructs a new loader. Stateless, so this is equivalent to
    /// `SymphoniaSampleLoader::default()`.
    pub fn new() -> Self {
        Self
    }
}

impl SampleLoader for SymphoniaSampleLoader {
    fn load_wav(&self, path: &Path) -> Result<SampleSet, LoadError> {
        let mut header_probe = std::fs::File::open(path)?;
        let mut header = [0u8; 12];
        header_probe.read_exact(&mut header)?;
        ensure_riff_form(&header, b"WAVE", "WAV")?;

        let file = std::fs::File::open(path)?;
        let mss = MediaSourceStream::new(Box::new(file), MediaSourceStreamOptions::default());

        let mut hint = Hint::new();
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            hint.with_extension(ext);
        }

        let probed = symphonia::default::get_probe()
            .format(
                &hint,
                mss,
                &FormatOptions::default(),
                &MetadataOptions::default(),
            )
            .map_err(|e| LoadError::Format(format!("failed to probe WAV file: {e}")))?;
        let mut format = probed.format;

        let track = format
            .tracks()
            .iter()
            .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
            .ok_or_else(|| {
                LoadError::Format("WAV file has no decodable audio track".to_string())
            })?;
        let track_id = track.id;
        let codec_params = track.codec_params.clone();

        let sample_rate_hz = codec_params
            .sample_rate
            .ok_or_else(|| LoadError::Format("WAV track is missing a sample rate".to_string()))?;
        let channel_count = codec_params
            .channels
            .map(|c| c.count() as u16)
            .ok_or_else(|| {
                LoadError::Format("WAV track is missing a channel layout".to_string())
            })?;

        let mut decoder = symphonia::default::get_codecs()
            .make(&codec_params, &DecoderOptions::default())
            .map_err(|e| LoadError::Format(format!("failed to create WAV decoder: {e}")))?;

        let mut frames: Vec<f32> = Vec::new();
        loop {
            let packet = match format.next_packet() {
                Ok(packet) => packet,
                Err(SymphoniaError::IoError(ref e))
                    if e.kind() == std::io::ErrorKind::UnexpectedEof =>
                {
                    break;
                }
                Err(SymphoniaError::ResetRequired) => break,
                Err(e) => return Err(LoadError::Format(format!("failed reading WAV packet: {e}"))),
            };

            if packet.track_id() != track_id {
                continue;
            }

            match decoder.decode(&packet) {
                Ok(decoded) => {
                    let spec = *decoded.spec();
                    let mut sample_buf = SampleBuffer::<f32>::new(decoded.capacity() as u64, spec);
                    sample_buf.copy_interleaved_ref(decoded);
                    frames.extend_from_slice(sample_buf.samples());
                }
                Err(SymphoniaError::DecodeError(_)) => continue,
                Err(e) => {
                    return Err(LoadError::Format(format!(
                        "failed decoding WAV packet: {e}"
                    )))
                }
            }
        }

        let name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("sample")
            .to_string();

        let sample = SampleData {
            sample_rate_hz,
            channel_count,
            frames,
            root_key: 60,
            key_range: (0, 127),
            velocity_range: (0, 127),
        };

        Ok(SampleSet::new(name, vec![sample]))
    }

    fn load_sf2(&self, path: &Path) -> Result<Vec<SampleSet>, LoadError> {
        let bytes = std::fs::read(path)?;
        ensure_riff_form(&bytes, b"sfbk", "SF2")?;

        let body = bytes.get(12..).ok_or_else(|| {
            LoadError::Format("SF2 file is too small to contain any chunks".to_string())
        })?;
        let chunks = collect_chunks(body)?;
        build_sample_sets(&chunks)
    }
}

/// Reads the 12-byte RIFF header (`RIFF` + size + form type) from `header`
/// and confirms the form type matches `expected` (e.g. `b"WAVE"` or
/// `b"sfbk"`). Returns [`LoadError::UnsupportedFormat`] on a form mismatch —
/// the file is valid RIFF but not the format this loader method expects —
/// and [`LoadError::Format`] if it isn't RIFF at all.
fn ensure_riff_form(header: &[u8], expected: &[u8; 4], label: &str) -> Result<(), LoadError> {
    if header.len() < 12 {
        return Err(LoadError::Format(
            "file is too small to be a RIFF container".to_string(),
        ));
    }
    if &header[0..4] != b"RIFF" {
        return Err(LoadError::Format("missing RIFF header".to_string()));
    }
    let form = [header[8], header[9], header[10], header[11]];
    if &form != expected {
        return Err(LoadError::UnsupportedFormat(format!(
            "expected {label} RIFF form, found '{}'",
            String::from_utf8_lossy(&form)
        )));
    }
    Ok(())
}

/// Flattens a RIFF chunk stream into a map keyed by chunk ID, recursing
/// through `LIST` chunks (SF2 nests its `pdta`/`sdta` chunks inside `LIST`
/// wrappers). Chunk IDs are unique across an SF2 file, so a flat map is
/// sufficient — callers never need to know which `LIST` a chunk came from.
fn collect_chunks(mut data: &[u8]) -> Result<HashMap<[u8; 4], Vec<u8>>, LoadError> {
    let mut chunks = HashMap::new();
    while data.len() >= 8 {
        let id = [data[0], data[1], data[2], data[3]];
        let size = u32::from_le_bytes([data[4], data[5], data[6], data[7]]) as usize;
        let body_start = 8;
        if data.len() < body_start + size {
            return Err(LoadError::Format(format!(
                "truncated '{}' chunk",
                String::from_utf8_lossy(&id)
            )));
        }
        let chunk_data = &data[body_start..body_start + size];
        let padded_size = size + (size % 2);

        if &id == b"LIST" {
            if chunk_data.len() < 4 {
                return Err(LoadError::Format("malformed LIST chunk".to_string()));
            }
            let nested = collect_chunks(&chunk_data[4..])?;
            chunks.extend(nested);
        } else {
            chunks.insert(id, chunk_data.to_vec());
        }

        data = &data[body_start + padded_size..];
    }
    Ok(chunks)
}

fn req_chunk<'a>(
    chunks: &'a HashMap<[u8; 4], Vec<u8>>,
    id: &[u8; 4],
    label: &str,
) -> Result<&'a [u8], LoadError> {
    chunks
        .get(id)
        .map(|v| v.as_slice())
        .ok_or_else(|| LoadError::Format(format!("missing required SF2 chunk: {label}")))
}

/// A parsed `phdr` (preset header) record. Only the fields the flattening
/// algorithm consumes are kept — the rest of the SF2 record (library,
/// genre, morphology) has no bearing on building a sample list.
struct PresetHeader {
    name: String,
    bag_ndx: u16,
}

/// A parsed `pbag`/`ibag` (zone index) record.
struct Bag {
    gen_ndx: u16,
}

/// A parsed `pgen`/`igen` generator record.
struct Generator {
    oper: u16,
    amount: u16,
}

/// A parsed `inst` (instrument header) record.
struct InstHeader {
    bag_ndx: u16,
}

/// A parsed `shdr` (sample header) record.
struct SampleHeader {
    name: String,
    start: u32,
    end: u32,
    sample_rate: u32,
    orig_key: u8,
}

fn decode_name(bytes: &[u8]) -> String {
    let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
    String::from_utf8_lossy(&bytes[..end]).trim().to_string()
}

fn parse_phdr(data: &[u8]) -> Result<Vec<PresetHeader>, LoadError> {
    if !data.len().is_multiple_of(38) {
        return Err(LoadError::Format("malformed phdr chunk".to_string()));
    }
    Ok(data
        .chunks_exact(38)
        .map(|r| PresetHeader {
            name: decode_name(&r[0..20]),
            bag_ndx: u16::from_le_bytes([r[24], r[25]]),
        })
        .collect())
}

fn parse_bag(data: &[u8]) -> Result<Vec<Bag>, LoadError> {
    if !data.len().is_multiple_of(4) {
        return Err(LoadError::Format("malformed bag chunk".to_string()));
    }
    Ok(data
        .chunks_exact(4)
        .map(|r| Bag {
            gen_ndx: u16::from_le_bytes([r[0], r[1]]),
        })
        .collect())
}

fn parse_gen(data: &[u8]) -> Result<Vec<Generator>, LoadError> {
    if !data.len().is_multiple_of(4) {
        return Err(LoadError::Format("malformed generator chunk".to_string()));
    }
    Ok(data
        .chunks_exact(4)
        .map(|r| Generator {
            oper: u16::from_le_bytes([r[0], r[1]]),
            amount: u16::from_le_bytes([r[2], r[3]]),
        })
        .collect())
}

fn parse_inst(data: &[u8]) -> Result<Vec<InstHeader>, LoadError> {
    if !data.len().is_multiple_of(22) {
        return Err(LoadError::Format("malformed inst chunk".to_string()));
    }
    Ok(data
        .chunks_exact(22)
        .map(|r| InstHeader {
            bag_ndx: u16::from_le_bytes([r[20], r[21]]),
        })
        .collect())
}

fn parse_shdr(data: &[u8]) -> Result<Vec<SampleHeader>, LoadError> {
    if !data.len().is_multiple_of(46) {
        return Err(LoadError::Format("malformed shdr chunk".to_string()));
    }
    Ok(data
        .chunks_exact(46)
        .map(|r| SampleHeader {
            name: decode_name(&r[0..20]),
            start: u32::from_le_bytes([r[20], r[21], r[22], r[23]]),
            end: u32::from_le_bytes([r[24], r[25], r[26], r[27]]),
            sample_rate: u32::from_le_bytes([r[36], r[37], r[38], r[39]]),
            orig_key: r[40],
        })
        .collect())
}

fn find_gen(gens: &[Generator], oper: u16) -> Option<u16> {
    gens.iter().find(|g| g.oper == oper).map(|g| g.amount)
}

/// SF2 packs an inclusive `(lo, hi)` MIDI range into a generator amount as
/// two bytes: low byte first, high byte second.
fn range_from_amount(amount: u16) -> (u8, u8) {
    let bytes = amount.to_le_bytes();
    (bytes[0], bytes[1])
}

/// Walks the preset -> instrument -> sample zone hierarchy described by the
/// SF2 spec and flattens it into one `SampleSet` per preset. Global zones
/// (a zone with no `instrument`/`sampleID` generator) are skipped rather
/// than applied as defaults — a deliberate simplification documented here
/// rather than silently mishandled.
fn build_sample_sets(chunks: &HashMap<[u8; 4], Vec<u8>>) -> Result<Vec<SampleSet>, LoadError> {
    let phdr = parse_phdr(req_chunk(chunks, b"phdr", "phdr")?)?;
    let pbag = parse_bag(req_chunk(chunks, b"pbag", "pbag")?)?;
    let pgen = parse_gen(req_chunk(chunks, b"pgen", "pgen")?)?;
    let inst = parse_inst(req_chunk(chunks, b"inst", "inst")?)?;
    let ibag = parse_bag(req_chunk(chunks, b"ibag", "ibag")?)?;
    let igen = parse_gen(req_chunk(chunks, b"igen", "igen")?)?;
    let shdr = parse_shdr(req_chunk(chunks, b"shdr", "shdr")?)?;
    let smpl: &[u8] = chunks.get(b"smpl").map(|v| v.as_slice()).unwrap_or(&[]);

    if phdr.len() < 2 {
        return Err(LoadError::Format("phdr chunk has no presets".to_string()));
    }

    let mut sets = Vec::with_capacity(phdr.len() - 1);
    for i in 0..phdr.len() - 1 {
        let preset = &phdr[i];
        let bag_start = preset.bag_ndx as usize;
        let bag_end = phdr[i + 1].bag_ndx as usize;
        let mut samples = Vec::new();

        for j in bag_start..bag_end {
            if j + 1 >= pbag.len() {
                break;
            }
            let gen_start = pbag[j].gen_ndx as usize;
            let gen_end = pbag[j + 1].gen_ndx as usize;
            if gen_start > gen_end || gen_end > pgen.len() {
                continue;
            }
            let zone_gens = &pgen[gen_start..gen_end];
            let Some(instrument_amount) = find_gen(zone_gens, GEN_INSTRUMENT) else {
                continue;
            };
            let preset_key_range = find_gen(zone_gens, GEN_KEY_RANGE)
                .map(range_from_amount)
                .unwrap_or((0, 127));
            let preset_vel_range = find_gen(zone_gens, GEN_VEL_RANGE)
                .map(range_from_amount)
                .unwrap_or((0, 127));

            let instrument_index = instrument_amount as usize;
            if instrument_index + 1 >= inst.len() {
                continue;
            }
            let instrument = &inst[instrument_index];
            let ibag_start = instrument.bag_ndx as usize;
            let ibag_end = inst[instrument_index + 1].bag_ndx as usize;

            for k in ibag_start..ibag_end {
                if k + 1 >= ibag.len() {
                    break;
                }
                let igen_start = ibag[k].gen_ndx as usize;
                let igen_end = ibag[k + 1].gen_ndx as usize;
                if igen_start > igen_end || igen_end > igen.len() {
                    continue;
                }
                let zone_igens = &igen[igen_start..igen_end];
                let Some(sample_amount) = find_gen(zone_igens, GEN_SAMPLE_ID) else {
                    continue;
                };
                let sample_index = sample_amount as usize;
                if sample_index + 1 >= shdr.len() {
                    continue;
                }
                let sample_header = &shdr[sample_index];

                let key_range = find_gen(zone_igens, GEN_KEY_RANGE)
                    .map(range_from_amount)
                    .unwrap_or(preset_key_range);
                let velocity_range = find_gen(zone_igens, GEN_VEL_RANGE)
                    .map(range_from_amount)
                    .unwrap_or(preset_vel_range);
                let root_key = find_gen(zone_igens, GEN_OVERRIDING_ROOT_KEY)
                    .map(|amount| amount as u8)
                    .unwrap_or(sample_header.orig_key);

                let start_byte = (sample_header.start as usize) * 2;
                let end_byte = (sample_header.end as usize) * 2;
                if start_byte > end_byte || end_byte > smpl.len() {
                    return Err(LoadError::Format(format!(
                        "sample '{}' references out-of-range PCM data",
                        sample_header.name
                    )));
                }
                let frames: Vec<f32> = smpl[start_byte..end_byte]
                    .chunks_exact(2)
                    .map(|b| i16::from_le_bytes([b[0], b[1]]) as f32 / i16::MAX as f32)
                    .collect();

                samples.push(SampleData {
                    sample_rate_hz: sample_header.sample_rate,
                    channel_count: 1,
                    frames,
                    root_key,
                    key_range,
                    velocity_range,
                });
            }
        }

        sets.push(SampleSet::new(preset.name.clone(), samples));
    }

    Ok(sets)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static TEST_FILE_COUNTER: AtomicUsize = AtomicUsize::new(0);

    fn temp_file_path(extension: &str) -> std::path::PathBuf {
        let id = TEST_FILE_COUNTER.fetch_add(1, Ordering::SeqCst);
        std::env::temp_dir().join(format!(
            "crest_synth_symphonia_test_{}_{}.{}",
            std::process::id(),
            id,
            extension
        ))
    }

    fn build_wav_bytes(sample_rate: u32, samples: &[i16]) -> Vec<u8> {
        let byte_rate = sample_rate * 2;
        let block_align: u16 = 2;
        let bits_per_sample: u16 = 16;
        let data_bytes: Vec<u8> = samples.iter().flat_map(|s| s.to_le_bytes()).collect();

        let mut fmt_chunk = Vec::new();
        fmt_chunk.extend_from_slice(&1u16.to_le_bytes()); // PCM
        fmt_chunk.extend_from_slice(&1u16.to_le_bytes()); // mono
        fmt_chunk.extend_from_slice(&sample_rate.to_le_bytes());
        fmt_chunk.extend_from_slice(&byte_rate.to_le_bytes());
        fmt_chunk.extend_from_slice(&block_align.to_le_bytes());
        fmt_chunk.extend_from_slice(&bits_per_sample.to_le_bytes());

        let mut body = Vec::new();
        body.extend_from_slice(b"WAVE");
        body.extend_from_slice(b"fmt ");
        body.extend_from_slice(&(fmt_chunk.len() as u32).to_le_bytes());
        body.extend_from_slice(&fmt_chunk);
        body.extend_from_slice(b"data");
        body.extend_from_slice(&(data_bytes.len() as u32).to_le_bytes());
        body.extend_from_slice(&data_bytes);

        let mut riff = Vec::new();
        riff.extend_from_slice(b"RIFF");
        riff.extend_from_slice(&(body.len() as u32).to_le_bytes());
        riff.extend_from_slice(&body);
        riff
    }

    fn name20(s: &str) -> [u8; 20] {
        let mut buf = [0u8; 20];
        let bytes = s.as_bytes();
        let len = bytes.len().min(20);
        buf[..len].copy_from_slice(&bytes[..len]);
        buf
    }

    fn chunk(id: &[u8; 4], data: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(id);
        out.extend_from_slice(&(data.len() as u32).to_le_bytes());
        out.extend_from_slice(data);
        if data.len() % 2 == 1 {
            out.push(0);
        }
        out
    }

    fn list_chunk(list_type: &[u8; 4], mut inner: Vec<u8>) -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(list_type);
        data.append(&mut inner);
        chunk(b"LIST", &data)
    }

    fn build_minimal_sf2_bytes() -> Vec<u8> {
        let mut phdr = Vec::new();
        phdr.extend_from_slice(&name20("Test"));
        phdr.extend_from_slice(&0u16.to_le_bytes());
        phdr.extend_from_slice(&0u16.to_le_bytes());
        phdr.extend_from_slice(&0u16.to_le_bytes());
        phdr.extend_from_slice(&0u32.to_le_bytes());
        phdr.extend_from_slice(&0u32.to_le_bytes());
        phdr.extend_from_slice(&0u32.to_le_bytes());
        phdr.extend_from_slice(&name20("EOP"));
        phdr.extend_from_slice(&0u16.to_le_bytes());
        phdr.extend_from_slice(&0u16.to_le_bytes());
        phdr.extend_from_slice(&1u16.to_le_bytes());
        phdr.extend_from_slice(&0u32.to_le_bytes());
        phdr.extend_from_slice(&0u32.to_le_bytes());
        phdr.extend_from_slice(&0u32.to_le_bytes());

        let mut pbag = Vec::new();
        pbag.extend_from_slice(&0u16.to_le_bytes());
        pbag.extend_from_slice(&0u16.to_le_bytes());
        pbag.extend_from_slice(&3u16.to_le_bytes());
        pbag.extend_from_slice(&0u16.to_le_bytes());

        let mut pgen = Vec::new();
        pgen.extend_from_slice(&41u16.to_le_bytes());
        pgen.extend_from_slice(&0u16.to_le_bytes());
        pgen.extend_from_slice(&43u16.to_le_bytes());
        pgen.extend_from_slice(&[0u8, 127u8]);
        pgen.extend_from_slice(&44u16.to_le_bytes());
        pgen.extend_from_slice(&[0u8, 127u8]);

        let mut inst = Vec::new();
        inst.extend_from_slice(&name20("TestInst"));
        inst.extend_from_slice(&0u16.to_le_bytes());
        inst.extend_from_slice(&name20("EOI"));
        inst.extend_from_slice(&1u16.to_le_bytes());

        let mut ibag = Vec::new();
        ibag.extend_from_slice(&0u16.to_le_bytes());
        ibag.extend_from_slice(&0u16.to_le_bytes());
        ibag.extend_from_slice(&1u16.to_le_bytes());
        ibag.extend_from_slice(&0u16.to_le_bytes());

        let mut igen = Vec::new();
        igen.extend_from_slice(&53u16.to_le_bytes());
        igen.extend_from_slice(&0u16.to_le_bytes());

        let mut shdr = Vec::new();
        shdr.extend_from_slice(&name20("TestSample"));
        shdr.extend_from_slice(&0u32.to_le_bytes());
        shdr.extend_from_slice(&4u32.to_le_bytes());
        shdr.extend_from_slice(&0u32.to_le_bytes());
        shdr.extend_from_slice(&4u32.to_le_bytes());
        shdr.extend_from_slice(&44_100u32.to_le_bytes());
        shdr.push(60);
        shdr.push(0);
        shdr.extend_from_slice(&0u16.to_le_bytes());
        shdr.extend_from_slice(&1u16.to_le_bytes());
        shdr.extend_from_slice(&name20("EOS"));
        shdr.extend_from_slice(&0u32.to_le_bytes());
        shdr.extend_from_slice(&0u32.to_le_bytes());
        shdr.extend_from_slice(&0u32.to_le_bytes());
        shdr.extend_from_slice(&0u32.to_le_bytes());
        shdr.extend_from_slice(&0u32.to_le_bytes());
        shdr.push(0);
        shdr.push(0);
        shdr.extend_from_slice(&0u16.to_le_bytes());
        shdr.extend_from_slice(&0u16.to_le_bytes());

        let mut smpl = Vec::new();
        for s in [0i16, 16384, -16384, 32767] {
            smpl.extend_from_slice(&s.to_le_bytes());
        }

        let pdta = list_chunk(
            b"pdta",
            [
                chunk(b"phdr", &phdr),
                chunk(b"pbag", &pbag),
                chunk(b"pgen", &pgen),
                chunk(b"inst", &inst),
                chunk(b"ibag", &ibag),
                chunk(b"igen", &igen),
                chunk(b"shdr", &shdr),
            ]
            .concat(),
        );
        let sdta = list_chunk(b"sdta", chunk(b"smpl", &smpl));

        let mut body = Vec::new();
        body.extend_from_slice(b"sfbk");
        body.extend_from_slice(&sdta);
        body.extend_from_slice(&pdta);

        let mut riff = Vec::new();
        riff.extend_from_slice(b"RIFF");
        riff.extend_from_slice(&(body.len() as u32).to_le_bytes());
        riff.extend_from_slice(&body);
        riff
    }

    #[test]
    fn load_wav_decodes_pcm_samples() {
        let path = temp_file_path("wav");
        std::fs::write(
            &path,
            build_wav_bytes(44_100, &[0, 16_384, -16_384, 32_767]),
        )
        .unwrap();

        let loader = SymphoniaSampleLoader::new();
        let result = loader.load_wav(&path);
        std::fs::remove_file(&path).ok();

        let set = result.expect("expected the WAV file to load");
        assert_eq!(set.samples.len(), 1);
        let sample = &set.samples[0];
        assert_eq!(sample.sample_rate_hz, 44_100);
        assert_eq!(sample.channel_count, 1);
        assert_eq!(sample.frames.len(), 4);
        assert_eq!(sample.key_range, (0, 127));
        assert_eq!(sample.velocity_range, (0, 127));
    }

    #[test]
    fn load_wav_propagates_io_errors_for_missing_files() {
        let loader = SymphoniaSampleLoader::new();
        let result = loader.load_wav(Path::new("/nonexistent/crest-synth-test/missing.wav"));
        assert!(matches!(result, Err(LoadError::Io(_))));
    }

    #[test]
    fn load_wav_reports_format_error_for_non_riff_file() {
        let path = temp_file_path("wav");
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"JUNK");
        bytes.extend_from_slice(&0u32.to_le_bytes());
        bytes.extend_from_slice(b"WAVE");
        std::fs::write(&path, bytes).unwrap();

        let loader = SymphoniaSampleLoader::new();
        let result = loader.load_wav(&path);
        std::fs::remove_file(&path).ok();

        assert!(matches!(result, Err(LoadError::Format(_))));
    }

    #[test]
    fn load_wav_rejects_an_sf2_file() {
        let path = temp_file_path("wav");
        std::fs::write(&path, build_minimal_sf2_bytes()).unwrap();

        let loader = SymphoniaSampleLoader::new();
        let result = loader.load_wav(&path);
        std::fs::remove_file(&path).ok();

        assert!(matches!(result, Err(LoadError::UnsupportedFormat(_))));
    }

    #[test]
    fn load_sf2_rejects_a_wav_file() {
        let path = temp_file_path("sf2");
        std::fs::write(&path, build_wav_bytes(44_100, &[0, 1, 2, 3])).unwrap();

        let loader = SymphoniaSampleLoader::new();
        let result = loader.load_sf2(&path);
        std::fs::remove_file(&path).ok();

        assert!(matches!(result, Err(LoadError::UnsupportedFormat(_))));
    }

    #[test]
    fn load_sf2_flattens_one_sample_set_per_preset() {
        let path = temp_file_path("sf2");
        std::fs::write(&path, build_minimal_sf2_bytes()).unwrap();

        let loader = SymphoniaSampleLoader::new();
        let result = loader.load_sf2(&path);
        std::fs::remove_file(&path).ok();

        let sets = result.expect("expected the SF2 bank to load");
        assert_eq!(sets.len(), 1);
        assert_eq!(sets[0].name, "Test");
        assert_eq!(sets[0].samples.len(), 1);

        let sample = &sets[0].samples[0];
        assert_eq!(sample.sample_rate_hz, 44_100);
        assert_eq!(sample.channel_count, 1);
        assert_eq!(sample.root_key, 60);
        assert_eq!(sample.key_range, (0, 127));
        assert_eq!(sample.velocity_range, (0, 127));
        assert_eq!(sample.frames.len(), 4);
        assert!((sample.frames[0] - 0.0).abs() < 1e-6);
        assert!((sample.frames[3] - 1.0).abs() < 1e-6);
    }
}
