use std::thread;
use std::sync::{Arc, atomic::AtomicBool, atomic::Ordering};
use std::convert::TryInto;
use zbus::{dbus_interface, fdo};

use rust_raspi_led_strip::LEDStrip;
use rust_raspi_led_strip::TalkingLED;

// TODO: Interface with actual LED controls
struct BlinkService<T>
    where
        T: LEDStrip,
{
    leds: T
}

impl<T: LEDStrip> BlinkService<T> {
    pub fn new(mut leds: T) -> Self {
        Self {
            leds: leds
        }
    }
}

#[dbus_interface(name = "org.zbus.BlinkService1")]
impl<T: 'static + LEDStrip> BlinkService<T> {
    // TODO: Expose LED functions
    fn set_state(&mut self, state: u8) -> () {
        println!("Received set_state: {}", state);
        //self.state = state;
    }
    fn get_state(&mut self) -> u8 {
        println!("Received get_state");
        //self.state
        0
    }
}

pub struct BlinkDbusService
{
    handle: Option<thread::JoinHandle<()>>,
    alive: Arc<AtomicBool>,
}

impl BlinkDbusService
{
    pub fn new() -> Self {
        Self {
            handle: None,
            alive: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn start(&mut self) -> () {
        let alive = self.alive.clone();

        self.handle = Some(std::thread::spawn(move || {
            let connection = zbus::Connection::new_session().unwrap();

            fdo::DBusProxy::new(&connection).unwrap().request_name(
                "org.zbus.BlinkService",
                fdo::RequestNameFlags::ReplaceExisting.into()
            ).unwrap();

            let mut object_server = zbus::ObjectServer::new(&connection);
            let service = BlinkService::<TalkingLED>::new(TalkingLED::new());

            object_server.at(&"/org/zbus/BlinkService".try_into().unwrap(), service).unwrap();

            alive.store(true, Ordering::SeqCst);
            while alive.load(Ordering::SeqCst) {
                if let Err(err) = object_server.try_handle_next() {
                    eprintln!("{}", err);
                }
            }
        }));
    }

    pub fn stop(&mut self) -> () {
        self.alive.store(false, Ordering::SeqCst);
        // TODO: Send request to Server to iniialize thread termination. Since the Server is
        //       blocking :(

        self.handle.take().expect("Called on a non-running thread.")
                   .join().expect("Could not join spawened thread.");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time;

    #[test]
    fn server_lifetime() {
        let mut srv = BlinkDbusService::new();
        srv.start();
        thread::sleep(time::Duration::from_millis(5000));
        srv.stop();
    }
}
