use std::error::Error;

use orpc::{
    CurrentServer, ServerRef,
    oqueue::{Sender, StrongObserver},
    orpc_server, spawn_thread,
};

use crate::block_device::{BlockCache, BlockDevice, ReadRequest};

#[orpc_server()]
pub struct Prefetcher {}

impl Prefetcher {
    pub fn spawn(cache: ServerRef<dyn BlockCache>) -> Result<ServerRef<Self>, Box<dyn Error>> {
        let read_observer = cache
            .read_request_observation_oqueue()
            .attach_strong_observer()?;
        let prefetch_sender = cache.prefetch_request_oqueue().attach_sender()?;
        let server = Self::new_with(|orpc_internal| Self { orpc_internal });
        spawn_thread(server.clone(), {
            move || {
                loop {
                    if let Some(block_id) = read_observer.try_strong_observe() {
                        prefetch_sender.send(block_id + 4);
                    }
                    CurrentServer::abort_point();
                }
            }
        });
        Ok(server)
    }
}