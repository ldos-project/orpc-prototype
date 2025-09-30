use std::{
    fs::OpenOptions,
    io::{Seek, Write},
    thread::sleep,
    time::Duration,
};

use crate::block_device::{BLOCK_SIZE, BlockDevice};

extern crate alloc;

mod block_device;
mod cache;
mod disk;
mod prefetch_policy;

fn main() {
    let block_file = setup_block_file();

    // Start the disk server
    let disk = disk::Disk::spawn(block_file).unwrap();
    // Start the cache server attached to the disk.
    let cache = cache::Cache::spawn(disk.clone(), 16).unwrap();

    // Setup a cache policy. You can actually have more than one and they will all send prefetch commands which will all
    // be used.
    let _policy = prefetch_policy::StrongStridePrefetcher::spawn(cache.clone()).unwrap();
    // let policy = prefetch_policy::NextPagePrefetcher::spawn(cache.clone()).unwrap();

    for i in 0..20 {
        cache.read(i * 3).unwrap();
        sleep(Duration::from_nanos(10));
    }
}

/// Setup a file which stands in for a block device.
fn setup_block_file() -> std::fs::File {
    const N_BLOCKS: usize = 1024;

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
