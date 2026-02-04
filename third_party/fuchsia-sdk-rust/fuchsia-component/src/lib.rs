#![allow(unused)]

pub mod client {
    use fidl::endpoints::{DiscoverableProtocolMarker, Proxy};
    
    pub fn connect_to_protocol<P: DiscoverableProtocolMarker>() -> Result<P::Proxy, Error> {
        unimplemented!("connect_to_protocol placeholder - will connect to protocol: {}", P::DEBUG_NAME)
    }
    
    #[derive(Debug)]
    pub struct Error;
    
    impl std::fmt::Display for Error {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "Component client error")
        }
    }
    
    impl std::error::Error for Error {}
}

pub mod server {
    use futures::stream::Stream;
    use futures::task::{Context, Poll};
    use std::pin::Pin;
    
    pub struct ServiceFs<ServiceObjTy> {
        _phantom: std::marker::PhantomData<ServiceObjTy>,
    }
    
    impl<ServiceObjTy> ServiceFs<ServiceObjTy> {
        pub fn new() -> Self {
            log::info!("Creating ServiceFs");
            ServiceFs {
                _phantom: std::marker::PhantomData,
            }
        }
        
        pub fn new_local() -> Self {
            log::info!("Creating local ServiceFs");
            ServiceFs {
                _phantom: std::marker::PhantomData,
            }
        }
        
        pub fn dir(&mut self, _name: &str) -> ServiceFsDir<'_, ServiceObjTy> {
            ServiceFsDir { _fs: self }
        }
        
        pub fn take_and_serve_directory_handle(&mut self) -> Result<&mut Self, std::io::Error> {
            log::info!("ServiceFs: taking and serving directory handle");
            Ok(self)
        }
        
        pub async fn collect<T>(self) -> T {
            log::info!("ServiceFs: collecting (blocking forever)");
            futures::future::pending().await
        }
    }
    
    impl<ServiceObjTy> Stream for ServiceFs<ServiceObjTy> {
        type Item = ServiceObjTy;
        
        fn poll_next(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
            Poll::Pending
        }
    }
    
    pub struct ServiceFsDir<'a, ServiceObjTy> {
        _fs: &'a mut ServiceFs<ServiceObjTy>,
    }
    
    impl<'a, ServiceObjTy> ServiceFsDir<'a, ServiceObjTy> {
        pub fn add_fidl_service<F, RS>(self, _handler: F) -> ServiceFs<ServiceObjTy>
        where
            F: FnMut(RS) -> ServiceObjTy,
        {
            log::info!("ServiceFsDir: adding FIDL service");
            ServiceFs {
                _phantom: std::marker::PhantomData,
            }
        }
    }
}
