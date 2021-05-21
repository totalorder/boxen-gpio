use rppal::gpio::{Gpio, Trigger, InputPin, OutputPin};
use std::thread;
use crossbeam_channel;
use crossbeam_channel::{Receiver,Sender};
use std::time::{Duration, Instant};
use std::collections::HashMap;
use std::collections::LinkedList;
use std::ops::{Add, Sub};
use std::sync::{Arc, Mutex};

struct InnerBlinkingLed {
    output_pin: OutputPin,
    blinking: bool,
}

struct BlinkingLed {
    inner: Arc<Mutex<InnerBlinkingLed>>
}

impl BlinkingLed {
    fn set_high(&mut self) {
        let mut inner = self.inner.lock().unwrap();
        inner.blinking = false;
        inner.output_pin.set_high();
    }

    fn set_low(&mut self) {
        let mut inner = self.inner.lock().unwrap();
        inner.blinking = false;
        inner.output_pin.set_low();
    }

    fn clone(&self) -> BlinkingLed {
        BlinkingLed {
            inner: Arc::clone(&self.inner)
        }
    }
}

pub struct Led {
    yellow: BlinkingLed,
    green: OutputPin,
}

impl Led {
    fn new(yellow: OutputPin, green: OutputPin) -> Led {
        let inner = InnerBlinkingLed {
            output_pin: yellow,
            blinking: false
        };

        Led {
            yellow: BlinkingLed {
                inner: Arc::new(Mutex::new(inner))
            },
            green,
        }
    }

    pub fn set_off(&mut self) {
        self.yellow.set_low();
        self.green.set_low();
    }

    pub fn set_green(&mut self) {
        self.yellow.set_low();
        self.green.set_high();
    }

    pub fn set_yellow(&mut self) {
        self.green.set_low();
        self.yellow.set_high();
    }

    pub fn set_yellow_blink(&mut self) {
        self.green.set_low();

        {
            self.yellow.inner.lock().unwrap().blinking = true;
        }

        let yellow = self.yellow.clone();
        thread::spawn(move || {
            let mut yellow_on = true;
            loop {
                {
                    let mut yellow_inner = yellow.inner.lock().unwrap();
                    println!("Yellow blinking: {}, on: {}", yellow_inner.blinking, yellow_on);
                    if !yellow_inner.blinking {
                        return;
                    }

                    if yellow_on {
                        yellow_inner.output_pin.set_high();
                    } else {
                        yellow_inner.output_pin.set_low();
                    }
                }
                yellow_on = !yellow_on;

                thread::sleep(Duration::from_millis(500));
            }
        });
    }
}

pub struct Button {
    pin: u8,
    initial_state: bool
}

impl Button {
    pub fn pin(&self) -> u8 {
        self.pin
    }

    pub fn initial_state(&self) -> bool {
        self.initial_state
    }
}

pub struct IO {
    buttons: HashMap<u8, InputPin>,
    deadlines: HashMap<u8, Instant>,
    debounce: Duration,
    tx: Sender<(u8, Instant)>,
    rx: Receiver<(u8, Instant)>
}

impl IO {
    pub fn create(debounce: Duration) -> IO {
        let (tx, rx) = crossbeam_channel::unbounded();
        IO {
            buttons: HashMap::new(),
            deadlines: HashMap::new(),
            debounce,
            tx,
            rx
        }
    }

    pub fn create_button(&mut self, pin: u8) -> Button {
        let mut button = Gpio::new()
            .expect("Failed to create Gpio")
            .get(pin)
            .expect("Failed to get pin")
            .into_input_pullup();
        let tx = self.tx.clone();
        let initial_state = button.is_low();
        button
            .set_async_interrupt(
            Trigger::RisingEdge,
            move |_| {
                tx.send((pin, Instant::now())).expect("Failed to send message")
            })
            .expect("Failed to add callback for button1");

        self.buttons.insert(pin, button);

        Button {
            pin,
            initial_state
        }
    }

    pub fn create_led(&self, yellow_pin: u8, green_pin: u8) -> Led {
        let yellow = Gpio::new()
            .expect("Failed to create Gpio")
            .get(yellow_pin)
            .expect("Failed to get pin")
            .into_output();

        let green = Gpio::new()
            .expect("Failed to create Gpio")
            .get(green_pin)
            .expect("Failed to get pin")
            .into_output();

        Led::new(yellow, green)
    }

    pub fn listen(mut self) -> Receiver<(u8, bool)> {
        let (tx, rx) = crossbeam_channel::unbounded();
        thread::spawn(move || {
            loop {
                let now = Instant::now();
                let mut expired_pins: LinkedList<u8> = LinkedList::new();
                for (pin, deadline) in self.deadlines.clone() {
                    let deadline_expired = !deadline.ge(&&mut now.clone());
                    if deadline_expired {
                        let button = self.buttons.get(&pin)
                            .expect("Deadline expired for invalid button");
                        let is_low = button.is_low();
                        expired_pins.push_back(pin);
                        tx.send((pin, is_low)).unwrap();
                    };
                }

                for expired_pin in expired_pins {
                    self.deadlines.remove(&expired_pin);
                }

                for (pin, instant) in self.rx.try_iter() {
                    let deadline = instant.add(self.debounce);
                    self.deadlines.insert(pin, deadline);
                }
                let mut closest_deadline = Instant::now().add(self.debounce);

                for (_, deadline) in self.deadlines.clone() {
                    if deadline.lt(&closest_deadline) {
                        closest_deadline = deadline;
                    }
                }

                let now = Instant::now();
                if now.lt(&closest_deadline) {
                    let sleep_duration = closest_deadline.sub(now);
                    thread::sleep(sleep_duration);
                }
            }
        });
        rx
    }
}

pub fn public_function() {
    println!("called rary's `public_function()`");
}
