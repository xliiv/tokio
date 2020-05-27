use futures::{future, SinkExt, StreamExt};
use std::error::Error;
use tokio::{io, net::TcpStream};
use tokio_util::codec;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let addr = "127.0.0.1:8080";
    let mut socket = TcpStream::connect(&addr).await?;
    println!("Connected to: {}", &addr);
    let (read_half, write_half) = socket.split();
    let mut client_stream = codec::FramedRead::new(read_half, codec::BytesCodec::new())
        // std_sink requires Bytes stream
        // here we have Result<BytesMut, Error> stream
        // so we have to map it, thus map in filter_map
        // however, we can also filter broken streams, thus filter in filter_map
        // so if broken stream, log the error and end the stream
        .filter_map(|result| match result {
            Ok(bytes) => future::ready(Some(bytes.freeze())),
            Err(e) => {
                println!("Cant read socket: {}", e);
                future::ready(None)
            }
        })
        .map(Ok);
    let mut client_sink = codec::FramedWrite::new(write_half, codec::BytesCodec::new());

    let mut std_stream = codec::FramedRead::new(io::stdin(), codec::BytesCodec::new())
        .map(|result| result.map(|bytes| bytes.freeze()));
    let mut std_sink = codec::FramedWrite::new(io::stdout(), codec::BytesCodec::new());

    future::try_join(
        client_sink.send_all(&mut std_stream),
        std_sink.send_all(&mut client_stream),
    ).await.expect("can't join sockets");

    Ok(())
}
