extern crate serial;

use std::io;
use std::time::Duration;

use serial::core::BaudRate;
use std::iter::Iterator;
use serial::prelude::*;

fn main() {
    // TODO: Read from env::arg_os().
    let mut port = serial::open("/dev/ttyUSB0").expect("Cannot open ttyUSB0.");
    interact(&mut port).unwrap();
}

fn read_bytes<T: SerialPort>(port: &mut T) -> io::Result<[u8; 10]> {
    let mut buf = [0_u8; 10];
    try!(port.read_exact(buf.as_mut()));
    Ok(buf)
}

#[derive(Debug)]
struct Message {
    pm25: f32,
    pm10: f32,
}

fn crc(buf: &[u8; 10]) -> u8 {
    let s: u16 = buf[2..8].iter().map(|x| *x as u16).sum();
    s as u8
}

fn check_crc(buf: &[u8; 10]) -> bool {
    let cur_crc = crc(buf) as u8;
    let nova_crc = buf[8] as u8;
    cur_crc == nova_crc
}

fn check_header(buf: &[u8; 10]) -> bool {
    buf[0] == 0xAA && buf[1] == 0xC0
}

fn check_message(buf: &[u8; 10]) -> bool {
    check_header(buf) && check_crc(buf)
}

fn parse_message(buf: &[u8; 10]) -> Option<()> {
    if check_message(buf) {
        return None;
    }

    return Some(())
}

fn interact<T: SerialPort>(port: &mut T) -> io::Result<()> {
    port.reconfigure(&|settings| {
        settings.set_baud_rate(BaudRate::Baud9600).unwrap();
        Ok(())
    }).unwrap();

    // Default interval between messages is 1s, so 1000ms is too low.
    try!(port.set_timeout(Duration::from_millis(2000)));

    loop {
        let bytes = read_bytes(port).unwrap();
        let msg = parse_message(&bytes);
        println!("byte: {:?}", msg);
    }

    Ok(())
}

#[test]
fn test_crc_check_passes() {
    let buf: &[u8; 10] = &[170, 192, 13, 0, 21, 0, 64, 147, 245, 171];
    assert_eq!(true, check_crc(buf));
}

#[test]
fn test_crc_check_fails() {
    let buf: &[u8; 10] = &[192, 14, 1, 21, 0, 64, 147, 246, 171, 170];
    assert_eq!(false, check_crc(buf));
}

#[test]
fn test_header_check_fails() {
    let buf: &[u8; 10] = &[192, 14, 1, 21, 0, 64, 147, 246, 171, 170];
    assert_eq!(false, check_header(buf));
}
