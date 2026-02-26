use crossterm::event::{Event, EventStream};
use futures::StreamExt;
use tokio::sync::mpsc;

#[derive(Debug)]
pub enum InputEvent {
   Crossterm(Event),
   Signal(Signal),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Signal {
   Terminate, // SIGTERM
   Interrupt, // SIGINT
   Hangup,    // SIGHUP
}

pub fn spawn() -> mpsc::Receiver<InputEvent> {
   let (tx, rx) = mpsc::channel(256);
   tokio::spawn(run(tx));
   rx
}

async fn run(tx: mpsc::Sender<InputEvent>) {
   #[cfg(unix)]
   let (mut sigterm, mut sigint, mut sighup) = {
      use tokio::signal::unix::{SignalKind, signal};
      (
         signal(SignalKind::terminate()).expect("failed to register SIGTERM"),
         signal(SignalKind::interrupt()).expect("failed to register SIGINT"),
         signal(SignalKind::hangup()).expect("failed to register SIGHUP"),
      )
   };

   let mut stream = EventStream::new();

   loop {
      #[cfg(unix)]
      tokio::select! {
          Some(Ok(event)) = stream.next() => {
              if tx.send(InputEvent::Crossterm(event)).await.is_err() { return; }
          }
          _ = sigterm.recv() => {
              let _ = tx.send(InputEvent::Signal(Signal::Terminate)).await;
              return;
          }
          _ = sigint.recv() => {
              let _ = tx.send(InputEvent::Signal(Signal::Interrupt)).await;
              return;
          }
          _ = sighup.recv() => {
              let _ = tx.send(InputEvent::Signal(Signal::Hangup)).await;
              return;
          }
      }

      #[cfg(not(unix))]
      tokio::select! {
          Some(Ok(event)) = stream.next() => {
              if tx.send(InputEvent::Crossterm(event)).await.is_err() { return; }
          }
          _ = tokio::signal::ctrl_c() => {
              let _ = tx.send(InputEvent::Signal(Signal::Interrupt)).await;
              return;
          }
      }
   }
}
