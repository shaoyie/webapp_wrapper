use std::{collections::HashMap, env};

use serde::Deserialize;
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};
use url::Url;

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

    WebviewWindowBuilder::new(app, window_label, WebviewUrl::External(config.webview_url()))
        .title(&config.app_name)
        .inner_size(1280.0, 800.0)
        .min_inner_size(960.0, 600.0)
        .center()
        .resizable(true)
        .fullscreen(false)
        .visible(true)
        .decorations(true)
        .build()
        .map(|_| ())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let shell_config = ShellConfig::resolve();
    println!(
        "Starting creator shell with app name '{}' and url '{}'",
        shell_config.app_name, shell_config.app_url
    );

    tauri::Builder::default()
        .setup(move |app| {
            spawn_shell_window(&app.handle(), &shell_config)?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
