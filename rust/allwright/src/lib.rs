#[cfg(feature = "client")]
mod client;
#[cfg(feature = "server")]
#[path = "engine.rs"]
mod engine_lib;
#[cfg(feature = "server")]
#[allow(dead_code)]
#[path = "platform/android.rs"]
mod android_lib;
#[cfg(feature = "server")]
#[allow(dead_code)]
#[path = "platform/desktop.rs"]
mod desktop_lib;
#[cfg(feature = "server")]
#[allow(dead_code)]
#[path = "platform/ios.rs"]
mod ios_lib;
#[cfg(feature = "server")]
#[allow(dead_code)]
#[path = "platform/linux.rs"]
mod linux_lib;
#[cfg(feature = "server")]
#[allow(dead_code)]
#[path = "platform/macos.rs"]
mod macos_lib;
#[cfg(feature = "server")]
#[allow(dead_code)]
#[path = "platform/mobile.rs"]
mod mobile_lib;
#[cfg(feature = "server")]
#[allow(dead_code)]
#[path = "platform/web.rs"]
mod web_lib;
#[cfg(feature = "server")]
#[allow(dead_code)]
#[path = "platform/windows.rs"]
mod windows_lib;

pub mod proto {
    tonic::include_proto!("allwright.engine.v1");
}

#[cfg(feature = "client")]
pub use client::*;
#[cfg(feature = "server")]
pub use engine_lib::serve;
