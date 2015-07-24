extern crate rustc_serialize;
extern crate chrono;
extern crate uuid;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::sync::mpsc::{channel, Receiver, Sender, SendError};
use std::thread::{Builder, JoinHandle};

use rustc_serialize::json::{Json, ToJson};
use uuid::Uuid;

pub struct Supervisor {
    name: String,
    actor_mailboxes: HashMap<Uuid, Sender<Json>>,
    actor_handles: HashMap<Uuid, JoinHandle<()>>,
}

impl Supervisor {
    fn new(name: &str) -> Supervisor {
        Supervisor {
            name: name.to_string(),
            actor_mailboxes: HashMap::new(),
            actor_handles: HashMap::new(),
        }
    }

    fn join(&mut self, pid : Uuid) -> Result<(), String> {
        match self.actor_handles.remove(&pid) {
            Some(actor_handle) => {
                self.actor_mailboxes.remove(&pid).unwrap();

                match actor_handle.join() {
                    Ok(_) => Ok(()),
                    Err(e) => Err("you really messed up".to_string()),
                }
            },
            None => Err(format!("PID {} does not map to a spawned actor", pid.to_string())),
        }
    }

    fn send_message(&self, pid: Uuid, message: Json) -> Result<(), String> {
        match self.actor_mailboxes.get(&pid) {
            Some(actor_mailbox) => {
                match actor_mailbox.send(message) {
                    Ok(_) => Ok(()),
                    Err(e) => { Err(e.to_string()) },
                }
            },
            None => Err(format!("PID {} does not map to a spawned actor", pid.to_string())),
        }
    }

    fn spawn<F>(&mut self, actor_name: &str, body: F) -> Result<Uuid, &str>
        where F : 'static + Send + Fn(Receiver<Json>) -> () {
        let pid = Uuid::new_v4();
        let (mailbox_sender, mailbox_receiver) = channel();
        let arc_body = Arc::new(Mutex::new(body));

        self.actor_mailboxes.insert(pid, mailbox_sender);
        let actor_handle_result = Builder::new().name(pid.to_string()).spawn(move || {
            arc_body.lock().unwrap()(mailbox_receiver);
        });

        match actor_handle_result {
            Ok(actor_handle) => self.actor_handles.insert(pid, actor_handle),
            Err(e) => return Err("boom"), // sigh
        };

        return Ok(pid);
    }
}

#[cfg(test)]
mod tests {
    use std::fs::{remove_file, File};
    use std::io::prelude::*;
    use std::sync::mpsc::{channel, Receiver, Sender};
    use std::thread;
    use chrono::*;
    use rustc_serialize::json::{Json, ToJson};
    use uuid::Uuid;
    use super::*;

    // spawn tests
    #[test]
    fn the_body_actor_callback_is_executed() {
        let mut supervisor = Supervisor::new("folks");
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

    // send_message tests
    #[test]
    fn error_is_returned_if_message_sent_for_pid_that_does_not_exist() {
        let mut supervisor = Supervisor::new("folks");
        let pid = Uuid::new_v4();

        let send_result = supervisor.send_message(pid, "{}".to_json());

        match send_result {
            Ok(_) => panic!("Message sent to non-existent PID"),
            Err(e) => assert_eq!(e, format!("PID {} does not map to a spawned actor", pid.to_string())),
        }
    }

    #[test]
    fn pid_can_be_used_to_send_message() {
        let mut supervisor = Supervisor::new("folks");
        let pid: Uuid = supervisor.spawn(
            "Bob",
            |r| { r.recv().unwrap(); () }
        ).unwrap();

        supervisor.send_message(pid, "{}".to_json()).unwrap();
    }

    // join tests
    #[test]
    fn pid_can_be_used_to_join_actor() {
        let mut supervisor = Supervisor::new("folks");
        let start_time = UTC::now();
        let pid: Uuid = supervisor.spawn(
            "Bob",
            |r| { thread::sleep_ms(1000); () }
        ).unwrap();
        assert!((start_time + Duration::milliseconds(1000)) > UTC::now());

        supervisor.join(pid).unwrap();

        assert!((start_time + Duration::milliseconds(1000)) < UTC::now());
    }

    #[test]
    fn joining_actor_makes_it_unreachable() {
        let mut supervisor = Supervisor::new("folks");
        let pid: Uuid = supervisor.spawn(
            "Bob",
            |r| { () }
        ).unwrap();
        supervisor.join(pid).unwrap();

        let send_result = supervisor.send_message(pid, "{}".to_json());

        match send_result {
            Ok(_) => panic!("Message sent to non-existent PID"),
            Err(e) => assert_eq!(e, format!("PID {} does not map to a spawned actor", pid.to_string())),
        }
    }

    #[test]
    fn error_is_returned_if_joining_actor_that_does_not_exist() {
        let mut supervisor = Supervisor::new("folks");
        let pid = Uuid::new_v4();

        let join_result = supervisor.join(pid);

        match join_result {
            Ok(_) => panic!("Actor joined for non-existent PID"),
            Err(e) => assert_eq!(e, format!("PID {} does not map to a spawned actor", pid.to_string())),
        }
    }
}
