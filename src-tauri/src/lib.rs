use std::{collections::HashMap, env, process::Command};

use serde::Deserialize;
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};
use url::Url;

#[tauri::command]
fn open_external_url(url: String) -> Result<(), String> {
    let parsed = Url::parse(&url).map_err(|_| "invalid url".to_string())?;
    if parsed.scheme() != "http" && parsed.scheme() != "https" {
        return Err("only http/https urls are allowed".to_string());
    }

    #[cfg(target_os = "windows")]
    let mut command = {
        let mut cmd = Command::new("cmd");
        cmd.args(["/C", "start", "", &url]);
        cmd
    };

    #[cfg(target_os = "macos")]
    let mut command = {
        let mut cmd = Command::new("open");
        cmd.arg(&url);
        cmd
    };

    #[cfg(all(unix, not(target_os = "macos")))]
    let mut command = {
        let mut cmd = Command::new("xdg-open");
        cmd.arg(&url);
        cmd
    };

    command
        .spawn()
        .map(|_| ())
        .map_err(|e| format!("failed to open external url: {e}"))
}

#[derive(Clone, Debug)]
struct ShellConfig {
    app_id: String,
    product_name: String,
    app_name: String,
    app_url: String,
}

impl ShellConfig {
    fn resolve() -> Self {
        let catalog = AppProfileCatalog::load();
        let cli = CliOverrides::parse();
        let requested_app_id = cli
            .app_id
            .clone()
            .or_else(|| env::var("APP_ID").ok());

        let (app_id, base_profile, used_default) = catalog.select_profile(requested_app_id);

        if used_default {
            println!(
                "[creator-shell] requested appId not found, falling back to '{}'.",
                catalog.default_app_id
            );
        }

        let mut config = ShellConfig {
            app_id,
            product_name: base_profile.product_name.clone(),
            app_name: base_profile.app_name.clone(),
            app_url: base_profile.app_url.clone(),
        };

        config.apply_env_overrides();
        config.apply_cli_overrides(cli);
        config.normalize();

        config
    }

    fn apply_env_overrides(&mut self) {
        if let Ok(name) = env::var("APP_NAME") {
            self.app_name = name;
        }
        if let Ok(url) = env::var("APP_URL") {
            self.app_url = url;
        }
    }

    fn apply_cli_overrides(&mut self, cli: CliOverrides) {
        if let Some(name) = cli.app_name {
            self.app_name = name;
        }
        if let Some(url) = cli.app_url {
            self.app_url = url;
        }
    }

    fn normalize(&mut self) {
        if self.app_name.trim().is_empty() {
            self.app_name = self.product_name.clone();
        }

        if Url::parse(&self.app_url).is_err() {
            self.app_url = DEFAULT_APP_URL.to_string();
        }
    }

    fn webview_url(&self) -> Url {
        Url::parse(&self.app_url).expect("app URL has been validated")
    }
}

const DEFAULT_APP_URL: &str = "http://192.168.1.92:3333";

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AppProfile {
    #[serde(default = "default_product_name")]
    product_name: String,
    app_name: String,
    app_url: String,
}

#[derive(Debug, Deserialize)]
struct AppProfileCatalog {
    #[serde(rename = "defaultAppId")]
    default_app_id: String,
    profiles: HashMap<String, AppProfile>,
}

impl AppProfileCatalog {
    fn load() -> Self {
        serde_json::from_str(include_str!("../../app-profiles/apps.json"))
            .expect("invalid app profile configuration")
    }

    fn select_profile(&self, requested: Option<String>) -> (String, AppProfile, bool) {
        match requested {
            Some(id) => self
                .profiles
                    .get(&id)
                    .cloned()
                    .map(|profile| (id, profile, false))
                    .unwrap_or_else(|| {
                        let fallback = self
                            .profiles
                            .get(&self.default_app_id)
                            .expect("default appId must exist");
                        (self.default_app_id.clone(), fallback.clone(), true)
                    }),
            None => {
                let profile = self
                    .profiles
                    .get(&self.default_app_id)
                    .expect("default appId must exist");
                (self.default_app_id.clone(), profile.clone(), false)
            }
        }
    }
}

fn default_product_name() -> String {
    "创作者".to_string()
}

#[derive(Default)]
struct CliOverrides {
    app_id: Option<String>,
    app_name: Option<String>,
    app_url: Option<String>,
}

impl CliOverrides {
    fn parse() -> Self {
        let mut overrides = CliOverrides::default();
        let mut args = env::args().skip(1);

        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--app-id" => overrides.app_id = args.next(),
                "--app-name" => overrides.app_name = args.next(),
                "--app-url" => overrides.app_url = args.next(),
                _ => {
                    if let Some(value) = arg.strip_prefix("--app-id=") {
                        overrides.app_id = Some(value.to_string());
                    } else if let Some(value) = arg.strip_prefix("--app-name=") {
                        overrides.app_name = Some(value.to_string());
                    } else if let Some(value) = arg.strip_prefix("--app-url=") {
                        overrides.app_url = Some(value.to_string());
                    }
                }
            }
        }

        overrides
    }
}

fn spawn_shell_window(app: &AppHandle, config: &ShellConfig) -> tauri::Result<()> {
    let window_label = "main";

    if app.get_webview_window(window_label).is_some() {
        return Ok(());
    }

    let download_fallback_script = r#"
      (() => {
        const invoke = (url) => window.__TAURI__?.core?.invoke?.("open_external_url", { url });
        const isLikelyDownload = (url) => {
          const lower = url.toLowerCase();
          return (
            /\/(download|export)(\/|\?|$)/.test(lower) ||
            /\.(zip|rar|7z|pdf|csv|xlsx?|docx?|pptx?|dmg|exe|msi|apk)(\?|#|$)/.test(lower)
          );
        };

        document.addEventListener(
          "click",
          (event) => {
            const link = event.target instanceof Element ? event.target.closest("a") : null;
            if (!link || !link.href) return;

            const href = link.href;
            const shouldOpenExternal =
              link.hasAttribute("download") ||
              link.target === "_blank" ||
              isLikelyDownload(href);

            if (!shouldOpenExternal || !/^https?:\/\//i.test(href)) return;

            event.preventDefault();
            invoke(href);
          },
          true
        );

        const rawOpen = window.open.bind(window);
        window.open = (url, ...rest) => {
          if (typeof url === "string" && /^https?:\/\//i.test(url)) {
            invoke(url);
            return null;
          }
          return rawOpen(url, ...rest);
        };
      })();
    "#;

    WebviewWindowBuilder::new(app, window_label, WebviewUrl::External(config.webview_url()))
        .title(&config.app_name)
        .inner_size(1280.0, 800.0)
        .min_inner_size(960.0, 600.0)
        .center()
        .resizable(true)
        .fullscreen(false)
        .visible(true)
        .decorations(true)
        .initialization_script(download_fallback_script)
        .build()
        .map(|_| ())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let shell_config = ShellConfig::resolve();
    println!(
        "Starting creator shell for appId '{}' with app name '{}' and url '{}'",
        shell_config.app_id, shell_config.app_name, shell_config.app_url
    );

    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![open_external_url])
        .setup(move |app| {
            spawn_shell_window(&app.handle(), &shell_config)?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
