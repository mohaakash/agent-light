use std::io::Read;
use std::sync::Mutex;
use std::thread;
use std::time::Duration;

use serde::Serialize;

use crate::core::DisplayState;

const BAUD_RATE: u32 = 115_200;
const SERIAL_TIMEOUT: Duration = Duration::from_millis(300);
const HANDSHAKE_ATTEMPTS: usize = 10;
const HANDSHAKE_RETRY_DELAY: Duration = Duration::from_millis(200);
const MAX_RESPONSE_LENGTH: usize = 96;
const SERIAL_DEVICE_ID: &str = "AGENTLIGHT:SERIAL:1";
const LEGACY_ESP8266_DEVICE_ID: &str = "AGENTLIGHT:ESP8266:1";

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct SerialPortOption {
    pub port_name: String,
    pub description: Option<String>,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct HardwareStatus {
    pub connected: bool,
    pub port_name: Option<String>,
    pub device_id: Option<String>,
    pub error: Option<String>,
}

impl Default for HardwareStatus {
    fn default() -> Self {
        Self {
            connected: false,
            port_name: None,
            device_id: None,
            error: None,
        }
    }
}

#[derive(Default)]
struct Connection {
    port: Option<Box<dyn serialport::SerialPort>>,
    status: HardwareStatus,
}

#[derive(Default)]
pub struct HardwareOutput {
    connection: Mutex<Connection>,
}

pub fn available_ports() -> Result<Vec<SerialPortOption>, String> {
    serialport::available_ports()
        .map(|ports| {
            ports
                .into_iter()
                .map(|port| {
                    let description = match port.port_type {
                        serialport::SerialPortType::UsbPort(info) => {
                            let parts = [info.manufacturer, info.product]
                                .into_iter()
                                .flatten()
                                .collect::<Vec<_>>();
                            (!parts.is_empty()).then(|| parts.join(" "))
                        }
                        _ => None,
                    };
                    SerialPortOption {
                        port_name: port.port_name,
                        description,
                    }
                })
                .collect()
        })
        .map_err(|error| format!("failed to enumerate serial ports: {error}"))
}

impl HardwareOutput {
    pub fn status(&self) -> Result<HardwareStatus, String> {
        self.connection
            .lock()
            .map(|connection| connection.status.clone())
            .map_err(|_| "hardware state lock is unavailable".to_string())
    }

    pub fn connect(&self, port_name: &str, state: DisplayState) -> Result<HardwareStatus, String> {
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| "hardware state lock is unavailable".to_string())?;
        connection.port = None;
        connection.status = HardwareStatus::default();

        let known_port = available_ports()?
            .into_iter()
            .any(|candidate| candidate.port_name == port_name);
        if !known_port {
            connection.status.error = Some("serial port is no longer available".to_string());
            return Err("serial port is no longer available".to_string());
        }

        let mut port = serialport::new(port_name, BAUD_RATE)
            .timeout(SERIAL_TIMEOUT)
            .open()
            .map_err(|error| {
                let message = format!("failed to open serial port: {error}");
                connection.status.error = Some(message.clone());
                message
            })?;

        let mut device_id = None;
        for _ in 0..HANDSHAKE_ATTEMPTS {
            match transact(&mut *port, "HELLO") {
                Ok(response) if is_supported_device_id(&response) => {
                    device_id = Some(response);
                    break;
                }
                _ => thread::sleep(HANDSHAKE_RETRY_DELAY),
            }
        }
        let Some(device_id) = device_id else {
            let message =
                "connected serial device did not identify as an AgentLight USB serial device";
            connection.status.error = Some(message.to_string());
            return Err(message.to_string());
        };

        let status_response =
            transact(&mut *port, &format!("STATUS:{}", state.as_protocol_name()))?;
        let expected = format!("ACK:STATUS:{}", state.as_protocol_name());
        if status_response != expected {
            let message = format!("unexpected device response: {status_response}");
            connection.status.error = Some(message.clone());
            return Err(message);
        }

        connection.status = HardwareStatus {
            connected: true,
            port_name: Some(port_name.to_string()),
            device_id: Some(device_id),
            error: None,
        };
        connection.port = Some(port);
        Ok(connection.status.clone())
    }

    pub fn disconnect(&self) -> Result<HardwareStatus, String> {
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| "hardware state lock is unavailable".to_string())?;
        connection.port = None;
        connection.status = HardwareStatus::default();
        Ok(connection.status.clone())
    }

    pub fn send_status(&self, state: DisplayState) -> Result<Option<HardwareStatus>, String> {
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| "hardware state lock is unavailable".to_string())?;
        let Some(port) = connection.port.as_mut() else {
            return Ok(None);
        };

        let expected = format!("ACK:STATUS:{}", state.as_protocol_name());
        match transact(&mut **port, &format!("STATUS:{}", state.as_protocol_name())) {
            Ok(response) if response == expected => Ok(None),
            Ok(response) => {
                let status = disconnect_with_error(
                    &mut connection,
                    format!("unexpected device response: {response}"),
                );
                Ok(Some(status))
            }
            Err(error) => {
                let status = disconnect_with_error(&mut connection, error);
                Ok(Some(status))
            }
        }
    }

    pub fn heartbeat(&self) -> Result<Option<HardwareStatus>, String> {
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| "hardware state lock is unavailable".to_string())?;
        let Some(port) = connection.port.as_mut() else {
            return Ok(None);
        };

        match transact(&mut **port, "PING") {
            Ok(response) if response == "PONG" => Ok(None),
            Ok(response) => {
                let status = disconnect_with_error(
                    &mut connection,
                    format!("unexpected heartbeat response: {response}"),
                );
                Ok(Some(status))
            }
            Err(error) => {
                let status = disconnect_with_error(&mut connection, error);
                Ok(Some(status))
            }
        }
    }
}

