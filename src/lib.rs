#[macro_use]
extern crate chrono;
extern crate coroutine;
#[macro_use]
extern crate log;
extern crate rustc_serialize;
extern crate uuid;

use std::collections::HashMap;
use std::fmt;
use std::sync::{Arc, Mutex, RwLock};
use std::sync::mpsc::{channel, Receiver, Sender};

use coroutine::builder::Builder;
use coroutine::Handle;
use rustc_serialize::json::{Json};
use uuid::Uuid;

pub struct Actor {
    join_handle: Handle,
    mailbox: Sender<Json>,
    name: String,
    uuid: Uuid,
}

impl Actor {
    fn new(
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

#[cfg(test)]
mod tests {
    use std::fs::{remove_file, File};
    use std::io::prelude::*;
    use std::thread;
    use chrono::*;
    use coroutine::Coroutine;
    use rustc_serialize::json::{Json, ToJson};
    use uuid::Uuid;
    use super::*;

    // spawn tests
    #[test]
    fn the_body_actor_callback_is_executed() {
        let supervisor = Supervisor::new("folks");
        let json_msg = "{\"hi\": \"friend\"}".to_json();
        let json_clone = json_msg.clone();
        let pid_bob: Uuid = supervisor.spawn(
            "Bob",
            |receiver| {
                let mut message_output_file = File::create("/tmp/bob-test.json").unwrap();
                message_output_file.write_all(receiver.recv().unwrap().to_string().as_bytes()).unwrap();
                ()
            }
        ).unwrap();

        supervisor.send_message(pid_bob, json_msg).unwrap();

        thread::sleep_ms(1001);
        let mut message_output_file = File::open("/tmp/bob-test.json").unwrap();
        let mut message_output = String::new();
        message_output_file.read_to_string(&mut message_output).unwrap();
        remove_file("/tmp/bob-test.json").unwrap();

        let actual_json = Json::from_str(&message_output).unwrap();
        assert_eq!(json_clone, actual_json);
    }

    #[test]
    fn actor_body_is_executed_for_every_message() {
        let supervisor = Supervisor::new("folks");
        let json_msg = "{\"hi\": \"friend\"}".to_json();
        let json_clone = json_msg.clone();
        let pid_steve: Uuid = supervisor.spawn(
            "steve",
            |receiver| {
                for i in (0..2) {
                    let m  =receiver.recv().unwrap().to_string();
                    let mut message_output_file = File::create("/tmp/steve-test.json").unwrap();
                    message_output_file.write_all(m.as_bytes()).unwrap();
                    Coroutine::sched();
                }
                ()
            }
        ).unwrap();

        supervisor.send_message(pid_steve, json_msg).unwrap();

        thread::sleep_ms(1001);
        let mut message_output_file = File::open("/tmp/steve-test.json").unwrap();
        let mut message_output = String::new();
        message_output_file.read_to_string(&mut message_output).unwrap();
        remove_file("/tmp/steve-test.json").unwrap();

        let actual_json = Json::from_str(&message_output).unwrap();
        assert_eq!(json_clone, actual_json);

        let json_msg2 = "{\"imnot\": \"yourfriend\"}".to_json();
        let json_clone2 = json_msg2.clone();
        supervisor.send_message(pid_steve, json_msg2).unwrap();

        thread::sleep_ms(1001);
        let mut message_output_file = File::open("/tmp/steve-test.json").unwrap();
        let mut message_output = String::new();
        message_output_file.read_to_string(&mut message_output).unwrap();
        remove_file("/tmp/steve-test.json").unwrap();

        let actual_json = Json::from_str(&message_output).unwrap();
        assert_eq!(json_clone2, actual_json);
    }

    // send_message tests
    #[test]
    fn error_is_returned_if_message_sent_for_pid_that_does_not_exist() {
        let supervisor = Supervisor::new("folks");
        let pid = Uuid::new_v4();

        let send_result = supervisor.send_message(pid, "{}".to_json());

        match send_result {
            Ok(_) => panic!("Message sent to non-existent PID"),
            Err(e) => assert_eq!(e, format!("PID {:?} does not map to a spawned actor", pid.to_string())),
        }
    }

    #[test]
    fn pid_can_be_used_to_send_message() {
        let supervisor = Supervisor::new("folks");
        let pid: Uuid = supervisor.spawn(
            "Bob",
            |r| { r.recv().unwrap(); () }
        ).unwrap();

        supervisor.send_message(pid, "{:?}".to_json()).unwrap();
    }

    // join tests
    #[test]
    fn pid_can_be_used_to_join_actor() {
        let supervisor = Supervisor::new("folks");
        let start_time = UTC::now();
        let pid: Uuid = supervisor.spawn(
            "Bob",
            |_| { thread::sleep_ms(1000); () }
        ).unwrap();
        assert!((start_time + Duration::milliseconds(1000)) > UTC::now());

        supervisor.join(pid).unwrap();

        assert!((start_time + Duration::milliseconds(1000)) < UTC::now());
    }

    #[test]
    fn joining_actor_makes_it_unreachable() {
        let supervisor = Supervisor::new("folks");
        let pid: Uuid = supervisor.spawn(
            "Bob",
            |_| { () }
        ).unwrap();
        supervisor.join(pid).unwrap();

        let send_result = supervisor.send_message(pid, "{}".to_json());

        match send_result {
            Ok(_) => panic!("Message sent to non-existent PID"),
            Err(e) => assert_eq!(e, format!("PID {:?} does not map to a spawned actor", pid.to_string())),
        }
    }

    #[test]
    fn error_is_returned_if_joining_actor_that_does_not_exist() {
        let supervisor = Supervisor::new("folks");
        let pid = Uuid::new_v4();

        let join_result = supervisor.join(pid);

        match join_result {
            Ok(_) => panic!("Actor joined for non-existent PID"),
            Err(e) => assert_eq!(e, format!("PID {:?} does not map to a spawned actor", pid.to_string())),
        }
    }
}
