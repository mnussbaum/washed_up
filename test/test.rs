extern crate rustc_serialize;
extern crate time;
extern crate uuid;

extern crate washed_up;

use std::error::Error;
use std::fs::{
    remove_file,
    File,
};
use std::io::prelude::*;
use std::time::Duration as StdDuration;
use std::thread;
use time::{
    now,
    Duration
};
use rustc_serialize::json::{
    Json,
    ToJson,
};
use uuid::Uuid;

use washed_up::Result as WashedUpResult;
use washed_up::supervisor::{
    Supervisor,
};

// spawn tests
#[test]
fn the_body_actor_callback_is_executed() {
    let mut supervisor = Supervisor::new("folks");
    let json_msg = "{\"hi\": \"friend\"}".to_json();
    let json_clone = json_msg.clone();
    let pid_bob: Uuid = supervisor.spawn(
        "Bob",
        |mut io_handle, receiver| {
            let mut message_output_file = File::create("/tmp/bob-test.json").unwrap();
            let message = io_handle.recv(&receiver).unwrap().to_string();
            message_output_file.write_all(message.as_bytes()).unwrap();
            ()
        }
    ).unwrap();

    supervisor.send_message(pid_bob, json_msg).unwrap();

    thread::sleep(StdDuration::from_millis(1001));
    let mut message_output_file = File::open("/tmp/bob-test.json").unwrap();
    let mut message_output = String::new();
    message_output_file.read_to_string(&mut message_output).unwrap();
    remove_file("/tmp/bob-test.json").unwrap();

    let actual_json = Json::from_str(&message_output).unwrap();
    assert_eq!(json_clone, actual_json);
}

#[test]
fn actor_body_is_executed_for_every_message() {
    let mut supervisor = Supervisor::new("folks");
    let json_msg = "{\"hi\": \"friend\"}".to_json();
    let pid_steve: Uuid = supervisor.spawn(
        "steve",
        |mut io_handle, receiver| {
            for _ in 0..2 {
                let mut message_output_file = File::create("/tmp/steve-test.json").unwrap();
                let m = io_handle.recv(&receiver).unwrap().to_string();
                message_output_file.write_all(m.as_bytes()).unwrap();
            }
            ()
        }
    ).unwrap();

    supervisor.send_message(pid_steve, json_msg).unwrap();

    let second_message = "{\"imnot\": \"yourfriend\"}".to_json();
    let second_message_clone = second_message.clone();
    supervisor.send_message(pid_steve, second_message).unwrap();

    thread::sleep(StdDuration::from_millis(1001));
    let mut message_output_file = File::open("/tmp/steve-test.json").unwrap();
    let mut message_output = String::new();
    message_output_file.read_to_string(&mut message_output).unwrap();
    remove_file("/tmp/steve-test.json").unwrap();

    let actual_json = Json::from_str(&message_output).unwrap();
    assert_eq!(second_message_clone, actual_json);
}

// send_message tests
#[test]
fn error_is_returned_if_message_sent_for_pid_that_does_not_exist() {
    let supervisor = Supervisor::new("folks");
    let pid = Uuid::new_v4();

    let send_result = supervisor.send_message(pid, "{}".to_json());

    match send_result {
        Ok(_) => panic!("Message sent to non-existent PID"),
        Err(e) => assert_eq!(e.description(), "PID does not map to a spawned actor"),
    }
}

#[test]
fn pid_can_be_used_to_send_message() {
    let mut supervisor = Supervisor::new("folks");
    let pid: Uuid = supervisor.spawn(
        "Bob",
        |mut i, r| { i.recv(&r).unwrap(); }
    ).unwrap();

    supervisor.send_message(pid, "{Hi:\"friend\"}".to_json()).unwrap();
    supervisor.join(pid).unwrap();
}

// join tests
#[test]
fn pid_can_be_used_to_join_actor() {
    let mut supervisor = Supervisor::new("folks");
    let pid: Uuid = supervisor.spawn(
        "Bob",
        |_, _| { thread::sleep(StdDuration::from_millis(1000)); () }
    ).unwrap();

    let start_time = now();
    assert!((start_time + Duration::milliseconds(1000)) > now());

    supervisor.join(pid).unwrap();

    assert!((start_time + Duration::milliseconds(1000)) < now());
}

#[test]
fn joining_actor_makes_it_unreachable() {
    let mut supervisor = Supervisor::new("folks");
    let pid: Uuid = supervisor.spawn(
        "Bob",
        |_, _| { () }
    ).unwrap();

    supervisor.join(pid).unwrap();

    let send_result = supervisor.send_message(pid, "{}".to_json());

    match send_result {
        Ok(_) => panic!("Message sent to non-existent PID"),
        Err(e) => assert_eq!(e.description(), "PID does not map to a spawned actor"),
    }
}

#[test]
fn error_is_returned_if_joining_actor_that_does_not_exist() {
    let supervisor = Supervisor::new("folks");
    let pid = Uuid::new_v4();

    let join_result: WashedUpResult<()>= supervisor.join(pid);

    match join_result {
        Ok(_) => panic!("Actor joined for non-existent PID"),
        Err(e) => assert_eq!(e.description(), "PID does not map to a spawned actor"),
    }
}

#[test]
fn actors_trap_panics() {
    let mut supervisor = Supervisor::new("folks");
    supervisor.spawn(
        "Bob",
        |_, _| { panic!("boom!"); }
    ).unwrap();
}
