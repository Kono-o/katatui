use std::fmt;
use std::time::Duration;

use futures::{SinkExt, StreamExt};
use prost::Message as ProstMessage;
use tokio::sync::mpsc;
use tokio_tungstenite::{connect_async, tungstenite::Message};

const SLEEP_MS: u64 = 200;

pub trait SocketAdapter: Send + 'static {
   type Send: ProstMessage + Default + Send + 'static;
   type Receive: ProstMessage + Default + Send + fmt::Debug + 'static;
}

#[derive(Debug)]
pub enum SocketEvent<A: SocketAdapter> {
   Starting,
   Connected,
   Disconnected,
   Reconnecting(u64),
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

      loop {
         let ws = match connect_async(&url).await {
            Ok((ws, _)) => ws,
            Err(_) => {
               tokio::time::sleep(Duration::from_millis(SLEEP_MS)).await;
               let _ = tx.send(SocketEvent::Reconnecting(SLEEP_MS)).await;
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

         if graceful {
            let _ = tx.send(SocketEvent::Disconnected).await;
            return;
         }

         tokio::time::sleep(Duration::from_millis(SLEEP_MS)).await;
         let _ = tx.send(SocketEvent::Reconnecting(SLEEP_MS)).await;
      }
   }
}