fn is_supported_device_id(response: &str) -> bool {
    response == SERIAL_DEVICE_ID || response == LEGACY_ESP8266_DEVICE_ID
}

fn disconnect_with_error(connection: &mut Connection, error: String) -> HardwareStatus {
    connection.port = None;
    connection.status = HardwareStatus {
        connected: false,
        port_name: None,
        device_id: None,
        error: Some(error),
    };
    connection.status.clone()
}

fn transact(port: &mut dyn serialport::SerialPort, command: &str) -> Result<String, String> {
    port.write_all(command.as_bytes())
        .and_then(|_| port.write_all(b"\n"))
        .and_then(|_| port.flush())
        .map_err(|error| format!("serial write failed: {error}"))?;
    read_line(port)
}

fn read_line<R: Read + ?Sized>(port: &mut R) -> Result<String, String> {
    let mut response = Vec::with_capacity(32);
    let mut byte = [0_u8; 1];
    while response.len() < MAX_RESPONSE_LENGTH {
        match port.read(&mut byte) {
            Ok(0) => continue,
            Ok(_) if byte[0] == b'\n' => {
                while response.last() == Some(&b'\r') {
                    response.pop();
                }
                return String::from_utf8(response)
                    .map_err(|error| format!("serial response was not UTF-8: {error}"));
            }
            Ok(_) => response.push(byte[0]),
            Err(error) if error.kind() == std::io::ErrorKind::TimedOut => {
                return Err("serial response timed out".to_string())
            }
            Err(error) => return Err(format!("serial read failed: {error}")),
        }
    }
    Err("serial response exceeded the maximum length".to_string())
}

impl DisplayState {
    fn as_protocol_name(self) -> &'static str {
        match self {
            DisplayState::Waiting => "WAITING",
            DisplayState::Working => "WORKING",
            DisplayState::Completed => "COMPLETED",
            DisplayState::Idle => "IDLE",
            DisplayState::Error => "ERROR",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{is_supported_device_id, read_line, DisplayState};
    use std::io::Cursor;

    #[test]
    fn protocol_names_cover_every_display_state() {
        assert_eq!(DisplayState::Waiting.as_protocol_name(), "WAITING");
        assert_eq!(DisplayState::Working.as_protocol_name(), "WORKING");
        assert_eq!(DisplayState::Completed.as_protocol_name(), "COMPLETED");
        assert_eq!(DisplayState::Idle.as_protocol_name(), "IDLE");
        assert_eq!(DisplayState::Error.as_protocol_name(), "ERROR");
    }

    #[test]
    fn response_reader_accepts_crlf_and_bounds_line_length() {
        let mut line = Cursor::new(b"PONG\r\n".to_vec());
        assert_eq!(read_line(&mut line).unwrap(), "PONG");

        let mut too_long = Cursor::new(vec![b'x'; 100]);
        assert!(read_line(&mut too_long)
            .unwrap_err()
            .contains("maximum length"));
    }

    #[test]
    fn accepts_generic_and_legacy_agentlight_device_ids() {
        assert!(is_supported_device_id("AGENTLIGHT:SERIAL:1"));
        assert!(is_supported_device_id("AGENTLIGHT:ESP8266:1"));
        assert!(!is_supported_device_id("AGENTLIGHT:ARDUINO:1"));
    }
}
