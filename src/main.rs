use std::{
    net::TcpListener,
    sync::{Arc, Mutex},
    thread::spawn,
    time::Duration,
};

use bus::Bus;
use tungstenite::Error;

fn main() {
    let bus = Arc::new(Mutex::new(Bus::new(10)));

    let bind_addr = "127.0.0.1:6678";
    let downstream_addresses = [
        // textractor-websocket, mpvacious
        "ws://localhost:6677",
        // agent
        "ws://localhost:9001",
    ];

    let server = TcpListener::bind(bind_addr).unwrap();
    println!("Listening on {bind_addr}");

    for addr in downstream_addresses {
        let bus = Arc::clone(&bus);
        spawn(move || loop {
            if let Ok((mut socket, _)) = tungstenite::connect(addr) {
                println!("Outgoing connection to {addr}");
                loop {
                    match socket.read() {
                        Ok(msg) => {
                            bus.lock().unwrap().broadcast(msg);
                        }
                        Err(Error::AlreadyClosed) => break,
                        Err(_) => {}
                    }
                }
                println!("Outgoing connection closed to {addr}");
            }

            std::thread::sleep(Duration::from_secs(1));
        });
    }

    for stream in server.incoming().flatten() {
        let rx = bus.lock().unwrap().add_rx();
        spawn(move || {
            let peer_addr = stream.peer_addr().unwrap();
            println!("Incoming connection from {peer_addr}");
            let mut websocket = tungstenite::accept(stream).unwrap();
            for msg in rx {
                websocket.send(msg).unwrap();
            }
            println!("Incoming connection closed from {peer_addr}");
        });
    }
}
