use std::thread;
use std::marker::PhantomData;
use std::sync::{Arc, atomic::AtomicBool, atomic::Ordering};
use std::convert::TryInto;
use zbus::{dbus_interface, fdo};

use rust_raspi_led_strip::LEDStrip;

// TODO: Interface with actual LED controls
struct BlinkService<T>
    where
        T: LEDStrip,
{
    leds: T
}

impl<T: LEDStrip> BlinkService<T> {
    pub fn new(leds: T) -> Self {
        Self {
            leds: leds
        }
    }
}

// Expose calls as DBus Interface 
#[dbus_interface(name = "org.zbus.BlinkService1")]
impl<T: 'static + LEDStrip> BlinkService<T> {
    fn set_all(&mut self, r: u8, g: u8, b: u8, brightness: f32)
    {
        self.leds.set_all(r, g, b, brightness)
    }

    fn set_pixel(&mut self, x: u32, r: u8, g: u8, b: u8, brightness: f32)
    {
        self.leds.set_pixel(x as usize, r, g, b, brightness)
    }

    fn set_brightness(&mut self, brightness: f32)
    {
        self.leds.set_brightness(brightness)
    }

    fn clear(&mut self)
    {
        self.leds.clear()
    }

    fn show(&mut self)
    {
        self.leds.show().unwrap();
    }
}

pub struct BlinkDbusService<T>
{
    handle: Option<thread::JoinHandle<()>>,
    alive: Arc<AtomicBool>,
    ignore: PhantomData<T>,
}

impl<T: 'static + LEDStrip> BlinkDbusService<T> {
    pub fn new() -> Self {
        Self {
            handle: None,
            alive: Arc::new(AtomicBool::new(false)),
            ignore: PhantomData,
        }
    }

    pub fn start(&mut self, leds: T)
        where
            T: Send
    {
        let alive = self.alive.clone();

        self.handle = Some(std::thread::spawn(move || {
            let connection = zbus::Connection::new_session().unwrap();

            fdo::DBusProxy::new(&connection).unwrap().request_name(
                "org.zbus.BlinkService",
                fdo::RequestNameFlags::ReplaceExisting.into()
            ).unwrap();

            let mut object_server = zbus::ObjectServer::new(&connection);
            let service = BlinkService::<T>::new(leds);

            object_server.at(&"/org/zbus/BlinkService".try_into().unwrap(), service).unwrap();

            alive.store(true, Ordering::SeqCst);
            while alive.load(Ordering::SeqCst) {
                if let Err(err) = object_server.try_handle_next() {
                    eprintln!("{}", err);
                }
            }
        }));
    }

    pub fn stop(&mut self) {
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
    use rust_raspi_led_strip::TalkingLED;

    #[test]
    fn server_lifetime() {
        let mut srv = BlinkDbusService::<TalkingLED>::new();
        srv.start(TalkingLED::new());
        thread::sleep(time::Duration::from_millis(60000));
        srv.stop();
    }
}
