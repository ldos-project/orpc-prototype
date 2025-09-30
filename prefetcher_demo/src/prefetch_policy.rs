#![allow(unused)]
//! Several prefetching policies that operate on a [`BlockCache`].

use std::error::Error;

use orpc::{CurrentServer, ServerRef, orpc_server, spawn_thread};

use crate::block_device::BlockCache;

/// A policy which always prefetches a page a fixed distance in the future.
#[orpc_server()]
pub struct NextPagePrefetcher {}

impl NextPagePrefetcher {
    pub fn spawn(cache: ServerRef<dyn BlockCache>) -> Result<ServerRef<Self>, Box<dyn Error>> {
        let server = Self::new_with(|orpc_internal| Self { orpc_internal });
        spawn_thread(server.clone(), {
            let read_observer = cache
                .read_request_observation_oqueue()
                .attach_strong_observer()?;
            let prefetch_sender = cache.prefetch_request_oqueue().attach_sender()?;
            move || {
                loop {
                    let block_id = read_observer.strong_observe();
                    prefetch_sender.send(block_id + 8);
                    CurrentServer::abort_point();
                }
            }
        });
        Ok(server)
    }
}

/// A prefetcher which prefetches based on a weak observer only. This can miss reads and not prefetch based on them at
/// all. It selects the block to prefetch based on the stride between the previous two reads.
#[orpc_server()]
pub struct WeakStridePrefetcher {}

impl WeakStridePrefetcher {
    pub fn spawn(cache: ServerRef<dyn BlockCache>) -> Result<ServerRef<Self>, Box<dyn Error>> {
        let server = Self::new_with(|orpc_internal| Self { orpc_internal });
        spawn_thread(server.clone(), {
            let read_observer = cache
                .read_request_observation_oqueue()
                .attach_weak_observer()?;
            let prefetch_sender = cache.prefetch_request_oqueue().attach_sender()?;
            move || {
                loop {
                    read_observer.wait();
                    let now = read_observer.recent_cursor();
                    let reads = read_observer.weak_observer_range(now - 2, now);
                    if reads.len() == 2 {
                        let stride = reads[1] - reads[0];
                        prefetch_sender.send(reads[1] + stride * 8);
                    }
                    CurrentServer::abort_point();
                }
            }
        });
        Ok(server)
    }
}

/// A prefetcher which prefetches using a strong observer. It will always generate one prefetch for every read. It
/// selects the block to prefetch based on the stride between the previous two reads.
#[orpc_server()]
pub struct StrongStridePrefetcher {}

impl StrongStridePrefetcher {
    pub fn spawn(cache: ServerRef<dyn BlockCache>) -> Result<ServerRef<Self>, Box<dyn Error>> {
        let server = Self::new_with(|orpc_internal| Self { orpc_internal });
        spawn_thread(server.clone(), {
            let strong_observer = cache
                .read_request_observation_oqueue()
                .attach_strong_observer()?;
            let weak_observer = cache
                .read_request_observation_oqueue()
                .attach_weak_observer()?;
            let prefetch_sender = cache.prefetch_request_oqueue().attach_sender()?;
            move || {
                loop {
                    let block_id = strong_observer.strong_observe();

                    let now = weak_observer.recent_cursor();
                    let reads = weak_observer.weak_observer_range(now - 2, now);

                    if reads.len() == 2 {
                        let stride = reads[1] - reads[0];
                        prefetch_sender.send(block_id + stride * 8);
                    }
                    CurrentServer::abort_point();
                }
            }
        });
        Ok(server)
    }
}
