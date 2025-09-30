use std::{
    fs::{File, OpenOptions},
    io::{Seek, Write},
    path::Path,
    thread::sleep,
    time::Duration,
};

use snafu::Whatever;

use crate::block_device::{BLOCK_SIZE, BlockCache, BlockDevice};

extern crate alloc;

mod block_device;
mod cache;
mod disk;
mod prefetch_policy;

const N_BLOCKS: usize = 1024;

fn main() {
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

    let disk = disk::Disk::spawn(block_file).unwrap();
    let cache = cache::Cache::spawn(disk.clone(), 16).unwrap();
    // let policy = prefetch_policy::NextPagePrefetcher::spawn(cache.clone()).unwrap();
    let policy = prefetch_policy::StrongStridePrefetcher::spawn(cache.clone()).unwrap();

    for i in 0..100 {
        cache.read(i*3).unwrap();
        // sleep(Duration::from_millis(1));
    }
}
