use std::collections::{
    HashMap,
};
use std::fmt;
use std::sync::{
    Arc,
    Mutex,
    RwLock,
};
use std::sync::mpsc::{
    channel,
    Receiver,
};

use coroutine::{
    builder,
    State,
};
use rustc_serialize::json::{
    Json,
};
use uuid::Uuid;

use actor::Actor;
use error::WashedUpError;

#[derive(Debug)]
pub struct Supervisor {
    actors: RwLock<HashMap<Uuid, Actor>>,
    pub name: String,
}

impl fmt::Display for Supervisor {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Supervisor {:?}", self.name)
    }
}

impl Supervisor {
    pub fn new(name: &str) -> Supervisor {
        let supervisor = Supervisor {
            actors: RwLock::new(HashMap::new()),
            name: name.to_string(),
        };
        info!("Instantiating new {:?}", supervisor);

        supervisor
    }

    pub fn join(&self, pid : Uuid) -> Result<State, WashedUpError> {
        let mut actors = try!(self.actors.write());
        match actors.remove(&pid) {
            Some(actor) => {
                Ok(try!(actor.join_handle.join()))
            },
            None => Err(WashedUpError::InvalidPid(pid)),
        }
    }

    pub fn send_message(&self, pid: Uuid, message: Json) -> Result<(), WashedUpError> {
        let actors = try!(self.actors.read());
        match actors.get(&pid) {
            Some(actor) => {
                try!(actor.mailbox.send(message));
                try!(actor.join_handle.resume());
                Ok(())
            },
            None => Err(WashedUpError::InvalidPid(pid)),
        }
    }

    pub fn spawn<F>(&self, actor_name: &str, body: F) -> Result<Uuid, WashedUpError>
        where F : 'static + Send + Fn(Receiver<Json>) -> () {
        let pid = Uuid::new_v4();
        let (mailbox_sender, mailbox_receiver) = channel();
        let arc_body = Arc::new(Mutex::new(body));

        let mut actors = try!(self.actors.write());
        let actor_handle = builder::Builder::new().name(pid.to_string()).spawn(move || {
            arc_body.lock().unwrap()(mailbox_receiver);
        });

        actors.insert(pid, Actor::new(
            actor_handle,
            mailbox_sender,
            actor_name.to_string(),
            pid,
        ));

        Ok(pid)
    }
}
