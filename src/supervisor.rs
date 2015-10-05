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

use coros::{
    Pool,
};
use rustc_serialize::json::{
    Json,
};
use uuid::Uuid;

use Result;
use actor::Actor;
use error::WashedUpError;

#[derive(Debug)]
pub struct Supervisor {
    actors: RwLock<HashMap<Uuid, Actor>>,
    actor_pool: Pool,
    pub name: String,
}

impl fmt::Display for Supervisor {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Supervisor {:?}", self.name)
    }
}

impl Supervisor {
    pub fn new(name: &str) -> Supervisor {
        let pool = Pool::new(name.to_string(), 4);
        pool.start().unwrap();
        let supervisor = Supervisor {
            actors: RwLock::new(HashMap::new()),
            actor_pool: pool,
            name: name.to_string(),
        };
        info!("Instantiating new {:?}", supervisor);

        supervisor
    }

    pub fn join(&self, pid : Uuid) -> Result<()> {
        let mut actors = try!(self.actors.write());
        match actors.remove(&pid) {
            Some(actor) => {
                Ok(try!(actor.join_handle.join().unwrap()))
            },
            None => Err(WashedUpError::InvalidPid(pid)),
        }
    }

    pub fn send_message(&self, pid: Uuid, message: Json) -> Result<()> {
        let actors = try!(self.actors.read());
        match actors.get(&pid) {
            Some(actor) => {
                Ok(try!(actor.mailbox.send(message)))
            },
            None => Err(WashedUpError::InvalidPid(pid)),
        }
    }

    pub fn spawn<F>(&self, actor_name: &str, body: F) -> Result<Uuid>
        where F : 'static + Sync + Send + Fn(Receiver<Json>) -> () {
        let pid = Uuid::new_v4();
        let (mailbox_sender, mailbox_receiver) = channel();
        let arc_body = Arc::new(Mutex::new(body));

        let mut actors = try!(self.actors.write());
        let actor_handle = self.actor_pool.spawn(move || {
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
