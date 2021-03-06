//! a rust wrapper of `libplctag`, with rust style APIs and useful extensions.
//!
//! ## How to use
//!
//! Download latest binary release of [libplctag](https://github.com/libplctag/libplctag/releases) and extract it to somewhere of your computer.
//!
//! Set environment variable `LIBPLCTAG_PATH` to the directory of extracted binaries.
//!
//! Add `plctag` to your Cargo.toml
//!
//! ```toml
//! [dependencies]
//! plctag= { git="https://github.com/Joylei/plctag-rs.git"}
//! ```
//!
//! You're OK to build your project.
//!
//! ```shell
//! cargo build
//! ```
//!
//!
//! # Examples
//! ## read/write tag
//! ```rust,ignore
//! use plctag::{Accessor, RawTag};
//! let timeout = 100;//ms
//!
//! // YOUR TAG DEFINITION
//! let path="protocol=ab-eip&plc=controllogix&path=1,0&gateway=192.168.1.120&name=MyTag1&elem_count=1&elem_size=16";
//! let tag = RawTag::new(path, timeout).unwrap();
//!
//! //read tag
//! let status = tag.read(timeout);
//! assert!(status.is_ok());
//! let offset = 0;
//! let value:u16 = tag.get_value(offset).unwrap();
//! println!("tag value: {}", value);
//!
//! let value = value + 10;
//! tag.set_value(offset, value).unwrap();
//!
//! //write tag
//! let status = tag.write(timeout);
//! assert!(status.is_ok());
//! println!("write done!");
//!
//! // tag will be destroyed when out of scope or manually call drop()
//! drop(tag);
//! ```
//!
//! ## UDT
//! read/write UDT
//! ```rust, ignore
//! use plctag::{Accessor, RawTag, Result, TagValue};
//!
//! // define your UDT
//! #[derive(Default, Debug)]
//! struct MyUDT {
//!     v1:u16,
//!     v2:u16,
//! }
//! impl TagValue for MyUDT {
//!     fn get_value(&mut self, tag: &RawTag, offset: u32) -> Result<()>{
//!         self.v1.get_value(tag, offset)?;
//!         self.v2.get_value(tag, offset + 2)?;
//!         Ok(())
//!     }
//!
//!     fn set_value(&self, tag: &RawTag, offset: u32) -> Result<()>{
//!         self.v1.set_value(tag, offset)?;
//!         self.v2.set_value(tag, offset + 2)?;
//!         Ok(())
//!     }
//! }
//!
//! fn main(){
//!     let timeout = 100;//ms
//!     let path="protocol=ab-eip&plc=controllogix&path=1,0&gateway=192.168.1.120&name=MyTag2&elem_count=2&elem_size=16";// YOUR TAG DEFINITION
//!     let tag = RawTag::new(path, timeout).unwrap();
//!
//!     //read tag
//!     let status = tag.read(timeout);
//!     assert!(status.is_ok());
//!     let offset = 0;
//!     let mut value:MyUDT = tag.get_value(offset).unwrap();
//!     println!("tag value: {:?}", value);
//!
//!     value.v1 = value.v1 + 10;
//!     tag.set_value(offset, value).unwrap();
//!
//!     //write tag
//!     let status = tag.write(timeout);
//!     assert!(status.is_ok());
//!     println!("write done!");
//! }
//!
//! ```
//!
//! Note:
//! Do not perform expensive operations when you implements `TagValue`.
//!
//! ## Builder
//! ```rust,ignore
//! use plctag::builder::*;
//! use plctag::RawTag;
//!
//! fn main() {
//!     let timeout = 100;
//!     let path = PathBuilder::default()
//!         .protocol(Protocol::EIP)
//!         .gateway("192.168.1.120")
//!         .plc(PlcKind::ControlLogix)
//!         .name("MyTag1")
//!         .element_size(16)
//!         .element_count(1)
//!         .path("1,0")
//!         .read_cache_ms(0)
//!         .build()
//!         .unwrap();
//!     let tag = RawTag::new(path, timeout).unwrap();
//!     let status = tag.status();
//!     assert!(status.is_ok());
//! }
//!
//! ```
//!
//! ## Logging adapter for `libplctag`
//! ```rust,ignore
//! use plctag::logging::log_adapt;
//! use plctag::plc::set_debug_level;
//! use plctag::DebugLevel;
//!
//! log_adapt(); //register logger
//! set_debug_level(DebugLevel::Info); // set debug level
//!
//! // now, you can receive log messages by any of logging implementations of crate `log`
//!
//! ```
//!
//! # Thread-safety
//! Operations are not thread-safe in this library, please use `std::sync::Mutex` or something similar to enforce thread-safety.
//!

