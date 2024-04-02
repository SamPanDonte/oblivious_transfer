use std::error::Error;
use std::time::Duration;

mod net;
mod net2;

#[tokio::main(flavor = "current_thread")]
pub async fn start() {
    println!("Hello, world async!");

    if let Err(err) = start_inner().await {
        println!("Error occurred: {err}")
    }
}

async fn start_inner() -> Result<(), Box<dyn Error>> {
    let socket = tokio::net::UdpSocket::bind("0.0.0.0:12345").await?;
    socket.set_broadcast(true)?;

    tokio::spawn(async {
        let mut interval = tokio::time::interval(Duration::from_secs(5));
        let sock = tokio::net::UdpSocket::bind("0.0.0.0:0").await.unwrap();
        sock.set_broadcast(true).unwrap();
        sock.connect("10.44.255.255:12345").await.unwrap();

        loop {
            interval.tick().await;
            match sock.send_to(b"1234", "10.44.255.255:12345").await {
                Ok(amount) => println!("Successfully send {amount} bytes"),
                Err(err) => println!("Error when sending: {err}"),
            };
        }
    });

    let mut buff = [0; 1024];

    loop {
        match socket.recv_from(&mut buff).await {
            Ok((amount, who)) => println!("Received ({amount}): {:?} form {who}", &buff[..amount]),
            Err(err) => println!("Error when receiving: {err}"),
        }
    }
}
