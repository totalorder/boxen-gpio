use rppal::gpio::{Gpio, Trigger, Level, InputPin, OutputPin};
use std::error::Error;
use std::thread;
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};
use std::time::{Duration, Instant};
use std::collections::HashMap;
use std::collections::LinkedList;
use std::ops::{Add, Sub};
use std::process::exit;
use boxen_gpio::IO;

fn main() {
    println!("Setting up button listeners");
    let mut io = IO::create(Duration::from_millis(50));
    let mut led = io.create_led(24, 23);
    led.set_off();
    let yellow_button = io.create_button(27);
    let green_button = io.create_button(17);

    let rx = io.listen();
    println!("Listening for button events...");
    for (pin, pressed) in rx.iter() {
        println!("Button {} {}", pin, if pressed { "pressed" } else { "released" });
        if pressed {
            match pin {
                pin if pin == yellow_button.pin() => led.set_yellow(),
                pin if pin == green_button.pin() => led.set_green(),
                _ => panic!("Received button press for unknown button")
            }
        } else {
            led.set_off();
        }
    }
}
