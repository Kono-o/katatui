use crate::{app_err, App, AppOutput};

pub(crate) fn install_cfg<A: App>() -> AppOutput<String> {
   let mut cfg_path = match dirs::config_dir() {
      None => return app_err!("failed to determine config dir"),
      Some(path) => path,
   };
   let mut cfg_app_path = cfg_path.clone();
   cfg_app_path.push(A::APP_NAME.to_string());

   let mut cfg_app_file = cfg_app_path.clone();
   cfg_app_file.push(A::CONFIG_FILE.to_string());

   let ok = AppOutput::ok(cfg_app_path.to_string_lossy().to_string());
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

pub(crate) fn read_cfg<A: App>() -> AppOutput<String> {
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
