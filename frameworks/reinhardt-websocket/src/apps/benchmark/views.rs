//! WebSocket handlers for the benchmark app.

use reinhardt::{ConsumerContext, Message, WebSocketResult, WebSocketRouter, websocket};

fn echo_message(message: Message) -> Option<Message> {
    match message {
        message @ (Message::Text { .. } | Message::Binary { .. }) => Some(message),
        _ => None,
    }
}

#[websocket("/ws", name = "echo")]
async fn echo(context: &mut ConsumerContext, message: Message) -> WebSocketResult<()> {
    if let Some(message) = echo_message(message) {
        context.connection.send(message).await?;
    }
    Ok(())
}

pub fn ws_url_patterns() -> WebSocketRouter {
    WebSocketRouter::new().consumer(echo)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_and_binary_messages_are_echoed() {
        let text = Message::text("hello".to_string());
        let binary = Message::binary(vec![0, 1, 2]);

        assert_eq!(echo_message(text.clone()), Some(text));
        assert_eq!(echo_message(binary.clone()), Some(binary));
        assert_eq!(echo_message(Message::Ping), None);
    }
}
