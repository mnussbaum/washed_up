extern crate coros;
#[macro_use] extern crate log;
extern crate rustc_serialize;
extern crate time;
extern crate uuid;

use std::result;
pub type Result<'a, T> = result::Result<T, WashedUpError<'a>>;

pub mod actor;
pub use actor::Actor;

pub mod error;
pub use error::WashedUpError;

pub mod supervisor;
pub use supervisor::Supervisor;
