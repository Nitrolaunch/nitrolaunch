use std::{
	collections::HashMap, fmt::Display, net::SocketAddr, path::Path, str::FromStr, sync::Arc,
};

use anyhow::Context;
use base64::{Engine, engine::GeneralPurposeConfig};
use http_body_util::{BodyExt, Full};
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
use nitro_shared::{
	UpdateDepth,
	id::{InstanceID, TemplateID},
	output::{MessageContents, NitroOutput},
	util::MakeSend,
};
use nitrolaunch::{
	config::{
		Config,
		modifications::{ConfigModification, apply_modifications_and_write},
	},
	config_crate::{
		ConfigDeser,
		instance::{InstanceConfig, QuickPlay},
		template::TemplateConfig,
	},
	instance::{
		launch::LaunchSettings,
		update::{InstanceUpdateContext, UpdateFacets, manager::UpdateSettings},
	},
	io::{lock::Lockfile, paths::Paths},
	plugin::PluginManager,
};
use serde::{Deserialize, Serialize};
use tokio::net::TcpListener;

use crate::{
	get_dir,
	output::{InputEvent, OutputEvent, RemoteOutput},
};

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
	let client = Client::new();
	let remote_out = RemoteOutput::new(paths);
	let state = State {
		client,
		paths: Arc::new(paths.clone()),
		plugins,
		settings: settings,
		o: remote_out,
	};

	o.display(MessageContents::Success("Server started".into()));

	loop {
		let Ok((stream, _)) = listener.accept().await else {
			o.display(MessageContents::Error("Failed to accept connection".into()));
			continue;
		};
		let io = TokioIo::new(stream);
		let mut o = o.get_lesser_copy();
		let state = state.clone();

		tokio::spawn(async move {
			if let Err(e) = http1::Builder::new()
				.serve_connection(
					io,
					hyper::service::service_fn(|req| handle(req, state.clone())),
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

async fn handle(req: Request<Incoming>, state: State) -> anyhow::Result<Response<Full<Bytes>>> {
	let path = req.uri().path();
	let method = req.method();
	let mut o = state.o.clone();

	o.debug(MessageContents::Simple(format!(
		"Received request: {} {}",
		method.as_str(),
		path
	)));

	let result = handle_inner(req, state).await;
	match &result {
		Ok(response) => {
			o.debug(MessageContents::Simple(format!(
				"Responding with status: {}",
				response.status()
			)));
		}
		Err(e) => {
			o.display(MessageContents::Error(format!(
				"Failed to handle request: {e:?}"
			)));
		}
	}

	result
}

async fn handle_inner(
	req: Request<Incoming>,
	state: State,
) -> anyhow::Result<Response<Full<Bytes>>> {
	let path = req.uri().path();
	let method = req.method();
	let mut o = state.o.clone();
	let key = req
		.headers()
		.get("Authorization")
		.map(|x| x.to_str().unwrap_or(""));

	if path == "/" && method == Method::GET {
		Ok(Response::builder()
			.status(200)
			.body(Full::new(Bytes::from_static(b"OK")))
			.unwrap())
	} else if path == "/sync" && method == Method::GET {
		if let Some(response) = state.settings.check_key_header(key, KeyPermission::Query) {
			return Ok(response);
		}
		match sync(state).await {
			Ok(response) => Ok(response),
			Err(e) => {
				o.display(MessageContents::Error(format!("{e:?}")));
				Ok(ise())
			}
		}
	} else if path.starts_with("/jobs/") && method == Method::GET {
		if let Some(response) = state.settings.check_key_header(key, KeyPermission::Query) {
			return Ok(response);
		}
		let Ok(job_id) = path.trim_start_matches("/jobs/").parse::<u64>() else {
			return Ok(invalid_request());
		};
		match get_job(state, job_id) {
			Ok(response) => Ok(response),
			Err(e) => {
				o.display(MessageContents::Error(format!("{e:?}")));
				Ok(ise())
			}
		}
	} else if path.starts_with("/jobs/") && path.ends_with("/input") && method == Method::POST {
		if let Some(response) = state.settings.check_key_header(key, KeyPermission::Query) {
			return Ok(response);
		}
		let Ok(job_id) = path
			.trim_start_matches("/jobs/")
			.trim_end_matches("/input")
			.parse::<u64>()
		else {
			return Ok(invalid_request());
		};
		let body = req
			.into_body()
			.collect()
			.await
			.context("Failed to collect body")?;
		let Ok(request) = serde_json::from_slice::<InputJobRequest>(&body.to_bytes()) else {
			return Ok(invalid_request());
		};
		match input_job(state, job_id, request) {
			Ok(response) => Ok(response),
			Err(e) => {
				o.display(MessageContents::Error(format!("{e:?}")));
				Ok(ise())
			}
		}
	} else if path == "/launch" && method == Method::POST {
		if let Some(response) = state.settings.check_key_header(key, KeyPermission::Run) {
			return Ok(response);
		}
		let body = req
			.into_body()
			.collect()
			.await
			.context("Failed to collect body")?;
		let Ok(request) = serde_json::from_slice::<LaunchRequest>(&body.to_bytes()) else {
			return Ok(invalid_request());
		};
		Ok(launch(state, request).await)
	} else if path == "/update" && method == Method::POST {
		if let Some(response) = state.settings.check_key_header(key, KeyPermission::Update) {
			return Ok(response);
		}
		let body = req
			.into_body()
			.collect()
			.await
			.context("Failed to collect body")?;
		let Ok(request) = serde_json::from_slice::<UpdateRequest>(&body.to_bytes()) else {
			return Ok(invalid_request());
		};
		Ok(update(state, request).await)
	} else if path.starts_with("/instances/")
		&& path.ends_with("/configure")
		&& method == Method::POST
	{
		if let Some(response) = state.settings.check_key_header(key, KeyPermission::Edit) {
			return Ok(response);
		}
		let id = path
			.trim_start_matches("/instances/")
			.trim_end_matches("/configure")
			.to_string();
		let body = req
			.into_body()
			.collect()
			.await
			.context("Failed to collect body")?;
		let Ok(request) = serde_json::from_slice::<InstanceConfig>(&body.to_bytes()) else {
			return Ok(invalid_request());
		};
		match configure_instance(state, &id, request).await {
			Ok(response) => Ok(response),
			Err(e) => {
				o.display(MessageContents::Error(format!("{e:?}")));
				Ok(ise())
			}
		}
	} else if path.starts_with("/templates/")
		&& path.ends_with("/configure")
		&& method == Method::POST
	{
		if let Some(response) = state.settings.check_key_header(key, KeyPermission::Edit) {
			return Ok(response);
		}
		let id = path
			.trim_start_matches("/templates/")
			.trim_end_matches("/configure")
			.to_string();
		let body = req
			.into_body()
			.collect()
			.await
			.context("Failed to collect body")?;
		let Ok(request) = serde_json::from_slice::<TemplateConfig>(&body.to_bytes()) else {
			return Ok(invalid_request());
		};
		match configure_template(state, &id, request).await {
			Ok(response) => Ok(response),
			Err(e) => {
				o.display(MessageContents::Error(format!("{e:?}")));
				Ok(ise())
			}
		}
	} else {
		Ok(Response::builder()
			.status(404)
			.body(Full::new(Bytes::from_static(b"Not Found")))
			.unwrap())
	}
}

async fn sync(mut state: State) -> anyhow::Result<Response<Full<Bytes>>> {
	let config = state.config().await?;
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
}

fn get_job(state: State, job_id: u64) -> anyhow::Result<Response<Full<Bytes>>> {
	if let Some(job) = state.o.get_job(job_id) {
		json_response(&GetJobResponse {
			id: job.id,
			events: job.events.into_iter().collect(),
			is_finished: job.is_finished,
		})
	} else {
		Ok(not_found())
	}
}

fn input_job(
	state: State,
	job_id: u64,
	request: InputJobRequest,
) -> anyhow::Result<Response<Full<Bytes>>> {
	if state.o.get_job(job_id).is_some() {
		state.o.send_input(request.event, job_id);
		Ok(Response::builder()
			.status(200)
			.body(Full::new(Bytes::from_static(b"OK")))
			.unwrap())
	} else {
		Ok(not_found())
	}
}

async fn launch(mut state: State, request: LaunchRequest) -> Response<Full<Bytes>> {
	state.o.new_job();
	let job_id = state.o.job_id().unwrap();
	let o = state.o.clone();
	let task = async move {
		let mut config = state.config().await?;
		let core = config
			.get_core(
				Some(&get_ms_client_id()),
				&UpdateSettings {
					depth: UpdateDepth::Shallow,
					offline_auth: request.offline,
				},
				&state.client,
				&config.plugins,
				&state.paths,
				&mut state.o,
			)
			.await?;

		let instance = config
			.instances
			.get_mut(&InstanceID::from(request.instance))
			.context("Instance does not exist")?;

		let settings = LaunchSettings {
			offline_auth: request.offline,
			pipe_stdin: false,
			quick_play: None,
		};

		let mut lock = Lockfile::open(&state.paths)?;
		let mut ctx = InstanceUpdateContext {
			packages: &config.packages,
			accounts: &mut config.accounts,
			plugins: &config.plugins,
			prefs: &config.prefs,
			paths: &state.paths,
			lock: &mut lock,
			client: &state.client,
			output: &mut state.o,
			core: &core,
		};

		let mut handle = instance
			.launch(settings, &mut ctx)
			.await
			.context("Failed to launch instance")?;

		handle.silence_output(true);
		state.o.finish();

		handle
			.wait(&config.plugins, &state.paths, &mut state.o)
			.await?;

		Ok::<(), anyhow::Error>(())
	};
	let task = unsafe { MakeSend::new(task) };

	tokio::spawn(async move {
		if let Err(e) = task.await {
			o.log(MessageContents::Error(format!(
				"Failed to launch instance: {e:?}"
			)));
		}
	});

	Response::builder()
		.status(200)
		.body(Full::new(Bytes::from(job_id.to_string())))
		.unwrap()
}

async fn update(mut state: State, request: UpdateRequest) -> Response<Full<Bytes>> {
	state.o.new_job();
	let job_id = state.o.job_id().unwrap();
	let o = state.o.clone();
	let task = async move {
		let mut config = state.config().await?;
		let core = config
			.get_core(
				Some(&get_ms_client_id()),
				&UpdateSettings {
					depth: request.depth,
					offline_auth: false,
				},
				&state.client,
				&config.plugins,
				&state.paths,
				&mut state.o,
			)
			.await?;

		let instance = config
			.instances
			.get_mut(&InstanceID::from(request.instance))
			.context("Instance does not exist")?;

		let mut lock = Lockfile::open(&state.paths)?;
		let mut ctx = InstanceUpdateContext {
			packages: &config.packages,
			accounts: &mut config.accounts,
			plugins: &config.plugins,
			prefs: &config.prefs,
			paths: &state.paths,
			lock: &mut lock,
			client: &state.client,
			output: &mut state.o,
			core: &core,
		};
		let facets = UpdateFacets {
			instance: request.update_instance,
			packages: request.update_packages,
			modpack: request.update_modpack,
		};

		instance.update(request.depth, facets, &mut ctx).await?;

		Ok::<(), anyhow::Error>(())
	};
	let task = unsafe { MakeSend::new(task) };

	tokio::spawn(async move {
		if let Err(e) = task.await {
			o.log(MessageContents::Error(format!(
				"Failed to update instance: {e:?}"
			)));
		}
	});

	Response::builder()
		.status(200)
		.body(Full::new(Bytes::from(job_id.to_string())))
		.unwrap()
}

async fn configure_instance(
	mut state: State,
	id: &str,
	instance_config: InstanceConfig,
) -> anyhow::Result<Response<Full<Bytes>>> {
	let config = state.config().await?;
	let mut raw_config = state.raw_config().await?;
	let id = InstanceID::from(id);
	let is_new = !config.instances.contains_key(&id);
	let modification = if is_new {
		ConfigModification::AddInstance(id, instance_config)
	} else {
		ConfigModification::UpdateInstance(id, instance_config)
	};
	apply_modifications_and_write(
		&mut raw_config,
		vec![modification],
		&state.paths,
		&state.plugins,
		&mut state.o,
	)
	.await?;

	Response::builder()
		.status(200)
		.body(Full::new(Bytes::from("OK")))
		.context("Failed to build response")
}

async fn configure_template(
	mut state: State,
	id: &str,
	template_config: TemplateConfig,
) -> anyhow::Result<Response<Full<Bytes>>> {
	let config = state.config().await?;
	let mut raw_config = state.raw_config().await?;
	let id = TemplateID::from(id);
	let is_new = !config.templates.contains_key(&id);
	let modification = if is_new {
		ConfigModification::AddTemplate(id, template_config)
	} else {
		ConfigModification::UpdateTemplate(id, template_config)
	};
	apply_modifications_and_write(
		&mut raw_config,
		vec![modification],
		&state.paths,
		&state.plugins,
		&mut state.o,
	)
	.await?;

	Response::builder()
		.status(200)
		.body(Full::new(Bytes::from("OK")))
		.context("Failed to build response")
}

#[derive(Clone)]
struct State {
	client: Client,
	paths: Arc<Paths>,
	plugins: PluginManager,
	settings: Arc<RemoteSettings>,
	o: RemoteOutput,
}

impl State {
	async fn config(&mut self) -> anyhow::Result<Config> {
		let client_id = get_ms_client_id();
		let config = Config::load(
			&Config::get_path(&self.paths),
			self.plugins.clone(),
			false,
			&self.paths,
			client_id,
			&mut self.o,
		)
		.await?;
		Ok(config)
	}

	async fn raw_config(&mut self) -> anyhow::Result<ConfigDeser> {
		Config::open(&Config::get_path(&self.paths))
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

fn not_found() -> Response<Full<Bytes>> {
	Response::builder()
		.status(404)
		.body(Full::new(Bytes::from_static(b"Not Found")))
		.unwrap()
}

fn ise() -> Response<Full<Bytes>> {
	Response::builder()
		.status(500)
		.body(Full::new(Bytes::from_static(b"Internal Server Error")))
		.unwrap()
}

fn invalid_request() -> Response<Full<Bytes>> {
	Response::builder()
		.status(400)
		.body(Full::new(Bytes::from_static(b"Invalid Request")))
		.unwrap()
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
	/// Can update instances
	Update,
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
			"update" => Ok(Self::Update),
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
			Self::Update => "update",
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

#[derive(Serialize, Deserialize)]
pub struct GetJobResponse {
	pub id: u64,
	pub events: Vec<OutputEvent>,
	pub is_finished: bool,
}

#[derive(Serialize, Deserialize)]
pub struct InputJobRequest {
	pub event: InputEvent,
}

#[derive(Serialize, Deserialize)]
pub struct LaunchRequest {
	pub instance: String,
	pub account: Option<String>,
	pub quick_play: QuickPlay,
	pub offline: bool,
}

#[derive(Serialize, Deserialize)]
pub struct UpdateRequest {
	pub instance: String,
	pub depth: UpdateDepth,
	pub update_instance: bool,
	pub update_packages: bool,
	pub update_modpack: bool,
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
