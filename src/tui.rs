use std::io::Stdout;
use std::time::Duration;

use anyhow::Result;
use crossterm::event::{Event, MouseEventKind};
use crossterm::{
   event::{DisableMouseCapture, EnableMouseCapture},
   execute,
   terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Frame, Terminal, backend::CrosstermBackend};
use tokio::sync::mpsc;

use crate::input::{self, InputEvent};
use crate::socket::{Socket, SocketAdapter, SocketEvent};

const TICK_MS: u64 = 10;

#[derive(Debug)]
pub enum TUIEvent<A: SocketAdapter> {
   Tick,
   Input(InputEvent),
   Socket(SocketEvent<A>),
}

pub trait TUIApp<A: SocketAdapter> {
   fn logic(&mut self, tui: &mut TUI<A>, event: TUIEvent<A>) -> bool;
   fn render(&mut self, tui: &TUI<A>, frame: &mut Frame);
}

pub struct TUI<A: SocketAdapter> {
   term: Terminal<CrosstermBackend<Stdout>>,
   redraw_tx: mpsc::UnboundedSender<()>,
   redraw_rx: mpsc::UnboundedReceiver<()>,
   socket_rx: Option<mpsc::Receiver<SocketEvent<A>>>,
   input_rx: mpsc::Receiver<InputEvent>,
   pub socket_tx: Option<mpsc::Sender<A::Send>>,
   ticks: u64,
   draws: u64,
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

      let (redraw_tx, redraw_rx) = mpsc::unbounded_channel();

      Ok(Self {
         term,
         redraw_tx,
         redraw_rx,
         socket_rx,
         input_rx: input::spawn(),
         socket_tx,
         ticks: 0,
         draws: 0,
      })
   }

   pub fn has_socket(&self) -> bool {
      self.socket_rx.is_some()
   }

   pub fn ticks(&self) -> u64 {
      self.ticks
   }
   pub fn tick_ms(&self) -> u64 {
      TICK_MS
   }
   pub fn draws(&self) -> u64 {
      self.draws
   }

   pub fn redraw(&self) {
      let _ = self.redraw_tx.send(());
   }

   pub fn send(&self, msg: A::Send) {
      if let Some(tx) = &self.socket_tx {
         let _ = tx.try_send(msg);
      }
   }

   fn draw(&mut self, app: &mut impl TUIApp<A>) -> Result<()> {
      self.draws += 1;
      // SAFETY: we split the borrow — term is mutably borrowed for draw,
      // the rest of self is passed as an immutable ref inside the closure.
      let tui_ref = unsafe { &*(self as *const Self) };
      self.term.draw(|f| app.render(tui_ref, f))?;
      Ok(())
   }

   fn flush_redraws(&mut self) {
      while self.redraw_rx.try_recv().is_ok() {}
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

   pub async fn run(&mut self, app: &mut impl TUIApp<A>) -> Result<()> {
      self.draw(app)?;

      let mut tick = tokio::time::interval(Duration::from_millis(TICK_MS));
      tick.tick().await; // consume the immediate first tick

      loop {
         enum Wake<A: SocketAdapter> {
            Redraw,
            Tick,
            Input(InputEvent),
            Socket(SocketEvent<A>),
         }

         let wake = tokio::select! {
             _ = self.redraw_rx.recv() => Wake::Redraw,
             _ = tick.tick() => Wake::Tick,
             Some(e) = async {
                 match &mut self.socket_rx {
                     Some(rx) => rx.recv().await,
                     None => std::future::pending().await,
                 }
             } => Wake::Socket(e),
             Some(e) = self.input_rx.recv() => Wake::Input(e),
         };

         match wake {
            Wake::Redraw => {
               self.draw(app)?;
            }
            Wake::Tick => {
               self.ticks += 1;
               if app.logic(self, TUIEvent::Tick) {
                  break;
               }
            }
            Wake::Input(e) => {
               let mut should_draw = triggers_redraw(&e);
               if app.logic(self, TUIEvent::Input(e)) {
                  break;
               }
               loop {
                  match self.input_rx.try_recv() {
                     Ok(e) => {
                        should_draw |= triggers_redraw(&e);
                        if app.logic(self, TUIEvent::Input(e)) {
                           self.cleanup()?;
                           return Ok(());
                        }
                     }
                     Err(_) => break,
                  }
               }
               if should_draw {
                  self.flush_redraws();
                  self.draw(app)?;
               }
            }
            Wake::Socket(e) => {
               if app.logic(self, TUIEvent::Socket(e)) {
                  break;
               }
               loop {
                  match self.socket_rx.as_mut().and_then(|rx| rx.try_recv().ok()) {
                     Some(e) => {
                        if app.logic(self, TUIEvent::Socket(e)) {
                           self.cleanup()?;
                           return Ok(());
                        }
                     }
                     None => break,
                  }
               }
               self.flush_redraws();
               self.draw(app)?;
            }
         }
      }

      self.cleanup()?;
      Ok(())
   }
}

fn triggers_redraw(e: &InputEvent) -> bool {
   match e {
      InputEvent::Crossterm(Event::Mouse(m)) => match m.kind {
         MouseEventKind::Moved => return false,
         _ => {}
      },
      _ => {}
   }
   true
}
