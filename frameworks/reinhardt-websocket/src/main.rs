use futures_util::{SinkExt, StreamExt};
use reinhardt_websockets::Message as ReinhardtMessage;
use tokio::io::AsyncWriteExt;
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::accept_hdr_async;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::tungstenite::handshake::server::{ErrorResponse, Request, Response};

fn validate_path(request: &Request, response: Response) -> Result<Response, ErrorResponse> {
    if request.uri().path() == "/ws" {
        Ok(response)
    } else {
        Err(http_error(404, "Not Found"))
    }
}

fn http_error(status: u16, body: &str) -> ErrorResponse {
    tokio_tungstenite::tungstenite::http::Response::builder()
        .status(status)
        .body(Some(body.to_string()))
        .expect("static WebSocket error response must be valid")
}

fn echo_data_message(message: Message) -> Option<Message> {
    match message {
        Message::Text(data) => match ReinhardtMessage::text(data.to_string()) {
            ReinhardtMessage::Text { data } => Some(Message::Text(data.into())),
            _ => unreachable!("text constructor returned another variant"),
        },
        Message::Binary(data) => match ReinhardtMessage::binary(data.to_vec()) {
            ReinhardtMessage::Binary { data } => Some(Message::Binary(data.into())),
            _ => unreachable!("binary constructor returned another variant"),
        },
        _ => None,
    }
}

async fn handle_connection(stream: TcpStream) {
    let mut stream = stream;
    if !has_websocket_upgrade(&stream).await {
        let _ = stream
            .write_all(
                b"HTTP/1.1 400 Bad Request\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
            )
            .await;
        return;
    }
    let Ok(websocket) = accept_hdr_async(stream, validate_path).await else {
        return;
    };
    let (mut writer, mut reader) = websocket.split();

    while let Some(message) = reader.next().await {
        let Ok(message) = message else {
            break;
        };

        match message {
            message @ (Message::Text(_) | Message::Binary(_)) => {
                if let Some(echo) = echo_data_message(message)
                    && writer.send(echo).await.is_err()
                {
                    break;
                }
            }
            Message::Ping(payload) => {
                if writer.send(Message::Pong(payload)).await.is_err() {
                    break;
                }
            }
            Message::Pong(_) => {}
            Message::Close(frame) => {
                let _ = writer.send(Message::Close(frame)).await;
                let _ = writer.flush().await;
                break;
            }
            Message::Frame(_) => {}
        }
    }
}

async fn has_websocket_upgrade(stream: &TcpStream) -> bool {
    let mut buffer = [0_u8; 4096];
    let headers = tokio::time::timeout(std::time::Duration::from_secs(2), async {
        loop {
            let read = stream.peek(&mut buffer).await.ok()?;
            if read == 0 {
                return None;
            }
            if buffer[..read]
                .windows(4)
                .any(|window| window == b"\r\n\r\n")
            {
                return std::str::from_utf8(&buffer[..read]).ok().map(str::to_owned);
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .ok()
    .flatten();

    let Some(headers) = headers else {
        return false;
    };
    let headers = headers.to_ascii_lowercase();
    headers.contains("\r\nupgrade: websocket\r\n")
        && headers
            .lines()
            .any(|line| line.starts_with("connection:") && line.contains("upgrade"))
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let listener = TcpListener::bind("0.0.0.0:8080").await?;
    println!("Reinhardt WebSocket echo server listening on ws://0.0.0.0:8080/ws");

    loop {
        let (stream, _) = listener.accept().await?;
        tokio::spawn(handle_connection(stream));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_and_binary_frames_round_trip_through_reinhardt_messages() {
        assert_eq!(
            echo_data_message(Message::Text("hello".into())),
            Some(Message::Text("hello".into()))
        );
        assert_eq!(
            echo_data_message(Message::Binary(vec![0, 1, 2].into())),
            Some(Message::Binary(vec![0, 1, 2].into()))
        );
    }
}
