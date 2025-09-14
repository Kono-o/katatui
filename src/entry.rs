use crate::app::{App, AppOutput};
use crate::app_err;
use crate::tui::TUI;

pub fn run<A: App>() -> AppOutput<()> {
   let install_result = install_default_cfg_to_disk::<A>();
   if let AppOutput::Err(_) = install_result {
      return install_result;
   }
   let cfg_src = match read_cfg_from_disk::<A>() {
      AppOutput::Ok(s) => s,
      AppOutput::Err(e) => return AppOutput::Err(e),
      AppOutput::Nil => return AppOutput::Nil,
   };
   let mut terminal = ratatui::init();
   let app_output = match TUI::<A>::new(cfg_src) {
      AppOutput::Ok(mut tui) => tui.run(&mut terminal),
      AppOutput::Err(e) => AppOutput::Err(e),
      AppOutput::Nil => AppOutput::Nil,
   };
   ratatui::restore();
   app_output
}

pub(crate) fn install_default_cfg_to_disk<A: App>() -> AppOutput<()> {
   pub const DEFAULT_CONFIG_SRC: &str = include_str!("../cfg_template.lua");
   let mut cfg_path = match dirs::config_dir() {
      Some(path) => path,
      None => return app_err!("failed to determine config dir"),
   };
   cfg_path.push(format!("{}/{}", A::APP_NAME, A::CONFIG_FILE));
   if cfg_path.exists() {
      return AppOutput::nil();
   }
   if let Some(parent) = cfg_path.parent() {
      if !parent.exists() {
         if let Err(e) = std::fs::create_dir_all(parent) {
            return app_err!(
               "failed to create parent dir for config at {:?}: {}",
               parent,
               e
            );
         }
      }
   } else {
      return app_err!("failed to get parent dir of config path");
   }
   if let Err(e) = std::fs::write(&cfg_path, DEFAULT_CONFIG_SRC) {
      return app_err!("failed to write default config at {:?}: {}", cfg_path, e);
   }
   AppOutput::nil()
}

pub(crate) fn read_cfg_from_disk<A: App>() -> AppOutput<String> {
   let mut cfg_path = match dirs::config_dir() {
      Some(path) => path,
      None => return app_err!("failed to determine config dir"),
   };
   cfg_path.push(format!("{}/{}", A::APP_NAME, A::CONFIG_FILE));
   match std::fs::read_to_string(&cfg_path) {
      Ok(s) => AppOutput::Ok(s),
      Err(e) => app_err!("failed to read config at {:?}: {}", cfg_path, e),
   }
}
