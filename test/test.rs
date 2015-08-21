extern crate coroutine;
extern crate rustc_serialize;
extern crate time;
extern crate uuid;

extern crate washed_up;

use std::fs::{
    remove_file,
    File,
};
use std::io::prelude::*;
use std::thread;
use time::{
    now,
    Duration
};
use coroutine::Coroutine;
use rustc_serialize::json::{
    Json,
    ToJson,
};
use uuid::Uuid;
use washed_up::supervisor::{
    Supervisor,
};

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
            for _ in (0..2) {
                let m = receiver.recv().unwrap().to_string();
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
    let start_time = now();
    let pid: Uuid = supervisor.spawn(
        "Bob",
        |_| { thread::sleep_ms(1000); () }
    ).unwrap();
    assert!((start_time + Duration::milliseconds(1000)) > now());

    supervisor.join(pid).unwrap();

    assert!((start_time + Duration::milliseconds(1000)) < now());
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
