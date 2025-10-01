#![allow(unused)]
//! Several prefetching policies that operate on a [`BlockCache`].

use std::error::Error;

use orpc::{CurrentServer, ServerRef, orpc_impl, orpc_server, spawn_thread};

use crate::{
    block_device::BlockCache,
    shutdown::{Shutdown, ShutdownState},
};

/// A policy which always prefetches a page a fixed distance in the future.
#[orpc_server(crate::shutdown::Shutdown)]
pub struct NextPagePrefetcher {
    shutdown: ShutdownState,
}

#[orpc_impl]
impl Shutdown for NextPagePrefetcher {
    // An implementation of a method on the Shutdown trait
    fn shutdown(&self) -> Result<(), orpc_impl::errors::RPCError> {
        self.shutdown.shutdown();
        panic!("'Accidental' failure.")
    }
}

impl NextPagePrefetcher {
    pub fn spawn(cache: ServerRef<dyn BlockCache>) -> Result<ServerRef<Self>, Box<dyn Error>> {
        let server = Self::new_with(|orpc_internal| Self {
            orpc_internal,
            shutdown: Default::default(),
        });
        // Spawn a server thread
        spawn_thread(server.clone(), {
            // Setup OQueue connections.
            let read_observer = cache
                .read_request_observation_oqueue()
                .attach_strong_observer()?;
            let prefetch_sender = cache.prefetch_request_oqueue().attach_sender()?;
            let server = server.clone();
            // Actual Server Loop.
            move || {
                loop {
                    let block_id = read_observer.strong_observe();
                    prefetch_sender.send(block_id + 1);
                    CurrentServer::abort_point();
                    server.shutdown.check_for_shutdown()?;
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
            // Setup OQueue Attachments
            let strong_observer = cache
                .read_request_observation_oqueue()
                .attach_strong_observer()?;
            let weak_observer = cache
                .read_request_observation_oqueue()
                .attach_weak_observer()?;
            let prefetch_sender = cache.prefetch_request_oqueue().attach_sender()?;
            // Server implementation
            move || {
                loop {
                    // Wait for the next read.
                    let block_id = strong_observer.strong_observe();

                    // Get the last two
                    let reads = weak_observer.weak_observe_recent(2);

                    // Prefetch algorithm
                    if reads.len() == 2 {
                        let stride = reads[1] - reads[0];
                        prefetch_sender.send(block_id + stride);
                    }

                    // Provide point for exiting the server if it crashed.
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
                    let reads = read_observer.weak_observe_recent(2);
                    if reads.len() == 2 {
                        let stride = reads[1] - reads[0];
                        prefetch_sender.send(reads[1] + stride);
                    }
                    CurrentServer::abort_point();
                }
            }
        });
        Ok(server)
    }
}