pub const LIBPLCTAG_VERSION: (usize, usize, usize) = (2, 1, 22);

#[macro_use]
extern crate log;
#[macro_use]
extern crate anyhow;
#[cfg(feature = "controller")]
extern crate parking_lot;
#[cfg(any(feature = "value"))]
extern crate paste;

pub mod builder;
pub(crate) mod debug;
pub(crate) mod ffi;
pub mod plc;
pub(crate) mod raw;
pub mod status;
#[cfg(feature = "value")]
pub(crate) mod value;

pub use debug::DebugLevel;
pub use raw::RawTag;
pub use status::Status;

#[cfg(feature = "value")]
pub use value::{Accessor, TagValue};

pub type Result<T> = std::result::Result<T, Status>;

pub mod prelude {
    #[cfg(feature = "value")]
    pub use crate::{Accessor, TagValue};
    pub use crate::{DebugLevel, RawTag, Result, Status};
}

/// handle internal log messages of `libplctag`
pub mod logging {
    use crate::plc;
    use crate::status;
    use std::ffi::CStr;
    use std::os::raw::c_char;

    #[doc(hidden)]
    #[no_mangle]
    unsafe extern "C" fn log_route(_tag_id: i32, level: i32, message: *const c_char) {
        let msg = CStr::from_ptr(message).to_string_lossy();
        match level {
            1 => error!("{}", msg),
            2 => warn!("{}", msg),
            3 => info!("{}", msg),
            4 => debug!("{}", msg),
            5 => trace!("{}", msg),
            _ => (),
        }
    }

    /// by default, `libplctag` logs internal messages to stdout, if you set debug level other than none.
    /// you can register your own logger by calling [plc::register_logger](../plc/fn.register_logger.html).
    /// For convenient, this method will register a logger for you and will forward internal log messages to crate`log`.
    ///
    /// # Note
    /// `libplctag` will print log messages to stdout even if you register your own logger by `plc::register_logger`.
    ///
    /// # Examples
    /// ```rust,ignore
    /// use plctag::logging::log_adapt;
    /// use plctag::plc::set_debug_level;
    /// use plctag::DebugLevel;
    ///
    /// log_adapt(); //register logger
    /// set_debug_level(DebugLevel::Info); // set debug level
    ///
    /// // now, you can receive log messages by any of logging implementations of crate `log`
    ///
    /// ```
    pub fn log_adapt() {
        unsafe {
            plc::unregister_logger();
            let rc = plc::register_logger(Some(log_route));
            debug_assert_eq!(rc, status::PLCTAG_STATUS_OK);
            info!("register logger for libplctag: {}", rc);
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use crate::plc;
        use crate::DebugLevel;
        use crate::RawTag;
        use log::*;
        use std::sync::{Arc, Mutex};

        struct MemLogger {
            buf: Arc<Mutex<Vec<String>>>,
        }

        impl MemLogger {
            fn new() -> Self {
                Self {
                    buf: Arc::new(Mutex::new(vec![])),
                }
            }

            fn buf(&self) -> Vec<String> {
                self.buf.lock().unwrap().clone()
            }

            fn init(&self) {
                log::set_max_level(LevelFilter::Trace);
                let _ = log::set_boxed_logger(Box::new(self.clone()));
            }
        }

        impl Clone for MemLogger {
            fn clone(&self) -> Self {
                Self {
                    buf: self.buf.clone(),
                }
            }
        }

        impl Log for MemLogger {
            fn enabled(&self, meta: &log::Metadata<'_>) -> bool {
                meta.level() <= Level::Error
            }
            fn log(&self, record: &log::Record<'_>) {
                self.buf
                    .lock()
                    .unwrap()
                    .push(format!("{} - {}", record.target(), record.args()));
            }
            fn flush(&self) {}
        }

        #[test]
        fn test_log_adapt() {
            let logger = MemLogger::new();
            logger.init();
            log_adapt();
            plc::set_debug_level(DebugLevel::Detail);

            let res = RawTag::new("make=system&family=library&name=debug&debug=4", 100);
            assert!(res.is_ok());
            let tag = res.unwrap();
            let status = tag.status();
            assert!(status.is_ok());

            let buf = logger.buf();
            assert!(buf.len() > 0);
            let msg = buf.join("\r\n");
            assert!(msg.contains("plc_tag_create"));
        }
    }
}
