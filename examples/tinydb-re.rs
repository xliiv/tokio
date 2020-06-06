use futures::sink::SinkExt;
use std::collections::HashMap;
use std::error::Error;
use std::sync::{Arc, Mutex};
use tokio;
use tokio::net::TcpListener;
use tokio::stream::StreamExt;
use tokio_util::codec;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let addr = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "127.0.0.1:8080".to_string());
    let mut listener = TcpListener::bind(&addr).await?;
    println!("Listening on: {}", &addr);

    let db = Arc::new(Database {
        map: Mutex::new(HashMap::new()),
    });

    loop {
        match listener.accept().await {
            Ok((socket, _)) => {
                let thread_db = Arc::clone(&db);
                tokio::spawn(async move {
                    let mut line_stream = codec::Framed::new(socket, codec::LinesCodec::new());

                    while let Some(line_result) = line_stream.next().await {
                        match line_result {
                            Ok(request_line) => {
                                let response = handle_request(&request_line, &thread_db);
                                let text_resp = response.serialize();
                                if let Err(e) = line_stream.send(text_resp.as_str()).await {
                                    eprintln!("error on sending response; error = {:?}", e);
                                }
                            }
                            Err(e) => eprintln!("error on decoding from socket; error = {:?}", e),
                        }
                    }
                });
            }
            Err(e) => eprintln!("error accepting socket; error = {:?}", e),
        }
    }
}

#[derive(Debug)]
struct Database {
    map: Mutex<HashMap<String, String>>,
}
#[derive(Debug)]
enum Request {
    Get { key: String },
    Set { key: String, value: String },
}
impl Request {
    fn parse(line: &str) -> Result<Self, String> {
        let splitted = line.split_ascii_whitespace().take(3).collect::<Vec<&str>>();
        match &splitted[..] {
            ["GET", ref key] => Ok(Request::Get {
                key: key.to_string(),
            }),
            ["SET", ref key, ref value] => Ok(Request::Set {
                key: key.to_string(),
                value: value.to_string(),
            }),
            _ => {
                let msg = "Unrecognized input.\nPossible inputs are\nGET <var>\nSET <var> <value>";
                Err(msg.to_string())
            }
        }
    }
}
enum Response {
    Value {
        key: String,
        value: String,
    },
    Set {
        key: String,
        value: String,
        previous: Option<String>,
    },
    Error {
        message: String,
    },
}
impl Response {
    fn serialize(&self) -> String {
        match self {
            Self::Value { key, value } => format!("{} = {}", key, value),
            Self::Set {
                key,
                value,
                previous,
            } => format!("set {} = `{}`, previous: {:?}", key, value, previous),
            Self::Error { message } => format!("error: {}", message),
        }
    }
}

fn handle_request(request_line: &str, db: &Arc<Database>) -> Response {
    let request = match Request::parse(&request_line) {
        Ok(request) => request,
        Err(message) => return Response::Error { message },
    };
    match request {
        Request::Get { key } => match db.map.lock().unwrap().get(&key) {
            Some(value) => Response::Value {
                key: key,
                value: value.to_string(),
            },
            None => Response::Error {
                message: format!("no key {}", key),
            },
        },
        Request::Set { key, value } => {
            let previous = db.map.lock().unwrap().insert(key.clone(), value.clone());
            Response::Set {
                key,
                value,
                previous,
            }
        }
    }
}

#[test]
fn test_parse() {
    // this is not real teat :)
    dbg!(Request::parse("GET x").unwrap());
    dbg!(Request::parse("SET x y").unwrap());
    dbg!(Request::parse("SET x"));
}
