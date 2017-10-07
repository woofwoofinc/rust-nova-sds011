extern crate serial;

use std::io;
use std::time::Duration;

use serial::core::BaudRate;
use serial::prelude::*;

fn main() {
    // TODO: Read from env::arg_os().
    let mut port = serial::open("/dev/ttyUSB0").expect("Cannot open ttyUSB0.");
    interact(&mut port).unwrap();
}

fn read_byte<T: SerialPort>(port: &mut T) -> io::Result<u8> {
    let mut buf = vec![0_u8; 1];
    try!(port.read_exact(buf.as_mut()));
    Ok(buf[0])
}

fn interact<T: SerialPort>(port: &mut T) -> io::Result<()> {
    port.reconfigure(&|settings| {
        settings.set_baud_rate(BaudRate::Baud9600).unwrap();
        Ok(())
    }).unwrap();

    // Default interval between messages is 1s, so 1000ms is too low.
    try!(port.set_timeout(Duration::from_millis(2000)));

    loop {
        let byte = read_byte(port).unwrap();
        println!("byte: {}", byte);
    }

    Ok(())
}
