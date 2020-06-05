use std::error::Error;
use tokio;
use tokio::net::UdpSocket;
use tokio_util::codec;
use tokio::io;
use tokio::stream::StreamExt;
use tokio::io::{Stdin, Stdout};
use tokio::net::udp::{RecvHalf, SendHalf};


#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let addr = "127.0.0.1:8080";
    let socket = UdpSocket::bind("127.0.0.1:0").await?;
    dbg!(socket.local_addr()?);
    socket.connect(&addr).await?;
    println!("Connected to: {}", &addr);

    let (socket_out, socket_in) = socket.split();
    let std_in = codec::FramedRead::new(io::stdin(), codec::BytesCodec::new());
    let std_out = codec::FramedWrite::new(io::stdout(), codec::BytesCodec::new());

    tokio::try_join!(
        socket_to_stdout(socket_out, std_out), stdin_to_socket(std_in, socket_in),
    )?;

    Ok(())
}

async fn socket_to_stdout(
    mut socket_reader: RecvHalf,
    mut std_out: codec::FramedWrite<Stdout, codec::BytesCodec>,
) -> Result<(), Box<dyn Error>>{
    loop {
        let mut buffer = vec![0; 1024];
        let bytes_count = socket_reader.recv(&mut buffer).await?;
        if bytes_count > 0 {
            use bytes::Bytes;
            use futures::SinkExt;
            std_out.send(Bytes::from(buffer)).await?;
        }
    }

}

async fn stdin_to_socket(
    mut stdin: codec::FramedRead<Stdin, codec::BytesCodec>,
    mut socket_in: SendHalf,
) -> Result<(), Box<dyn Error>>{
    while let Some(item) = stdin.next().await {
        let buffer = item?;
        socket_in.send(&buffer).await?;
    }

    Ok(())
}
