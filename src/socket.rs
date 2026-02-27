use std::fmt;
use std::time::Duration;

use futures::{SinkExt, StreamExt};
use prost::Message as ProstMessage;
use tokio::sync::mpsc;
use tokio_tungstenite::{connect_async, tungstenite::Message};

const RECONNECT_MS: u64 = 400;
const WAIT_MS: u64 = 300;

pub trait SocketAdapter: Send + 'static {
   type Send: ProstMessage + Default + Send + 'static;
   type Receive: ProstMessage + Default + Send + fmt::Debug + 'static;
}

#[derive(Debug)]
pub enum SocketEvent<A: SocketAdapter> {
   /// Socket task has started and is attempting its first connection.
   Starting,
   /// Successfully established a WebSocket connection.
   Connected,
   /// The server cleanly closed the connection (graceful close frame).
   Closed,
   /// An active connection dropped unexpectedly; will attempt to reconnect.
   ConnectionLost,
   /// A connection attempt failed while no session was active.
   CouldntConnect,
   /// About to wait `ms` milliseconds before the next reconnect attempt.
   AttemptingToReconnectIn(u64),
   /// A successfully decoded packet was received from the server.
   Packet(A::Receive),
}

pub struct Socket<A: SocketAdapter> {
   port: u16,
   _adapter: std::marker::PhantomData<A>,
}

impl<A: SocketAdapter> Socket<A> {
   pub fn spawn(port: u16) -> (mpsc::Sender<A::Send>, mpsc::Receiver<SocketEvent<A>>) {
      let (event_tx, event_rx) = mpsc::channel(256);
      let (cmd_tx, cmd_rx) = mpsc::channel(256);
      let socket = Socket::<A> {
         port,
         _adapter: std::marker::PhantomData,
      };
      tokio::spawn(socket.run(event_tx, cmd_rx));
      (cmd_tx, event_rx)
   }

   async fn run(self, tx: mpsc::Sender<SocketEvent<A>>, mut cmd_rx: mpsc::Receiver<A::Send>) {
      let url = format!("ws://localhost:{}/ws", self.port);
      let _ = tx.send(SocketEvent::Starting).await;

      // True only while a session is currently active. Reset to false
      // as soon as the session ends so failed reconnects emit CouldntConnect.
      let mut connected = false;

      loop {
         let ws = match connect_async(&url).await {
            Ok((ws, _)) => ws,
            Err(_) => {
               if connected {
                  let _ = tx.send(SocketEvent::ConnectionLost).await;
                  connected = false;
               } else {
                  let _ = tx.send(SocketEvent::CouldntConnect).await;
               }
               tokio::time::sleep(Duration::from_millis(WAIT_MS)).await; // let the UI catch up
               let _ = tx
                  .send(SocketEvent::AttemptingToReconnectIn(RECONNECT_MS))
                  .await;
               tokio::time::sleep(Duration::from_millis(RECONNECT_MS)).await;
               continue;
            }
         };

         let _ = tx.send(SocketEvent::Connected).await;
         let (mut sink, mut stream) = ws.split();

         let graceful = loop {
            tokio::select! {
                msg = stream.next() => match msg {
                    Some(Ok(Message::Binary(data))) => {
                        match A::Receive::decode(data) {
                            Ok(decoded) => {
                                if tx.send(SocketEvent::Packet(decoded)).await.is_err() {
                                    return;
                                }
                            }
                            Err(_) => {} // ignore malformed packets
                        }
                    }
                    Some(Ok(Message::Close(_))) => break true,
                    Some(Err(_)) => break false,
                    None => break false,
                    _ => {}
                },
                Some(msg) = cmd_rx.recv() => {
                    let _ = sink.send(Message::Binary(msg.encode_to_vec().into())).await;
                }
            }
         };

         connected = false;

         if graceful {
            let _ = tx.send(SocketEvent::Closed).await;
            return;
         }

         let _ = tx.send(SocketEvent::ConnectionLost).await;
         tokio::time::sleep(Duration::from_millis(WAIT_MS)).await;
         let _ = tx
            .send(SocketEvent::AttemptingToReconnectIn(RECONNECT_MS))
            .await;
         tokio::time::sleep(Duration::from_millis(RECONNECT_MS)).await;
      }
   }
}
