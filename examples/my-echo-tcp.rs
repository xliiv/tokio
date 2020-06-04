use std::error::Error;
use tokio::net::TcpListener;
use tokio::prelude::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let addr = "127.0.0.1:8080";
    let mut listener = TcpListener::bind(&addr).await?;
    println!("Serving at:{}", &addr);
    loop {
        let (mut socket, _) = listener.accept().await?;
        tokio::spawn(async move {
            let mut buffer = [0; 1024];
            loop {
                let read_count = socket.read(&mut buffer).await.expect("can't read socket");
                dbg!(&read_count);
                if read_count == 0 {
                    return;
                }
                socket
                    .write_all(&mut buffer)
                    .await
                    .expect("can't write socket");
            }
        });
    }
}
