use std::{
	collections::HashMap, fmt::Display, net::SocketAddr, path::Path, str::FromStr, sync::Arc,
};

use anyhow::Context;
use base64::{Engine, engine::GeneralPurposeConfig};
use http_body_util::Full;
use hyper::{
	Method, Request, Response,
	body::{Bytes, Incoming},
	server::conn::http1,
};
use hyper_util::rt::TokioIo;
use nitro_core::{
	auth_crate::mc::ClientId,
	io::{json_from_file, json_to_file},
};
use nitro_net::download::Client;
use nitro_shared::output::{MessageContents, NitroOutput};
use nitrolaunch::{
	config::Config,
	config_crate::{instance::InstanceConfig, template::TemplateConfig},
	io::paths::Paths,
	plugin::PluginManager,
};
use serde::{Deserialize, Serialize};
use tokio::{net::TcpListener, sync::Mutex};

use crate::get_dir;

pub const PORT: u16 = 1112;

pub async fn run(paths: &Paths, o: &mut impl NitroOutput) -> anyhow::Result<()> {
	let remote_dir = get_dir(&paths.data);
	if !remote_dir.exists() {
		let _ = std::fs::create_dir_all(&remote_dir);
	}

	let settings = RemoteSettings::open(&remote_dir).unwrap_or_default();
	let settings = Arc::new(settings);

	let addr: SocketAddr = ([127, 0, 0, 1], PORT).into();
	let listener = TcpListener::bind(addr)
		.await
		.context("Failed to bind to port")?;

	let plugins = PluginManager::load(paths, o).await?;
	let client_id = get_ms_client_id();
	let config = Config::load(&Config::get_path(paths), plugins, true, paths, client_id, o).await?;
	let config = Arc::new(Mutex::new(config));
	let client = Client::new();

	o.display(MessageContents::Success("Server started".into()));

	loop {
		let Ok((stream, _)) = listener.accept().await else {
			o.display(MessageContents::Error("Failed to accept connection".into()));
			continue;
		};
		let io = TokioIo::new(stream);
		let mut o = o.get_lesser_copy();
		let o2 = o.get_lesser_copy();
		let config = config.clone();
		let settings = settings.clone();
		let client = client.clone();

		tokio::spawn(async move {
			if let Err(e) = http1::Builder::new()
				.serve_connection(
					io,
					hyper::service::service_fn(|req| {
						handle(
							req,
							config.clone(),
							settings.clone(),
							client.clone(),
							o2.get_lesser_copy(),
						)
					}),
				)
				.await
			{
				o.display(MessageContents::Error(format!(
					"Failed to serve connection: {e}"
				)));
			}
		});
	}
}

async fn handle(
	req: Request<Incoming>,
	config: Arc<Mutex<Config>>,
	settings: Arc<RemoteSettings>,
	client: Client,
	mut o: impl NitroOutput,
) -> anyhow::Result<Response<Full<Bytes>>> {
	let path = req.uri().path();
	let method = req.method();
	let key = req
		.headers()
		.get("Authorization")
		.map(|x| x.to_str().unwrap_or(""));
	if path == "/" {
		Ok(Response::builder()
			.status(200)
			.body(Full::new(Bytes::from_static(b"OK")))
			.unwrap())
	} else if path == "/sync" && method == Method::GET {
		if let Some(response) = settings.check_key_header(key, KeyPermission::Query) {
			return Ok(response);
		}
		let config = config.lock().await;
		let instances = config
			.instances
			.iter()
			.map(|(id, instance)| (id.to_string(), instance.config().clone()))
			.collect::<HashMap<_, _>>();
		let templates = config
			.templates
			.iter()
			.map(|(id, template)| (id.to_string(), template.clone()))
			.collect::<HashMap<_, _>>();
		let base_template = config.base_template.clone();
		let response = SyncResponse {
			instances,
			templates,
			base_template,
		};

		json_response(&response)
	} else {
		Ok(Response::builder()
			.status(404)
			.body(Full::new(Bytes::from_static(b"Not Found")))
			.unwrap())
	}
}

