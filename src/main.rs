use std::{
    collections::HashMap,
    time::Duration,
    thread::sleep,
    io::Read
};
use serde::{Serialize,Deserialize};

const BAUD_RATE: u32 = 9600;

#[derive(Serialize, Deserialize, Debug)]
struct TemperatureReading {
    humidity: i32,
    temperature: i32
}

fn main() {
    dotenv::dotenv().ok();
    let probeid_text = std::env::var("PROBE_ID").expect("PROBE_ID environment variable is not set");
    let probeid = probeid_text.parse::<i32>().expect("unable to parse ./probeid to int");
    let url = std::env::var("URL").expect("URL environment variable is not set");
    let token = std::env::var("TOKEN").expect("TOKEN environment variable is not set");
    let com_port = std::env::var("COM_PORT").expect("COM_PORT environment variable is not set");

    loop {
        sleep(Duration::from_millis(500));
        println!("Waiting for temperature probe on {}", &com_port);
        let ports = serialport::available_ports().expect("No ports found!");

        for p in ports {
            println!("Found device on {}", p.port_name);
            if p.port_name == com_port {
                let port = serialport::new(&com_port, BAUD_RATE)
                    .timeout(Duration::from_millis(2000))
                    .open();

                match port {
                    Ok(mut port) => {
                        let mut serial_buf: Vec<u8> = vec![0; 256];
                        let _ =port.read(serial_buf.as_mut_slice());
                        match process_data(serial_buf) {
                            Ok(data) => {
                                println!("{:?}", data);
                                send_data(data, probeid, &url, &token)
                            },
                            Err(e) => {println!("{:?}", e)}
                        }
                    },
                    Err(_) => {
                        println!("Error connecting to port {} with a baudrate of {}", &com_port, BAUD_RATE);
                    }
                }
            }
        }
    }
}

fn send_data(temp: TemperatureReading, probeid: i32, url: &str, token: &str) {
    let mut body = HashMap::new();
    body.insert("id", probeid);
    body.insert("temp", temp.temperature);

    let cb = reqwest::blocking::ClientBuilder::new();
    match cb.build() {
        Ok(client) => {
            let res = client.post(url)
                .bearer_auth(token)
                .json(&body)
                .send();
            match res {
                Ok(res) => {
                    if res.status().is_success() {
                        println!("temp data sent to controller");
                    } else if res.status().is_server_error() {
                        println!("controller replied with server error!");
                    } else {
                        println!("controller responded with unkown error. Status: {:?}, {:?}", res.status(), res.text());
                    }
                },
                Err(e) => { println!("{:?}", e) }
            }

        }
        Err(e) => {println!("{:?}", e)}
    }

}

fn process_data(data: Vec<u8>) -> Result<TemperatureReading, serde_json::Error> {
    let clean: &[_] = &['\0','\r', '\n'];
    let text = String::from_utf8_lossy(&data.clone()[..]).trim_matches(clean).to_string();
    let split_result = text.split("\r\n").collect::<Vec<_>>();
    if split_result.len() >= 2 {
        serde_json::from_str(&split_result[1])
    } else {
        serde_json::from_str(&text)
    }
}
