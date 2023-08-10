use std::{
    env,
    fs::File,
    io::{BufReader, BufWriter, Write},
    path::{Path, PathBuf},
    thread::sleep,
    time::Duration,
};
use tracing::{instrument, span, Level};
use tracing_flame::FlameLayer;
use tracing_subscriber::{prelude::*, registry::Registry};

fn main() {
    let mut iter = std::env::args();
    let _binary = iter.next().unwrap();
    let profile = iter.next().expect("Expected profile file");

    let file = std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(profile)
        .unwrap();
    let buffer = std::io::BufWriter::new(file);

    let flame_layer = FlameLayer::new(buffer);
    let guard = flame_layer.flush_on_drop();
    let collector = Registry::default().with(flame_layer);
    tracing::subscriber::set_global_default(collector).unwrap();

    outer();
    sleep(Duration::from_micros(5));
    drop(guard);
}

#[instrument]
fn outer() {
    sleep(Duration::from_micros(100));
    inner_one();
    sleep(Duration::from_micros(500));
    inner_two();
}
#[instrument]
fn inner_one() {
    sleep(Duration::from_micros(4000));
    innermost();
}
#[instrument]
fn inner_two() {
    sleep(Duration::from_micros(500));
    innermost();
}
#[instrument]
fn innermost() {
    sleep(Duration::from_micros(400));
}
