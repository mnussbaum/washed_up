use std::collections::HashMap;
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

use coroutine::builder::Builder;
use rustc_serialize::json::Json;
use uuid::Uuid;

pub use actor::Actor;

pub struct Supervisor {
    actors: RwLock<HashMap<Uuid, Actor>>,
    pub name: String,
}

impl fmt::Debug for Supervisor {
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

    pub fn join(&self, pid : Uuid) -> Result<(), String> {
        match self.actors.write() {
            Ok(mut actors) => {
                match actors.remove(&pid) {
                    Some(actor) => {
                        let actor_descriptor = format!("{:?}", actor);

                        match actor.join_handle.join() {
                            Ok(_) => Ok(()),
                            Err(e) => {
                                error!("Error joining {:?}: {:?}", actor_descriptor, e);
                                Err(format!("Error joining {:?}: {:?}", actor_descriptor, e))
                            }
                        }
                    },
                    None => Err(format!("PID {:?} does not map to a spawned actor", pid.to_string())),
                }
            },
            Err(e) => {
                error!("Error obtaining actor write lock for {:?}: {:?}", self, e);
                Err(e.to_string())
            },
        }
    }

    pub fn send_message(&self, pid: Uuid, message: Json) -> Result<(), String> {
        match self.actors.read() {
            Ok(actors) => {
                match actors.get(&pid) {
                    Some(actor) => {
                        match actor.mailbox.send(message) {
                            Ok(_) => {
                                match actor.join_handle.resume() {
                                    Ok(_) => Ok(()),
                                    Err(e) => {
                                        error!("Error resuming {:?}: {:?}", actor, e);
                                        Err(format!("{:?}", e))
                                    }
                                }
                            },
                            Err(e) => {
                                error!("Error sending message to {:?}: {:?}", actor, e);
                                Err(e.to_string())
                            },
                        }
                    },
                    None => Err(format!("PID {:?} does not map to a spawned actor", pid.to_string())),
                }
            },
            Err(e) => {
                error!("Error obtaining actor read lock for {:?}: {:?}", self, e);
                Err(e.to_string())
            },
        }
    }

    pub fn spawn<F>(&self, actor_name: &str, body: F) -> Result<Uuid, String>
        where F : 'static + Send + Fn(Receiver<Json>) -> () {
        let pid = Uuid::new_v4();
        let (mailbox_sender, mailbox_receiver) = channel();
        let arc_body = Arc::new(Mutex::new(body));

        match self.actors.write() {
            Ok(mut actors) => {
                let actor_handle = Builder::new().name(pid.to_string()).spawn(move || {
                    arc_body.lock().unwrap()(mailbox_receiver);
                });

                actors.insert(pid, Actor::new(
                    actor_handle,
                    mailbox_sender,
                    actor_name.to_string(),
                    pid,
                ));

                Ok(pid)
            },
            Err(e) => {
                error!("Error obtaining actor write lock for {:?}: {:?}", self, e);
                Err(e.to_string())
            },
        }
    }
}

