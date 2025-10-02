use crate::{app_err, App, AppOutput};
use mlua::Lua;

pub(crate) type CfgPath = Option<String>;
pub(crate) type CfgSrc = Option<String>;
pub(crate) type Cfg = Option<Lua>;

pub(crate) fn install_cfg<A: App>() -> AppOutput<CfgPath> {
   let mut cfg_path = match dirs::config_dir() {
      None => return app_err!("failed to determine config dir"),
      Some(path) => path,
   };
   let mut cfg_app_path = cfg_path.clone();
   cfg_app_path.push(A::APP_NAME.to_string());

   let ok = AppOutput::ok(Some(cfg_app_path.to_string_lossy().to_string()));

   let cfg_file = match A::CONFIG_FILE {
      None => return ok,
      Some(f) => f,
   };

   let mut cfg_app_file = cfg_app_path.clone();
   cfg_app_file.push(cfg_file.to_string());

   if cfg_app_file.exists() {
      return ok;
   }
   if let Some(parent) = cfg_app_file.parent() {
      if !parent.exists() {
         if let Err(e) = std::fs::create_dir_all(parent) {
            return app_err!(
               "failed to create parent dir {} for config at {:?}: {}",
               A::APP_NAME,
               parent,
               e
            );
         }
      }
   } else {
      return app_err!("failed to get parent dir of config path");
   }
   if let Err(e) = std::fs::write(&cfg_app_file, A::DEFAULT_CONFIG_SRC) {
      return app_err!(
         "failed to write default config at {:?}: {}",
         cfg_app_file,
         e
      );
   }
   ok
}

pub(crate) fn read_cfg<A: App>() -> AppOutput<CfgSrc> {
   let mut cfg_path = match dirs::config_dir() {
      Some(path) => path,
      None => return app_err!("failed to determine config dir"),
   };
   let cfg_file = match A::CONFIG_FILE {
      None => return AppOutput::ok(None),
      Some(f) => f,
   };
   cfg_path.push(format!("{}/{}", A::APP_NAME, cfg_file));
   match std::fs::read_to_string(&cfg_path) {
      Ok(src) => AppOutput::Ok(Some(src)),
      Err(e) => app_err!("failed to read config at {:?}: {}", cfg_path, e),
   }
}
