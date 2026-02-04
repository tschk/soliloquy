// Generated bindings for fuchsia.ui.app
// To generate actual bindings, ensure Fuchsia SDK is installed
// and FIDL sources are available, then run:
//   ./tools/soliloquy/gen_fidl_bindings.sh

#![allow(unused)]

use fidl::endpoints::{ControlHandle as _, Responder as _};
pub use fidl::endpoints::{
    create_endpoints, create_proxy, create_request_stream, ClientEnd, DiscoverableProtocolMarker,
    Proxy, ServerEnd, ServiceMarker,
};
pub use fidl::Error;

pub mod fidl_fuchsia_ui_app {
    use super::*;
    use fidl::encoding::{Decodable, Encodable};

    #[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
    pub struct ViewProviderMarker;
    
    impl fidl::endpoints::ProtocolMarker for ViewProviderMarker {
        type Proxy = ViewProviderProxy;
        type RequestStream = ViewProviderRequestStream;
        const DEBUG_NAME: &'static str = "(anonymous) ViewProvider";
    }

    impl fidl::endpoints::DiscoverableProtocolMarker for ViewProviderMarker {
        const PROTOCOL_NAME: &'static str = "fuchsia.ui.app.ViewProvider";
    }

    pub type ViewProviderProxy = fidl::endpoints::Proxy<ViewProviderMarker>;
    pub struct ViewProviderRequestStream {
        _marker: std::marker::PhantomData<ViewProviderMarker>,
    }

    impl fidl::RequestStream for ViewProviderRequestStream {
        type Protocol = ViewProviderMarker;
        type ControlHandle = ViewProviderControlHandle;

        fn from_channel(_channel: fidl::AsyncChannel) -> Self {
             Self { _marker: std::marker::PhantomData }
        }

        fn control_handle(&self) -> Self::ControlHandle {
            ViewProviderControlHandle { _inner: std::sync::Arc::new(fidl::ServeInner) }
        }

        fn into_inner(self) -> (std::sync::Arc<fidl::ServeInner>, bool) {
            (std::sync::Arc::new(fidl::ServeInner), false)
        }

        fn from_inner(_inner: std::sync::Arc<fidl::ServeInner>, _is_terminated: bool) -> Self {
             Self { _marker: std::marker::PhantomData }
        }
    }

    pub enum ViewProviderRequest {
        CreateView {
            token: fidl_fuchsia_ui_views::ViewCreationToken,
            control_handle: ViewProviderControlHandle,
        },
        CreateView2 {
            args: CreateView2Args,
            control_handle: ViewProviderControlHandle,
        },
    }

    // impl futures::stream::Stream for ViewProviderRequestStream {
    //     type Item = Result<ViewProviderRequest, fidl::Error>;
        
    //     fn poll_next(
    //         self: std::pin::Pin<&mut Self>,
    //         _cx: &mut std::task::Context<'_>,
    //     ) -> std::task::Poll<Option<Self::Item>> {
    //         std::task::Poll::Pending
    //     }
    // }

    #[derive(Debug, Clone)]
    pub struct ViewProviderControlHandle {
        _inner: std::sync::Arc<fidl::ServeInner>,
    }

    impl ViewProviderControlHandle {
        pub fn shutdown(&self) {
            unimplemented!("ViewProviderControlHandle placeholder")
        }

        pub fn shutdown_with_epitaph(&self, _status: fidl::Status) {
            unimplemented!("ViewProviderControlHandle placeholder")
        }
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct CreateView2Args {
        pub view_creation_token: fidl_fuchsia_ui_views::ViewCreationToken,
    }
}

pub use fidl_fuchsia_ui_app::*;

mod fidl_fuchsia_ui_views {
    use super::*;

    #[derive(Debug, Clone, PartialEq)]
    pub struct ViewCreationToken {
        pub value: fidl::EventPair,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct ViewportCreationToken {
        pub value: fidl::EventPair,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct ViewRef {
        pub reference: fidl::EventPair,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct ViewRefControl {
        pub reference: fidl::EventPair,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct ViewIdentityOnCreation {
        pub view_ref: ViewRef,
        pub view_ref_control: ViewRefControl,
    }
}
