extern crate rustc_serialize;
extern crate uuid;

use std::collections::HashMap;
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
        where F : Fn(Receiver<Json>) -> () {
        let pid = Uuid::new_v4();
        let (mailbox_sender, mailbox_receiver) = channel();

        self.spawned_actors.insert(pid, mailbox_sender);
        body(mailbox_receiver);
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
            |r| { thread::spawn(move || { r.recv().unwrap() }); () }
        ).unwrap();

        supervisor.send_message(pid, "{}".to_json()).unwrap();
    }

    // #[test]
    // fn the_body_actor_callback_is_executed() {
    //     let mut supervisor = Supervisor::new("folks");
    //     let json_msg = "{\"hi\": \"friend\"}".to_json();
    //     let json_clone = json_msg.clone();
    //     let (send_chan, recv_chan) = channel();
    //     let pid: Uuid = supervisor.spawn_actor(
    //         "Bob",
    //         |receiver| { send_chan.send(receiver.recv().unwrap().to_json()); () }
    //     ).unwrap();
    //
    //     supervisor.send_message(pid, json_msg).unwrap();
    //
    //     thread::sleep_ms(100);
    //     assert_eq!(recv_chan.try_recv().unwrap(), json_clone);
    // }
}
