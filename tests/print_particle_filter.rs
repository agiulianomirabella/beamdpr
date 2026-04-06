extern crate egsphsp;

use std::fs::File;
use std::fs::remove_file;
use std::io::Write;
use std::path::Path;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use egsphsp::PHSPReader;

const RECORD_BYTES: usize = 28;
const POSITRON_BIT: u32 = 1u32 << 29;
const ELECTRON_BIT: u32 = 1u32 << 30;
const CHARGED_MASK: u32 = POSITRON_BIT | ELECTRON_BIT;

fn run_print(args: &[&str]) -> Vec<u8> {
    let output = Command::new(env!("CARGO_BIN_EXE_beamdpr"))
        .args(args)
        .output()
        .expect("failed to run beamdpr");
    assert!(
        output.status.success(),
        "beamdpr failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    output.stdout
}

fn latch_from_chunk(chunk: &[u8]) -> u32 {
    let latch = i32::from_le_bytes(chunk[24..28].try_into().unwrap());
    latch as u32
}

fn write_mode0_phsp(path: &Path, latches: &[u32]) {
    let mut file = File::create(path).unwrap();
    file.write_all(b"MODE0").unwrap();
    file.write_all(&(latches.len() as i32).to_le_bytes()).unwrap();
    let photons = latches
        .iter()
        .filter(|&&latch| latch & CHARGED_MASK == 0)
        .count() as i32;
    file.write_all(&photons.to_le_bytes()).unwrap();
    file.write_all(&1.0f32.to_le_bytes()).unwrap(); // max energy
    file.write_all(&0.0f32.to_le_bytes()).unwrap(); // min energy
    file.write_all(&(latches.len() as f32).to_le_bytes()).unwrap();
    file.write_all(&[0u8; 3]).unwrap(); // MODE0 header padding (record_size - 25)

    for &latch in latches {
        file.write_all(&(latch as i32).to_le_bytes()).unwrap();
        file.write_all(&1.0f32.to_le_bytes()).unwrap(); // total_energy
        file.write_all(&0.0f32.to_le_bytes()).unwrap(); // x
        file.write_all(&0.0f32.to_le_bytes()).unwrap(); // y
        file.write_all(&0.0f32.to_le_bytes()).unwrap(); // x_cos
        file.write_all(&0.0f32.to_le_bytes()).unwrap(); // y_cos
        file.write_all(&1.0f32.to_le_bytes()).unwrap(); // weight
    }
}

#[test]
fn print_particle_photon_only() {
    let out = run_print(&[
        "print",
        "--field",
        "energy",
        "x",
        "y",
        "x_cos",
        "y_cos",
        "weight",
        "latch",
        "--number",
        "200",
        "--particle",
        "photon",
        "test_data/first.egsphsp1",
    ]);
    assert_eq!(out.len(), 200 * RECORD_BYTES);
    for chunk in out.chunks_exact(RECORD_BYTES) {
        let latch = latch_from_chunk(chunk);
        assert_eq!(
            latch & CHARGED_MASK,
            0,
            "expected photon (charged bits not set), latch={latch}"
        );
    }
}

#[test]
fn print_particle_charged_only() {
    let out = run_print(&[
        "print",
        "--field",
        "energy",
        "x",
        "y",
        "x_cos",
        "y_cos",
        "weight",
        "latch",
        "--number",
        "200",
        "--particle",
        "charged",
        "test_data/first.egsphsp1",
    ]);
    assert_eq!(out.len(), 200 * RECORD_BYTES);
    for chunk in out.chunks_exact(RECORD_BYTES) {
        let latch = latch_from_chunk(chunk);
        assert_ne!(
            latch & CHARGED_MASK,
            0,
            "expected charged particle (charged bits set), latch={latch}"
        );
    }
}

#[test]
fn print_number_applies_after_particle_filter() {
    let expected_charged =
        PHSPReader::from(File::open(Path::new("test_data/first.egsphsp1")).unwrap())
            .unwrap()
            .map(|r| r.unwrap())
            .filter(|r| r.charged())
            .count();

    let out = run_print(&[
        "print",
        "--field",
        "energy",
        "x",
        "y",
        "x_cos",
        "y_cos",
        "weight",
        "latch",
        "--number",
        "50000",
        "--particle",
        "charged",
        "test_data/first.egsphsp1",
    ]);
    assert_eq!(out.len() / RECORD_BYTES, expected_charged);
}

#[test]
fn print_without_number_prints_all_matching_records() {
    let expected_charged =
        PHSPReader::from(File::open(Path::new("test_data/first.egsphsp1")).unwrap())
            .unwrap()
            .map(|r| r.unwrap())
            .filter(|r| r.charged())
            .count();

    let out = run_print(&[
        "print",
        "--field",
        "energy",
        "x",
        "y",
        "x_cos",
        "y_cos",
        "weight",
        "latch",
        "--particle",
        "charged",
        "test_data/first.egsphsp1",
    ]);
    assert_eq!(out.len() / RECORD_BYTES, expected_charged);
}

#[test]
fn print_particle_treats_positrons_as_charged() {
    let mut path = std::env::temp_dir();
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
    path.push(format!(
        "beamdpr_print_particle_{}_{}.egsphsp1",
        std::process::id(),
        now
    ));
    let path_string = path.to_string_lossy().to_string();

    write_mode0_phsp(&path, &[0, ELECTRON_BIT, POSITRON_BIT]);

    let charged = run_print(&[
        "print",
        "--field",
        "energy",
        "x",
        "y",
        "x_cos",
        "y_cos",
        "weight",
        "latch",
        "--number",
        "100",
        "--particle",
        "charged",
        &path_string,
    ]);
    assert_eq!(charged.len(), 2 * RECORD_BYTES);
    for chunk in charged.chunks_exact(RECORD_BYTES) {
        assert_ne!(latch_from_chunk(chunk) & CHARGED_MASK, 0);
    }

    let photons = run_print(&[
        "print",
        "--field",
        "energy",
        "x",
        "y",
        "x_cos",
        "y_cos",
        "weight",
        "latch",
        "--number",
        "100",
        "--particle",
        "photon",
        &path_string,
    ]);
    assert_eq!(photons.len(), RECORD_BYTES);
    for chunk in photons.chunks_exact(RECORD_BYTES) {
        assert_eq!(latch_from_chunk(chunk) & CHARGED_MASK, 0);
    }

    remove_file(path).unwrap();
}
