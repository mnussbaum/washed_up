extern crate rustc_serialize;
extern crate uuid;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::sync::mpsc::{channel, Receiver, Sender, SendError};
use std::thread;

use rustc_serialize::json::{Json, ToJson};
use uuid::Uuid;

pub struct Supervisor {
    name: String,
    spawned_actors: HashMap<Uuid, Sender<Json>>,
}

impl Supervisor {
    fn new(name: &str) -> Supervisor {
        Supervisor {
            name: name.to_string(),
            spawned_actors: HashMap::new(),
        }
    }

    fn spawn_actor<F>(&mut self, actor_name: &str, body: F) -> Result<Uuid, &str>
        where F : 'static + Send + Fn(Receiver<Json>) -> () {
        let pid = Uuid::new_v4();
        let (mailbox_sender, mailbox_receiver) = channel();
        let arc_body = Arc::new(Mutex::new(body));

        self.spawned_actors.insert(pid, mailbox_sender);
        thread::Builder::new().name(pid.to_string()).spawn(move || {
            arc_body.lock().unwrap()(mailbox_receiver);
        });
        return Ok(pid);
    }

    fn send_message(&self, pid: Uuid, message: Json) -> Result<(), String> {
        match self.spawned_actors.get(&pid) {
            Some(actor_mailbox) => {
                match actor_mailbox.send(message) {
                    Ok(_) => Ok(()),
                    Err(e) => { Err(e.to_string()) },
                }
            },
            None => Err(format!("PID {} does not map to a spawned actor", pid.to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs::{remove_file, File};
    use std::io::prelude::*;
    use std::sync::mpsc::{channel, Receiver, Sender};
    use std::thread;
    use rustc_serialize::json::{Json, ToJson};
    use uuid::Uuid;
    use super::*;

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
        let pid: Uuid = supervisor.spawn_actor(
            "Bob",
            |r| { r.recv().unwrap(); () }
        ).unwrap();

        supervisor.send_message(pid, "{}".to_json()).unwrap();
    }

    #[test]
    fn the_body_actor_callback_is_executed() {
        let mut supervisor = Supervisor::new("folks");
        let json_msg = "{\"hi\": \"friend\"}".to_json();
        let json_clone = json_msg.clone();
        let pid_bob: Uuid = supervisor.spawn_actor(
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
}
