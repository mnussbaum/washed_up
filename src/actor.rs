use std::fmt;

use coroutine::Handle;
use rustc_serialize::json::Json;
use uuid::Uuid;
use std::sync::mpsc::{
    Sender,
};

pub struct Actor {
    pub join_handle: Handle,
    pub mailbox: Sender<Json>,
    pub name: String,
    pub uuid: Uuid,
}

impl Actor {
    pub fn new(
        join_handle: Handle,
        mailbox: Sender<Json>,
        name: String,
        uuid: Uuid,
    ) -> Actor {
        let actor = Actor {
            join_handle: join_handle,
            mailbox: mailbox,
            name: name,
            uuid: uuid,
        };
        info!("Instantiating new {:?}", actor);

        actor
    }
}

impl fmt::Debug for Actor {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Actor {:?} with PID {:?}", self.name, self.uuid)
    }
}
