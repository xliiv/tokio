use futures::sink::SinkExt;
use futures::stream::StreamExt;
use std::collections::HashMap;
use std::error::Error;
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use tokio;
use tokio::net::TcpStream;
use tokio::stream::Stream;
use tokio::sync::mpsc;
use tokio::sync::Mutex;
use tokio_util::codec;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let addr = "127.0.0.1:8080";
    let mut listener = tokio::net::TcpListener::bind(&addr).await?;
    println!("Listening at {}", &addr);
    let shared: Arc<Mutex<Shared>> = Arc::new(Mutex::new(Shared::new()));

    loop {
        let (client, addr) = listener.accept().await?;
        println!("CONN {}", &&addr);
        let shared = Arc::clone(&shared);
        tokio::spawn(async move {
            if let Err(e) = handle_client(client, addr, shared).await {
                eprintln!("Error: {:?}", e);
            }
        });
    }
}

struct Shared {
    peers: HashMap<SocketAddr, mpsc::UnboundedSender<String>>,
}
impl Shared {
    fn new() -> Self {
        Shared {
            peers: HashMap::new(),
        }
    }
    //TODO async
    fn broadcast(&mut self, source_peer: &SocketAddr, message: &str) {
        for (addr, stream) in self.peers.iter_mut() {
            if addr != source_peer {
                let _ = stream.send(message.into());
            }
        }
    }
}

enum Message {
    FromOthers(String),
    FromUser(String),
}
struct Peer {
    user_lines: codec::Framed<TcpStream, codec::LinesCodec>,
    chat_msges: mpsc::UnboundedReceiver<String>,
}
impl Peer {
    pub fn new(
        user_lines: codec::Framed<TcpStream, codec::LinesCodec>,
        chat_msges: mpsc::UnboundedReceiver<String>,
    ) -> Self {
        Self {
            user_lines,
            chat_msges,
        }
    }
}
impl Stream for Peer {
    type Item = Result<Message, codec::LinesCodecError>;
    fn poll_next(mut self: Pin<&mut Self>, context: &mut Context) -> Poll<Option<Self::Item>> {
        // check for message from others
        if let Poll::Ready(Some(m)) = Pin::new(&mut self.chat_msges).poll_next(context) {
            return Poll::Ready(Some(Ok(Message::FromOthers(m))));
        }
        // check for message from user
        let line_opt = futures::ready!(Pin::new(&mut self.user_lines).poll_next(context));

        Poll::Ready(match line_opt {
            Some(Ok(msg)) => Some(Ok(Message::FromUser(msg))),
            Some(Err(e)) => Some(Err(e)),
            // The stream has been exhausted.
            None => None,
        })
    }
}

async fn handle_client(
    client: TcpStream,
    this_peer_addr: SocketAddr,
    shared: Arc<Mutex<Shared>>,
) -> Result<(), Box<dyn Error>> {
    let mut lines = codec::Framed::new(client, codec::LinesCodec::new());

    // read name of connected user
    lines.send("Enter your name:").await?;
    let username = match lines.next().await {
        Some(Ok(name)) => name,
        _ => {
            println!(
                "Failed to get username from {}. Client disconnected.",
                this_peer_addr
            );
            return Ok(());
        }
    };

    let (peer_tx, peer_rx) = mpsc::unbounded_channel();
    shared.lock().await.peers.insert(this_peer_addr, peer_tx);
    let mut this_peer = Peer::new(lines, peer_rx);
    while let Some(result) = this_peer.next().await {
        match result {
            Ok(Message::FromUser(msg)) => {
                // msg comes from user, so send the msg to others
                let full_msg = format!("{}: {}", username, msg);
                shared.lock().await.broadcast(&this_peer_addr, &full_msg);
            }
            Ok(Message::FromOthers(msg)) => {
                // msg from other users, so send it to the current user
                this_peer.user_lines.send(&msg).await?;
            }
            Err(e) => eprintln!("{}", e),
        }
    }

    //// client connected => insert it to Peers and tell others
    //let (peer_tx, mut peer_rx) = mpsc::unbounded_channel();
    //{
    //    let mut locked_state = shared.lock().await;
    //    locked_state.peers.insert(this_peer_addr, peer_tx);
    //    let msg = format!("{} has joined the chat", username);
    //    locked_state.broadcast(&this_peer_addr, &msg);
    //}

    //loop {
    //    tokio::select! {
    //        maybe_value = lines.next() => {
    //            match maybe_value {
    //                Some(Ok(line)) => {
    //                    println!("{}: READ: {}", &this_peer_addr, &line);
    //                    let msg = format!("{}: {}", username, &line);
    //                    shared.lock().await.broadcast(&this_peer_addr, &msg);
    //                }
    //                Some(Err(e)) => {
    //                    eprintln!("error {}", e);
    //                    break;
    //                }
    //                None => {
    //                    break;
    //                }
    //            }
    //        }
    //        Some(broadcasted_msg) = peer_rx.recv() => {
    //            println!("{}: BROADCASTED: {}", &this_peer_addr, &broadcasted_msg);
    //            lines.send(&broadcasted_msg).await?;
    //        }
    //    }
    //}

    {
        // client disconnected => remove it from Peers and tell others
        let mut locked_state = shared.lock().await;
        locked_state.peers.remove(&this_peer_addr);
        let msg = format!("{} has left the chat", username);
        locked_state.broadcast(&this_peer_addr, &msg);
    }

    Ok(())
}
