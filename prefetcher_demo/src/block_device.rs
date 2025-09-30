//! Traits for block devices.

use orpc::{
    oqueue::{
        OQueueAttachError, OQueueRef, Sender,
        locking::{LockingQueue, ObservableLockingQueue},
    },
    orpc_impl::errors::RPCError,
    orpc_trait,
};
use snafu::Snafu;

pub const BLOCK_SIZE: usize = 4096;

#[derive(Clone)]
pub(crate) struct Block {
    pub block_id: usize,
    pub data: Box<[u8]>,
}

pub(crate) struct ReadRequest {
    pub block_id: usize,
    pub reply_to: Box<dyn Sender<Block>>,
}

#[derive(Snafu, Debug)]
pub(crate) enum IOError {
    #[snafu(transparent)]
    RPCError { source: RPCError },
    #[snafu(transparent)]
    OQueueAttachError { source: OQueueAttachError },
    #[snafu(display("{message}"))]
    IOFailure { message: String },
}

#[orpc_trait]
pub(crate) trait BlockDevice {
    fn read(&self, block_id: usize) -> Result<Block, IOError>;
    fn read_request_observation_oqueue(&self) -> OQueueRef<usize> {
        ObservableLockingQueue::new(16, 4)
    }
    fn read_request_oqueue(&self) -> OQueueRef<ReadRequest> {
        LockingQueue::new(16)
    }
}

#[orpc_trait]
pub(crate) trait BlockCache: BlockDevice {
    fn prefetch_request_oqueue(&self) -> OQueueRef<usize> {
        LockingQueue::new(16)
    }
}
