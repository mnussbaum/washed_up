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

use coroutine::Error as CoroutineError;
use rustc_serialize::json::{
    Json,
};
use uuid::Uuid;

use actor::Actor;

#[derive(Debug)]
pub enum WashedUpError<'a> {
    ActorReadLockPoisoned(PoisonError<RwLockReadGuard<'a, HashMap<Uuid, Actor>>>),
    ActorSendError(SendError<Json>),
    ActorWriteLockPoisoned(PoisonError<RwLockWriteGuard<'a, HashMap<Uuid, Actor>>>),
    Coroutine(CoroutineError),
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
            WashedUpError::Coroutine(ref err) => err.description(),
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
            WashedUpError::Coroutine(ref err) => Some(err),
            WashedUpError::InvalidPid(_) => None,
        }
    }
}

impl<'a> fmt::Display for WashedUpError<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self.description())
    }
}

impl<'a> From<CoroutineError> for WashedUpError<'a> {
    fn from(err: CoroutineError) -> WashedUpError<'a> {
        WashedUpError::Coroutine(err)
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
