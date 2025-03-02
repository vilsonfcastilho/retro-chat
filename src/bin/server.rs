use chrono::Local;
use serde::{Deserialize, Serialize};
use std::error::Error;
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::{TcpListener, TcpStream},
    sync::broadcast,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
enum MessageType {
    UserMessage,
    SystemNotification,
}

/// This is an attribute instructing the compiler to auto-generate impls for the 4 traits
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ChatMessage {
    username: String,
    content: String,
    timestamp: String,
    message_type: MessageType,
}

// What is Tokio?
// Tokio is a ASYNC runtime for the rust language.
// Async programming allows you to do things wile waiting for one task to finish.
/// This is an attribute to the main function to tranform the main function in to async
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Bind the server to the specified IP and port
    let listener: TcpListener = TcpListener::bind("127.0.0.1:8082").await?;

    // Display server startup message with formatting
    println!("╔═══════════════════════════════════════════╗");
    println!("║        RETRO CHAT SERVER ACTIVE           ║");
    println!("║        Port: 8082  Host: 127.0.0.1        ║");
    println!("║        Press Ctrl+C to shutdown           ║");
    println!("╚═══════════════════════════════════════════╝");

    // Create a broadcast channel for message distribution
    let (tx, _) = broadcast::channel::<String>(100);

    loop {
        // Accepting the new connection
        let (socket, addr) = listener.accept().await?;

        // Display the connection information
        println!("┌─[{}] New connection", Local::now().format("%H:%M:%S"));
        println!("└─ Address: {}", addr);

        // Clone sender for this connection and subscribe a receiver
        let tx: broadcast::Sender<String> = tx.clone();
        let rx: broadcast::Receiver<String> = tx.subscribe();

        tokio::spawn(async move { handle_connection(socket, tx, rx).await });
    }
}

// Function to handle the client connection
async fn handle_connection(
    mut socket: TcpStream,
    tx: broadcast::Sender<String>,
    mut rx: broadcast::Receiver<String>,
) {
    let (reader, mut writer) = socket.split();
    let mut reader = BufReader::new(reader);
    let mut username: String = String::new();

    // Read the username sent by the client
    reader.read_line(&mut username).await.unwrap();
    let username: String = username.trim().to_string();

    // Send a system notification indicating the user has joined the chat
    let join_msg: ChatMessage = ChatMessage {
        username: username.clone(),
        content: "joined the chat".to_string(),
        timestamp: Local::now().format("%H:%M:%S").to_string(),
        message_type: MessageType::SystemNotification,
    };
    let join_json: String = serde_json::to_string(&join_msg).unwrap();
    tx.send(join_json).unwrap();

    // Initialize a buffer for incoming messages from the client
    let mut line: String = String::new();
    loop {
        tokio::select! {
            result = reader.read_line(&mut line) => {
                if result.unwrap() == 0 {
                    break;
                }

                // Create and broadcast user message
                let msg = ChatMessage {
                    username: username.clone(),
                    content: line.trim().to_string(),
                    timestamp: Local::now().format("%H:%M:%S").to_string(),
                    message_type: MessageType::UserMessage,
                };
                let json = serde_json::to_string(&msg).unwrap();
                tx.send(json).unwrap();
                line.clear();
            }

            // Handle the incoming broadcast and sed them to the client
            result = rx.recv() => {
                let msg = result.unwrap();
                writer.write_all(msg.as_bytes()).await.unwrap();
                writer.write_all(b"\n").await.unwrap();
            }
        }
    }

    let leave_msg: ChatMessage = ChatMessage {
        username: username.clone(),
        content: "left the chat".to_string(),
        timestamp: Local::now().format("%H:%M:%S").to_string(),
        message_type: MessageType::SystemNotification,
    };
    let leave_json: String = serde_json::to_string(&leave_msg).unwrap();
    tx.send(leave_json).unwrap();

    // Log disconnection info to the terminal
    println!(
        "└─[{}] {} disconnected",
        Local::now().format("%H:%M:%S"),
        username
    );
}
