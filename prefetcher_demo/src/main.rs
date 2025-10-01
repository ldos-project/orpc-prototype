use std::{
    env,
    fs::OpenOptions,
    io::{Seek, Write, stdin},
    thread::{self, sleep},
    time::Duration,
};

use crate::{
    block_device::{BLOCK_SIZE, BlockDevice},
    shutdown::Shutdown as _,
};

extern crate alloc;

mod block_device;
mod cache;
mod disk;
mod prefetch_policy;
pub(crate) mod shutdown;

struct Config {
    stride: usize,
}

impl Config {
    fn parse() -> Result<Config, &'static str> {
        let args: Vec<String> = env::args().collect();
        if args.len() != 2 {
            return Err("Usage: cargo run <stride>");
        }
        let stride: usize = match args[1].parse() {
            Ok(num) => num,
            Err(_) => return Err("Error: stride must be a number"),
        };
        Ok(Config { stride })
    }
}

const N_BLOCKS: usize = 1024;

fn main() {
    let config = Config::parse().expect("Failed to parse configuration");

    let block_file = setup_block_file();

    // Start the disk server
    let disk = disk::Disk::spawn(block_file).unwrap();
    // Start the cache server attached to the disk.
    let cache = cache::Cache::spawn(disk.clone(), 16).unwrap();

    // Setup a cache policy. You can actually have more than one and they will all send prefetch commands which will all
    // be used.
    let policy = prefetch_policy::NextPagePrefetcher::spawn(cache.clone()).unwrap();

    // A client (would be a server in the real thing)
    let client = thread::spawn({
        let cache = cache.clone();
        move || {
            let mut i = 0;
            loop {
                cache.read((i * config.stride) % N_BLOCKS).unwrap();
                sleep(Duration::from_secs(1));
                i += 1;
            }
        }
    });

    // Wait for [enter].
    let mut line = Default::default();
    stdin().read_line(&mut line).expect("stdin read failed");
    println!("\nShutting down policy!\n");

    // Attempt to shutdown policy and report the error if it fails.
    if let Err(e) = policy.shutdown() {
        println!("Error in sync call shutting down policy: {}", e);
    }

    // Wait for [enter].
    stdin().read_line(&mut line).expect("stdin read failed");
    println!("\nStarting new policy!\n");

    // Start the new policy
    let _policy = prefetch_policy::StrongStridePrefetcher::spawn(cache.clone()).unwrap();

    client.join().unwrap();
}

/// Setup a file which stands in for a block device.
fn setup_block_file() -> std::fs::File {
    let mut block_file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open("/tmp/block_tmp_file")
        .unwrap();
    block_file
        .seek(std::io::SeekFrom::Current((N_BLOCKS * BLOCK_SIZE) as i64))
        .unwrap();
    block_file.write(&[0]).unwrap();
    block_file
}
