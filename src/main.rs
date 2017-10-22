#[macro_use]
extern crate arrayref;
#[macro_use]
extern crate error_chain;

extern crate serial;

use std::io;
use std::time::Duration;

use serial::core::BaudRate;
use std::iter::Iterator;
use serial::prelude::*;

// Keeping everything in one file for now for my sanity.
mod errors {
    extern crate serial;

    // Create the Error, ErrorKind, ResultExt, and Result types
    error_chain!{
        errors {
            InvalidHeaderError(h: u8, t: u8) {
                description("invalid header or tail bytes"),
                display("Invalid header tail bytes. Header: {:x}, tail: {:x}", h, t)
            }
            ChecksumError(expected: u8, actual: u8) {
                description("invalid checksum"),
                display("Invalid checksum. Expected: {:x}, actual: {:x}", expected, actual)
            }
            SerialReconfigureError(err: serial::Error) {
                description("failed serial reconfiguration"),
                display("Failed to reconfigure serial port: {}", err)
            }
        }
    }
}

use errors::*;

fn main() {
    // TODO: Read from env::arg_os().
    let mut port = serial::open("/dev/ttyUSB0").expect("Cannot open ttyUSB0.");
    let mut nova = Nova::new(&mut port);

    nova.interact().unwrap();
}

struct Nova<'a> {
    port: &'a mut SerialPort,
}

impl<'a> Nova<'a> {
    pub fn new<T: SerialPort>(port: &mut T) -> Nova {
        Nova {
            port: port
        }
    }

    pub fn interact(self: &mut Self) -> io::Result<()> {
        match self.port.reconfigure(&|settings| {
            settings.set_baud_rate(BaudRate::Baud9600).unwrap();
            settings.set_flow_control(serial::FlowControl::FlowNone);
            Ok(())
        }) {
            // TODO: Figure out how to do this the right way.
            Err(e) => return Err(e.into()),
            Ok(()) => ()
        }

        // Default interval between messages is 1s, so 1000ms is too low.
        try!(self.port.set_timeout(Duration::from_millis(2000)));

        loop {
            let bytes = read_bytes(self.port).unwrap();
            println!("bytes: {:?}", bytes);
            let msg = parse_message(&bytes);
            println!("msg: {:?}", msg);
        }
    }
}

fn read_bytes(port: &mut SerialPort) -> io::Result<[u8; 10]> {
    let mut buf = [0_u8; 10];
    try!(port.read_exact(buf.as_mut()));
    Ok(buf)
}

#[derive(Debug)]
struct RawResponse<'a> {
    header: u8,  // Always 0xAA
    command: u8, // 0xC0 in active mode, 0xC5 as reply
    data: &'a [u8; 6],
    checksum: u8, // Sum of data bytes, rolling over
    tail: u8,     // always 0xAB
}

#[derive(Debug)]
struct Message {
    pm25: f32,
    pm10: f32,
}

fn crc(buf: &[u8; 6]) -> u8 {
    let s: u16 = buf.iter().map(|x| *x as u16).sum();
    s as u8
}

fn check_crc(rsp: &RawResponse) -> Result<()> {
    let actual_crc = crc(&rsp.data);
    if actual_crc == rsp.checksum {
        Ok(())
    } else {
        bail!(ErrorKind::ChecksumError(actual_crc, rsp.checksum))
    }
}

fn check_header(rsp: &RawResponse) -> Result<()> {
    if rsp.header == 0xAA && rsp.command == 0xC0 && rsp.tail == 0xAB {
        Ok(())
    } else {
        bail!(ErrorKind::InvalidHeaderError(rsp.header, rsp.tail))
    }
}

fn check_response(rsp: &RawResponse) -> Result<()> {
    check_header(rsp).and_then(|_| check_crc(rsp))
}

/// Turns a response into a struct, without making any
/// guarantees about its validity.
fn read_response(buf: &[u8; 10]) -> RawResponse {
    RawResponse {
        header: buf[0],
        command: buf[1],
        data: array_ref!(buf, 2, 6),
        checksum: buf[8],
        tail: buf[9],
    }
}

fn parse_message(buf: &[u8; 10]) -> Result<Message> {
    let rsp = read_response(buf);
    try!(check_header(&rsp));
    try!(check_response(&rsp));

    // Extract PM values. Formula from the spec:
    //   PM2.5 value: PM2.5 (ug/m3) = ((PM2.5 High byte *256) + PM2.5 low byte) / 10
    //   PM10 value: PM10 (ug/m3) = ((PM10 high byte*256) + PM10 low byte) / 10
    Ok(Message {
        pm25: ((buf[2] as u16) | ((buf[3] as u16) << 8)) as f32 / 10.0,
        pm10: ((buf[4] as u16) | ((buf[5] as u16) << 8)) as f32 / 10.0,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_crc_check_fails() {
        let rsp = read_response(&[192, 14, 1, 21, 0, 64, 147, 246, 171, 170]);
        assert!(check_crc(&rsp).is_err());
    }

    #[test]
    fn test_crc_check_passes() {
        let rsp = read_response(&[170, 192, 13, 0, 21, 0, 64, 147, 245, 171]);
        assert!(check_crc(&rsp).is_ok());
    }

    #[test]
    fn test_header_valid() {
        let rsp = read_response(&[170, 192, 13, 0, 21, 0, 64, 147, 245, 171]);
        check_header(&rsp).unwrap();
        assert!(check_header(&rsp).is_ok());
    }

    #[test]
    fn test_header_check_fails() {
        let rsp = read_response(&[192, 14, 1, 21, 0, 64, 147, 246, 171, 170]);
        assert!(check_header(&rsp).is_err());
    }

    #[test]
    fn test_parses_message() {
        let bytes = &[170, 192, 13, 0, 21, 0, 64, 147, 245, 171];
        let msg = parse_message(bytes);
        assert!(msg.is_ok());
    }

    #[test]
    fn test_parses_values_in_active_mode() {
        let bytes = &[170, 192, 13, 0, 21, 0, 64, 147, 245, 171];
        let msg = parse_message(bytes).unwrap();
        assert_eq!(msg.pm25, 1.3);
        assert_eq!(msg.pm10, 2.1);
    }
}