fn json_response<T: Serialize>(data: &T) -> anyhow::Result<Response<Full<Bytes>>> {
	let json = serde_json::to_string(data).context("Failed to serialize JSON")?;
	Ok(Response::builder()
		.status(200)
		.header("Content-Type", "application/json")
		.body(Full::new(Bytes::from(json)))
		.unwrap())
}

#[derive(Serialize, Deserialize, Default)]
#[serde(default)]
pub struct RemoteSettings {
	keys: Vec<Key>,
}

impl RemoteSettings {
	pub fn open(dir: &Path) -> anyhow::Result<Self> {
		json_from_file(dir.join("settings.json"))
	}

	pub fn save(&self, dir: &Path) -> anyhow::Result<()> {
		let _ = std::fs::create_dir_all(dir);
		json_to_file(dir.join("settings.json"), self)
	}

	fn check_key(&self, key: &str, permission: KeyPermission) -> bool {
		let Some(key) = self.keys.iter().find(|k| k.id == key) else {
			return false;
		};
		key.permissions.is_empty() || key.permissions.contains(&permission)
	}

	fn check_key_header(
		&self,
		header: Option<&str>,
		permission: KeyPermission,
	) -> Option<Response<Full<Bytes>>> {
		let response = Response::builder()
			.status(401)
			.body(Full::new(Bytes::from_static(b"Unauthorized")))
			.unwrap();
		let Some(header) = header else {
			return Some(response);
		};
		if !self.check_key(header, permission) {
			return Some(response);
		}
		None
	}

	pub fn add_key(&mut self, permissions: Vec<KeyPermission>) -> String {
		let id = generate_random_key();
		let key = Key {
			id: id.clone(),
			permissions,
		};
		self.keys.push(key);
		id
	}
}

/// A key that can be used to access the remote server
#[derive(Serialize, Deserialize, Clone)]
struct Key {
	id: String,
	/// The permissions that this key has. If empty, the key has all permissions.
	permissions: Vec<KeyPermission>,
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug, clap::ValueEnum)]
#[serde(rename_all = "snake_case")]
pub enum KeyPermission {
	/// Can query the server for information
	Query,
	/// Can start/stop instances
	Run,
	/// Can edit and add instances
	Edit,
	/// Can delete instances
	Delete,
}

impl FromStr for KeyPermission {
	type Err = ();

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		match s {
			"query" => Ok(Self::Query),
			"run" => Ok(Self::Run),
			"edit" => Ok(Self::Edit),
			"delete" => Ok(Self::Delete),
			_ => Err(()),
		}
	}
}

impl Display for KeyPermission {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		let s = match self {
			Self::Query => "query",
			Self::Run => "run",
			Self::Edit => "edit",
			Self::Delete => "delete",
		};
		write!(f, "{s}")
	}
}

#[derive(Serialize, Deserialize)]
pub struct SyncResponse {
	pub instances: HashMap<String, InstanceConfig>,
	pub templates: HashMap<String, TemplateConfig>,
	pub base_template: TemplateConfig,
}

/// Generates a random access key
pub fn generate_random_key() -> String {
	let num: u128 = rand::random();

	base64::engine::GeneralPurpose::new(&base64::alphabet::URL_SAFE, GeneralPurposeConfig::new())
		.encode(num.to_ne_bytes())
		.replace("=", "")
}

/// Get the Microsoft client ID
fn get_ms_client_id() -> ClientId {
	ClientId::new(get_raw_ms_client_id().to_string())
}

const fn get_raw_ms_client_id() -> &'static str {
	if let Some(id) = option_env!("NITRO_MS_CLIENT_ID") {
		id
	} else {
		// Please don't use my client ID :)
		"402abc71-43fb-45c1-b230-e7fc9d4485fe"
	}
}
