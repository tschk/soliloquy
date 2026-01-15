#![allow(unused)]

pub use futures::prelude::*;
pub use futures::StreamExt;

pub struct Executor;

impl Executor {
    pub fn new() -> Result<Self, std::io::Error> {
        Ok(Executor)
    }
    
    pub fn run_singlethreaded<F>(fut: F) -> F::Output
    where
        F: std::future::Future,
    {
        futures::executor::block_on(fut)
    }
}

pub fn run_singlethreaded<F>(fut: F) -> F::Output
where
    F: std::future::Future,
{
    futures::executor::block_on(fut)
}

