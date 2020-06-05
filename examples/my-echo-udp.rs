use std::error::Error;
use std::str::from_utf8;
use tokio::net::UdpSocket;

struct Server {
    socket: UdpSocket,
    buffer: Vec<u8>,
}
impl Server {
    async fn run(&mut self) -> Result<(), std::io::Error> {
        loop {
            // read message from client
            let (read_num, target) = self.socket.recv_from(&mut self.buffer).await?;
            let received = &self.buffer[0..read_num];
            let decoded = from_utf8(received).expect("Can't decode received data");
            dbg!(decoded);
            // echo back the message to client
            self.socket.send_to(received, target).await?;
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let addr = "127.0.0.1:8080";
    let socket = UdpSocket::bind(&addr).await?;
    println!("Listening on {}", &addr);

    Server {
        socket,
        buffer: vec![0; 1024],
    }
    .run()
    .await?;

    Ok(())
}
