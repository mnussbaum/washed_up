use std::any::Any;
use std::collections::{
    HashMap,
};
use std::error::{
    Error,
};
use std::fmt;
use std::sync::{
    RwLockReadGuard,
    RwLockWriteGuard,
    PoisonError,
};
use std::sync::mpsc::{
    SendError,
};

use rustc_serialize::json::{
    Json,
};
use uuid::Uuid;
use coros::CoroError;

use actor::Actor;

#[derive(Debug)]
pub enum WashedUpError<'a> {
    ActorReadLockPoisoned(PoisonError<RwLockReadGuard<'a, HashMap<Uuid, Actor>>>),
    ActorSendError(SendError<Json>),
    ActorWriteLockPoisoned(PoisonError<RwLockWriteGuard<'a, HashMap<Uuid, Actor>>>),
    ActorPanic(&'a str),
    CoroError(CoroError<'a>),
    InvalidPid(Uuid),
}

impl<'a> WashedUpError<'a> {
    pub fn description(&self) -> &str {
        match *self {
            WashedUpError::ActorReadLockPoisoned(_) => {
                "Supervisor's actor read lock poisoned"
            },
            WashedUpError::ActorSendError(ref err) => err.description(),
            WashedUpError::ActorWriteLockPoisoned(_) => {
                "Supervisor's actor write lock poisoned"
            },
            WashedUpError::ActorPanic(ref err_as_string) => err_as_string,
            WashedUpError::CoroError(ref err) => err.description(),
            WashedUpError::InvalidPid(_) => {
                "PID does not map to a spawned actor"
            },
        }
    }
}

impl<'a> Error for WashedUpError<'a> {
    fn description(&self) -> &str {
        self.description()
    }

    fn cause(&self) -> Option<&Error> {
        match *self {
            WashedUpError::ActorReadLockPoisoned(_) => None,
            WashedUpError::ActorSendError(ref err) => Some(err),
            WashedUpError::ActorWriteLockPoisoned(_) => None,
            WashedUpError::ActorPanic(_) => None,
            WashedUpError::CoroError(ref err) => Some(err),
            WashedUpError::InvalidPid(_) => None,
        }
    }
}

impl<'a> fmt::Display for WashedUpError<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self.description())
    }
}

impl<'a> From<Box<Any + Send>> for WashedUpError<'a> {
    fn from(maybe_err: Box<Any + Send>) -> WashedUpError<'a> {
         match maybe_err.downcast_ref::<&str>() {
             Some(err_as_string) => {
                 WashedUpError::ActorPanic(err_as_string)
             },
             None => {
                 WashedUpError::ActorPanic("Unknown panic in actor")
             }
         }
    }
}

impl<'a> From<SendError<Json>> for WashedUpError<'a> {
    fn from(err: SendError<Json>) -> WashedUpError<'a> {
        WashedUpError::ActorSendError(err)
    }
}

impl<'a> From<Uuid> for WashedUpError<'a> {
    fn from(invalid_pid: Uuid) -> WashedUpError<'a> {
        WashedUpError::InvalidPid(invalid_pid)
    }
}

impl<'a> From<PoisonError<RwLockReadGuard<'a, HashMap<Uuid, Actor>>>> for WashedUpError<'a> {
    fn from(err: PoisonError<RwLockReadGuard<'a, HashMap<Uuid, Actor>>>) -> WashedUpError<'a> {
        error!("Error obtaining actor read lock {:?}", err);
        WashedUpError::ActorReadLockPoisoned(err)
    }
}

impl<'a> From<PoisonError<RwLockWriteGuard<'a, HashMap<Uuid, Actor>>>> for WashedUpError<'a> {
    fn from(err: PoisonError<RwLockWriteGuard<'a, HashMap<Uuid, Actor>>>) -> WashedUpError<'a> {
        error!("Error obtaining actor write lock {:?}", err);
        WashedUpError::ActorWriteLockPoisoned(err)
    }
}

impl<'a> From<CoroError<'a>> for WashedUpError<'a> {
    fn from(err: CoroError<'a>) -> WashedUpError<'a> {
        WashedUpError::CoroError(err)
    }
}
