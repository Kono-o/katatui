use std::io::Stdout;
use std::time::Duration;

use anyhow::Result;
use crossterm::{
   event::{DisableMouseCapture, EnableMouseCapture},
   execute,
   terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Frame, Terminal, backend::CrosstermBackend};
use tokio::sync::mpsc;

use crate::input::{self, InputEvent};
use crate::socket::{Socket, SocketAdapter, SocketEvent};

const TICK_MS: u64 = 100;

#[derive(Debug)]
pub enum TUIEvent<A: SocketAdapter> {
   Tick,
   Input(InputEvent),
   Socket(SocketEvent<A>),
}

pub trait TUIApp<A: SocketAdapter> {
   fn logic(&mut self, tui: &mut TUI<A>, event: TUIEvent<A>) -> bool;
   fn render(&mut self, frame: &mut Frame, draws: u64, has_socket: bool);
}

pub struct TUI<A: SocketAdapter> {
   term: Terminal<CrosstermBackend<Stdout>>,
   draws: u64,
   force_redraw: bool,
   socket_rx: Option<mpsc::Receiver<SocketEvent<A>>>,
   input_rx: mpsc::Receiver<InputEvent>,
   pub socket_tx: Option<mpsc::Sender<A::Send>>,
}

impl<A: SocketAdapter> TUI<A> {
   pub fn new() -> Result<Self> {
      Self::build(None, None)
   }

   pub fn new_with_socket(port: u16) -> Result<Self> {
      let (tx, rx) = Socket::<A>::spawn(port);
      Self::build(Some(tx), Some(rx))
   }

   fn build(
      socket_tx: Option<mpsc::Sender<A::Send>>,
      socket_rx: Option<mpsc::Receiver<SocketEvent<A>>>,
   ) -> Result<Self> {
      let orig = std::panic::take_hook();
      std::panic::set_hook(Box::new(move |info| {
         let _ = disable_raw_mode();
         let _ = execute!(std::io::stdout(), DisableMouseCapture, LeaveAlternateScreen);
         orig(info);
      }));

      enable_raw_mode()?;
      execute!(std::io::stdout(), EnterAlternateScreen, EnableMouseCapture)?;
      let mut term = Terminal::new(CrosstermBackend::new(std::io::stdout()))?;
      term.clear()?;

      Ok(Self {
         term,
         draws: 0,
         force_redraw: false,
         socket_rx,
         input_rx: input::spawn(),
         socket_tx,
      })
   }

   pub fn draws(&self) -> u64 {
      self.draws
   }
   pub fn has_socket(&self) -> bool {
      self.socket_rx.is_some()
   }
   pub fn redraw(&mut self) {
      self.force_redraw = true;
   }

   pub fn send(&self, msg: A::Send) {
      if let Some(tx) = &self.socket_tx {
         let _ = tx.try_send(msg);
      }
   }

   fn draw_frame(&mut self, app: &mut impl TUIApp<A>) -> Result<()> {
      self.draws += 1;
      self.force_redraw = false;
      let draws = self.draws;
      let has_socket = self.has_socket();
      self.term.draw(|f| app.render(f, draws, has_socket))?;
      Ok(())
   }

   fn cleanup(&mut self) -> Result<()> {
      disable_raw_mode()?;
      execute!(
         self.term.backend_mut(),
         DisableMouseCapture,
         LeaveAlternateScreen
      )?;
      self.term.show_cursor()?;
      Ok(())
   }

   fn try_recv_next(&mut self) -> Option<TUIEvent<A>> {
      if let Some(rx) = &mut self.socket_rx {
         if let Ok(e) = rx.try_recv() {
            return Some(TUIEvent::Socket(e));
         }
      }
      if let Ok(e) = self.input_rx.try_recv() {
         return Some(TUIEvent::Input(e));
      }
      None
   }

   pub async fn run(&mut self, app: &mut impl TUIApp<A>) -> Result<()> {
      self.draw_frame(app)?;

      let mut tick = tokio::time::interval(Duration::from_millis(TICK_MS));
      tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

      loop {
         let first = tokio::select! {
            _ = tick.tick() => TUIEvent::Tick,
            Some(e) = async {
               match &mut self.socket_rx {
                  Some(rx) => rx.recv().await,
                  None     => std::future::pending().await,
               }
            } => TUIEvent::Socket(e),
            Some(e) = self.input_rx.recv() => TUIEvent::Input(e),
         };

         if app.logic(self, first) {
            break;
         }

         while let Some(event) = self.try_recv_next() {
            if app.logic(self, event) {
               break;
            }
         }

         if self.force_redraw {
            self.draw_frame(app)?;
         }
      }

      self.cleanup()?;
      Ok(())
   }
}
