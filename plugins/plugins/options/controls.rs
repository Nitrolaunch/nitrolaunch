use nitro_plugin::control::{Control, ControlSchema};
use serde_json::json;

pub fn all_client_options() -> Vec<Control> {
	let mut out = client_options();
	out.extend(control_options());
	out.extend(key_options());
	out.extend(chat_options());
	out.extend(video_options());
	out.extend(sound_options());

	out
}

pub fn all_server_options() -> Vec<Control> {
	let mut out = server_options();
	out.extend(control_options());
	out.extend(rcon_options());
	out.extend(query_options());
	out.extend(whitelist_options());
	out.extend(datapacks_options());
	out.extend(world_options());
	out.extend(resource_pack_options());
	out.extend(management_options());

	out
}

pub fn universal_options() -> Vec<Control> {
	serde_json::from_value(json!([
		{ "id": "client_options", "name": "Client Options", "icon": "controller", "schema": { "type": "section" } },
		{ "id": "server_options", "name": "Server Options", "icon": "server", "schema": { "type": "section" } },
		{ "id": "control_options", "name": "Control Options", "icon": "controller", "schema": { "type": "section" } },
		{ "id": "keybinds", "name": "Keybinds", "icon": "font", "schema": { "type": "section" } },
		{ "id": "chat_options", "name": "Chat Options", "icon": "speech_bubble", "schema": { "type": "section" } },
		{ "id": "video_options", "name": "Video Options", "icon": "picture", "schema": { "type": "section" } },
		{ "id": "sound_options", "name": "Sound Options", "icon": "audio", "schema": { "type": "section" } },
		{ "id": "rcon_options", "name": "RCON Options", "icon": "play", "schema": { "type": "section" } },
		{ "id": "server_query_options", "name": "Query Options", "icon": "globe", "schema": { "type": "section" } },
		{ "id": "whitelist_options", "name": "Whitelist Options", "icon": "filter", "schema": { "type": "section" } },
		{ "id": "server_world_options", "name": "World Options", "icon": "sun", "schema": { "type": "section" } },
		{ "id": "server_datapack_options", "name": "Datapacks Options", "icon": "curly_braces", "schema": { "type": "section" } },
		{ "id": "server_resource_pack_options", "name": "Server Resource Options", "icon": "palette", "schema": { "type": "section" } },
		{ "id": "server_management_options", "name": "Server Management Options", "icon": "sword", "schema": { "type": "section" } }
	]))
	.unwrap()
}

fn client_options() -> Vec<Control> {
	serde_json::from_value(json!([
		{ "id": "realms_notifications", "name": "Realms Notifications", "section": "client_options", "schema": { "type": "boolean" } },
		{ "id": "reduced_debug_info", "name": "Reduced Debug Info", "section": "client_options", "schema": { "type": "boolean" } },
		{ "id": "skip_multiplayer_warning", "name": "Skip Multiplayer Warning", "section": "client_options", "schema": { "type": "boolean" } },
		{ "id": "skip_realms_32_bit_warning", "name": "Skip Realms 32-bit Warning", "section": "client_options", "schema": { "type": "boolean" } },
		{ "id": "hide_bundle_tutorial", "name": "Hide Bundle Tutorial", "section": "client_options", "schema": { "type": "boolean" } },
		{ "id": "joined_server", "name": "Joined Server Before", "section": "client_options", "schema": { "type": "boolean" } },
		{ "id": "sync_chunk_writes", "name": "Sync Chunk Writes", "section": "client_options", "schema": { "type": "boolean" } },
		{ "id": "use_native_transport", "name": "Use Native Transport", "section": "client_options", "schema": { "type": "boolean" } },
		{ "id": "held_item_tooltips", "name": "Held Item Tooltips", "section": "client_options", "schema": { "type": "boolean" } },
		{ "id": "advanced_item_tooltips", "name": "Advanced Item Tooltips", "section": "client_options", "schema": { "type": "boolean" } },
		{ "id": "hide_matched_names", "name": "Hide Matched Names", "section": "client_options", "schema": { "type": "boolean" } },
		{ "id": "pause_on_lost_focus", "name": "Pause On Lost Focus", "section": "client_options", "schema": { "type": "boolean" } },
		{ "id": "hide_server_address", "name": "Hide Server Address", "section": "client_options", "schema": { "type": "boolean" } },
		{ "id": "show_autosave_indicator", "name": "Show Autosave Indicator", "section": "client_options", "schema": { "type": "boolean" } },
		{ "id": "allow_server_listing", "name": "Allow Server Listing", "section": "client_options", "schema": { "type": "boolean" } },
		{ "id": "snooper_enabled", "name": "Allow Snooper / Telemetry", "section": "client_options", "schema": { "type": "boolean" } },
		{
			"id": "difficulty",
			"name": "Difficulty",
			"section": "client_options",
			"schema": difficulty()
		},
		{
			"id": "log_level",
			"name": "Log Level",
			"section": "client_options",
			"schema": log_level()
		},
		{
			"id": "main_hand",
			"name": "Main Hand",
			"section": "client_options",
			"schema": main_hand()
		},
		{
			"id": "tutorial_step",
			"name": "Tutorial Step",
			"section": "client_options",
			"schema": tutorial_step()
		},
		{
			"id": "language",
			"name": "Language",
			"section": "client_options",
			"schema": { "type": "string" }
		},
		{
			"id": "resource_packs",
			"name": "Resource Packs",
			"section": "client_options",
			"schema": { "type": "string_list" }
		},
		{
			"id": "custom",
			"name": "Custom Options",
			"section": "client_options",
			"schema": {
				"type": "json"
			}
		}
	]))
	.unwrap()
}

fn control_options() -> Vec<Control> {
	serde_json::from_value(json!([
		{ "id": "control.auto_jump", "name": "Auto Jump", "section": "control_options", "schema": { "type": "boolean" } },
		{ "id": "control.discrete_mouse_scroll", "name": "Discrete Mouse Scroll", "section": "control_options", "schema": { "type": "boolean" } },
		{ "id": "control.invert_mouse_y", "name": "Invert Mouse Y", "section": "control_options", "schema": { "type": "boolean" } },
		{ "id": "control.enable_touchscreen", "name": "Enable Touchscreen", "section": "control_options", "schema": { "type": "boolean" } },
		{ "id": "control.toggle_sprint", "name": "Toggle Sprint", "section": "control_options", "schema": { "type": "boolean" } },
		{ "id": "control.toggle_crouch", "name": "Toggle Crouch", "section": "control_options", "schema": { "type": "boolean" } },
		{ "id": "control.raw_mouse_input", "name": "Raw Mouse Input", "section": "control_options", "schema": { "type": "boolean" } },
		{
			"id": "control.mouse_sensitivity",
			"name": "Mouse Sensitivity",
			"section": "control_options",
			"schema": { "type": "number", "min": 0, "max": 200, "step": 1, "slider": true }
		},
		{
			"id": "control.mouse_wheel_sensitivity",
			"name": "Mouse Wheel Sensitivity",
			"section": "control_options",
			"schema": { "type": "number", "min": 0.0, "max": 10.0, "step": 0.1, "slider": true }
		}
	]))
	.unwrap()
}

fn key_options() -> Vec<Control> {
	serde_json::from_value(json!([
		{ "id": "control.keys.forward", "name": "Move Forward", "section": "keybinds", "schema": { "type": "keybind" } },
		{ "id": "control.keys.left", "name": "Move Left", "section": "keybinds", "schema": { "type": "keybind" } },
		{ "id": "control.keys.back", "name": "Move Back", "section": "keybinds", "schema": { "type": "keybind" } },
		{ "id": "control.keys.right", "name": "Move Right", "section": "keybinds", "schema": { "type": "keybind" } },
		{ "id": "control.keys.jump", "name": "Jump", "section": "keybinds", "schema": { "type": "keybind" } },
		{ "id": "control.keys.sneak", "name": "Sneak", "section": "keybinds", "schema": { "type": "keybind" } },
		{ "id": "control.keys.sprint", "name": "Sprint", "section": "keybinds", "schema": { "type": "keybind" } },
		{ "id": "control.keys.attack", "name": "Attack / Destroy", "section": "keybinds", "schema": { "type": "keybind" } },
		{ "id": "control.keys.use", "name": "Use Item / Place Block", "section": "keybinds", "schema": { "type": "keybind" } },
		{ "id": "control.keys.drop", "name": "Drop Item", "section": "keybinds", "schema": { "type": "keybind" } },
		{ "id": "control.keys.pick_item", "name": "Pick Block", "section": "keybinds", "schema": { "type": "keybind" } },
		{ "id": "control.keys.swap_offhand", "name": "Swap Offhand", "section": "keybinds", "schema": { "type": "keybind" } },
		{ "id": "control.keys.inventory", "name": "Open Inventory", "section": "keybinds", "schema": { "type": "keybind" } },
		{ "id": "control.keys.advancements", "name": "Advancements", "section": "keybinds", "schema": { "type": "keybind" } },
		{ "id": "control.keys.chat", "name": "Open Chat", "section": "keybinds", "schema": { "type": "keybind" } },
		{ "id": "control.keys.command", "name": "Command", "section": "keybinds", "schema": { "type": "keybind" } },
		{ "id": "control.keys.playerlist", "name": "Player List", "section": "keybinds", "schema": { "type": "keybind" } },
		{ "id": "control.keys.social_interactions", "name": "Social Interactions", "section": "keybinds", "schema": { "type": "keybind" } },
		{ "id": "control.keys.toggle_perspective", "name": "Toggle Perspective", "section": "keybinds", "schema": { "type": "keybind" } },
		{ "id": "control.keys.smooth_camera", "name": "Smooth Camera", "section": "keybinds", "schema": { "type": "keybind" } },
		{ "id": "control.keys.fullscreen", "name": "Toggle Fullscreen", "section": "keybinds", "schema": { "type": "keybind" } },
		{ "id": "control.keys.spectator_outlines", "name": "Spectator Outlines", "section": "keybinds", "schema": { "type": "keybind" } },
		{ "id": "control.keys.screenshot", "name": "Take Screenshot", "section": "keybinds", "schema": { "type": "keybind" } },
		{ "id": "control.keys.hotbar_1", "name": "Hotbar Slot 1", "section": "keybinds", "schema": { "type": "keybind" } },
		{ "id": "control.keys.hotbar_2", "name": "Hotbar Slot 2", "section": "keybinds", "schema": { "type": "keybind" } },
		{ "id": "control.keys.hotbar_3", "name": "Hotbar Slot 3", "section": "keybinds", "schema": { "type": "keybind" } },
		{ "id": "control.keys.hotbar_4", "name": "Hotbar Slot 4", "section": "keybinds", "schema": { "type": "keybind" } },
		{ "id": "control.keys.hotbar_5", "name": "Hotbar Slot 5", "section": "keybinds", "schema": { "type": "keybind" } },
		{ "id": "control.keys.hotbar_6", "name": "Hotbar Slot 6", "section": "keybinds", "schema": { "type": "keybind" } },
		{ "id": "control.keys.hotbar_7", "name": "Hotbar Slot 7", "section": "keybinds", "schema": { "type": "keybind" } },
		{ "id": "control.keys.hotbar_8", "name": "Hotbar Slot 8", "section": "keybinds", "schema": { "type": "keybind" } },
		{ "id": "control.keys.hotbar_9", "name": "Hotbar Slot 9", "section": "keybinds", "schema": { "type": "keybind" } },
		{ "id": "control.keys.save_toolbar", "name": "Save Toolbar", "section": "keybinds", "schema": { "type": "keybind" } },
		{ "id": "control.keys.load_toolbar", "name": "Load Toolbar", "section": "keybinds", "schema": { "type": "keybind" } },
		{ "id": "control.keys.boss_mode", "name": "Toggle Boss Mode", "section": "keybinds", "schema": { "type": "keybind" } },
		{ "id": "control.keys.decrease_view", "name": "Decrease View Distance", "section": "keybinds", "schema": { "type": "keybind" } },
		{ "id": "control.keys.increase_view", "name": "Increase View Distance", "section": "keybinds", "schema": { "type": "keybind" } },
		{ "id": "control.keys.stream_commercial", "name": "Stream Commercial", "section": "keybinds", "schema": { "type": "keybind" } },
		{ "id": "control.keys.stream_pause_unpause", "name": "Pause / Unpause Stream", "section": "keybinds", "schema": { "type": "keybind" } },
		{ "id": "control.keys.stream_start_stop", "name": "Start / Stop Stream", "section": "keybinds", "schema": { "type": "keybind" } },
		{ "id": "control.keys.stream_toggle_microphone", "name": "Toggle Microphone", "section": "keybinds", "schema": { "type": "keybind" } }
	]))
	.unwrap()
}

fn chat_options() -> Vec<Control> {
	serde_json::from_value(json!([
		{ "id": "chat.auto_command_suggestions", "name": "Auto Command Suggestions", "section": "chat_options", "schema": { "type": "boolean" } },
		{ "id": "chat.enable_colors", "name": "Enable Colors", "section": "chat_options", "schema": { "type": "boolean" } },
		{ "id": "chat.enable_links", "name": "Enable Links", "section": "chat_options", "schema": { "type": "boolean" } },
		{ "id": "chat.prompt_links", "name": "Prompt Links", "section": "chat_options", "schema": { "type": "boolean" } },
		{ "id": "chat.force_unicode", "name": "Force Unicode Font", "section": "chat_options", "schema": { "type": "boolean" } },
		{ "id": "chat.background_for_chat_only", "name": "Chat-Only Background", "section": "chat_options", "schema": { "type": "boolean" } },
		{
			"id": "chat.visibility",
			"name": "Chat Visibility",
			"section": "chat_options",
			"schema": chat_visibility()
		},
		{
			"id": "chat.narrator_mode",
			"name": "Narrator Mode",
			"section": "chat_options",
			"schema": narrator_mode()
		},
		{
			"id": "chat.opacity",
			"name": "Chat Opacity",
			"section": "chat_options",
			"schema": { "type": "number", "min": 0.0, "max": 1.0, "step": 0.01, "slider": true }
		},
		{
			"id": "chat.line_spacing",
			"name": "Line Spacing",
			"section": "chat_options",
			"schema": { "type": "number", "min": 0.0, "max": 1.0, "step": 0.01, "slider": true }
		},
		{
			"id": "chat.background_opacity",
			"name": "Background Opacity",
			"section": "chat_options",
			"schema": { "type": "number", "min": 0.0, "max": 1.0, "step": 0.01, "slider": true }
		},
		{
			"id": "chat.focused_height",
			"name": "Focused Height",
			"section": "chat_options",
			"schema": { "type": "number", "min": 0.0, "max": 1.0, "step": 0.01, "slider": true }
		},
		{
			"id": "chat.unfocused_height",
			"name": "Unfocused Height",
			"section": "chat_options",
			"schema": { "type": "number", "min": 0.0, "max": 1.0, "step": 0.01, "slider": true }
		},
		{
			"id": "chat.delay",
			"name": "Chat Delay",
			"section": "chat_options",
			"schema": { "type": "number", "min": 0.0, "max": 10.0, "step": 0.1, "slider": true }
		},
		{
			"id": "chat.scale",
			"name": "Chat Scale",
			"section": "chat_options",
			"schema": { "type": "number", "min": 0.0, "max": 1.0, "step": 0.01, "slider": true }
		},
		{
			"id": "chat.width",
			"name": "Chat Width",
			"section": "chat_options",
			"schema": { "type": "number", "min": 0.0, "max": 1.0, "step": 0.01, "slider": true }
		}
	]))
	.unwrap()
}

fn video_options() -> Vec<Control> {
	serde_json::from_value(json!([
		{ "id": "video.vsync", "name": "VSync", "section": "video_options", "schema": { "type": "boolean" } },
		{ "id": "video.entity_shadows", "name": "Entity Shadows", "section": "video_options", "schema": { "type": "boolean" } },
		{ "id": "video.fullscreen", "name": "Fullscreen", "section": "video_options", "schema": { "type": "boolean" } },
		{ "id": "video.view_bobbing", "name": "View Bobbing", "section": "video_options", "schema": { "type": "boolean" } },
		{ "id": "video.dark_mojang_background", "name": "Dark Mojang Background", "section": "video_options", "schema": { "type": "boolean" } },
		{ "id": "video.hide_lightning_flashes", "name": "Hide Lightning Flashes", "section": "video_options", "schema": { "type": "boolean" } },
		{ "id": "video.smooth_lighting", "name": "Smooth Lighting", "section": "video_options", "schema": { "type": "boolean" } },
		{ "id": "video.allow_block_alternatives", "name": "Allow Block Alternatives", "section": "video_options", "schema": { "type": "boolean" } },
		{
			"id": "video.fov",
			"name": "Field of View",
			"section": "video_options",
			"schema": { "type": "number", "min": 30, "max": 110, "step": 1, "slider": true }
		},
		{
			"id": "video.screen_effect_scale",
			"name": "Screen Effect Scale",
			"section": "video_options",
			"schema": { "type": "number", "min": 0.0, "max": 1.0, "step": 0.01, "slider": true }
		},
		{
			"id": "video.fov_effect_scale",
			"name": "FOV Effect Scale",
			"section": "video_options",
			"schema": { "type": "number", "min": 0.0, "max": 1.0, "step": 0.01, "slider": true }
		},
		{
			"id": "video.darkness_effect_scale",
			"name": "Darkness Effect Scale",
			"section": "video_options",
			"schema": { "type": "number", "min": 0.0, "max": 1.0, "step": 0.01, "slider": true }
		},
		{
			"id": "video.brightness",
			"name": "Brightness",
			"section": "video_options",
			"schema": { "type": "number", "min": 0.0, "max": 1.0, "step": 0.01, "slider": true }
		},
		{
			"id": "video.render_distance",
			"name": "Render Distance",
			"section": "video_options",
			"schema": { "type": "number", "min": 2, "max": 32, "step": 1, "slider": true }
		},
		{
			"id": "video.simulation_distance",
			"name": "Simulation Distance",
			"section": "video_options",
			"schema": { "type": "number", "min": 2, "max": 32, "step": 1, "slider": true }
		},
		{
			"id": "video.entity_distance_scaling",
			"name": "Entity Distance Scaling",
			"section": "video_options",
			"schema": { "type": "number", "min": 0.5, "max": 5.0, "step": 0.1, "slider": true }
		},
		{
			"id": "video.gui_scale",
			"name": "GUI Scale",
			"section": "video_options",
			"schema": { "type": "number", "min": 0, "max": 6, "step": 1, "slider": true }
		},
		{
			"id": "video.max_fps",
			"name": "Max FPS",
			"section": "video_options",
			"schema": { "type": "number", "min": 10, "max": 260, "step": 1, "slider": true }
		},
		{
			"id": "video.biome_blend",
			"name": "Biome Blend",
			"section": "video_options",
			"schema": { "type": "number", "min": 0, "max": 7, "step": 1 }
		},
		{
			"id": "video.mipmap_levels",
			"name": "Mipmap Levels",
			"section": "video_options",
			"schema": { "type": "number", "min": 0, "max": 4, "step": 1 }
		},
		{
			"id": "video.window_width",
			"name": "Window Width",
			"section": "video_options",
			"schema": { "type": "number", "min": 320, "max": 7680, "step": 1 }
		},
		{
			"id": "video.window_height",
			"name": "Window Height",
			"section": "video_options",
			"schema": { "type": "number", "min": 240, "max": 4320, "step": 1 }
		},
		{
			"id": "video.particles",
			"name": "Particles",
			"section": "video_options",
			"schema": particles_mode()
		},
		{
			"id": "video.graphics_mode",
			"name": "Graphics Mode",
			"section": "video_options",
			"schema": graphics_mode()
		},
		{
			"id": "video.chunk_updates_mode",
			"name": "Chunk Updates Mode",
			"section": "video_options",
			"schema": chunk_updates_mode()
		},
		{
			"id": "video.clouds",
			"name": "Clouds",
			"section": "video_options",
			"schema": cloud_render_mode()
		},
		{
			"id": "video.attack_indicator",
			"name": "Attack Indicator",
			"section": "video_options",
			"schema": attack_indicator_mode()
		},
	]))
	.unwrap()
}

fn sound_options() -> Vec<Control> {
	let volume = volume_options();

	let mut options: Vec<Control> = serde_json::from_value(json!([
		{
			"id": "sound.device",
			"name": "Device",
			"section": "sound_options",
			"schema": {
				"type": "string"
			}
		},
		{
			"id": "sound.show_subtitles",
			"name": "Show Subtitles",
			"section": "sound_options",
			"schema": {
				"type": "boolean"
			}
		},
		{
			"id": "sound.directional_audio",
			"name": "Directional Audio",
			"section": "sound_options",
			"schema": {
				"type": "boolean"
			}
		},
	]))
	.unwrap();

	options.extend(volume);
	options
}

fn volume_options() -> Vec<Control> {
	serde_json::from_value(json!([
		{
			"id": "sound.volume.master",
			"name": "Master Volume",
			"section": "sound_options",
			"schema": volume(),
		},
		{
			"id": "sound.volume.music",
			"name": "Music Volume",
			"section": "sound_options",
			"schema": volume(),
		},
		{
			"id": "sound.volume.record",
			"name": "Record Volume",
			"section": "sound_options",
			"schema": volume(),
		},
		{
			"id": "sound.volume.weather",
			"name": "Weather Volume",
			"section": "sound_options",
			"schema": volume(),
		},
		{
			"id": "sound.volume.block",
			"name": "Block Volume",
			"section": "sound_options",
			"schema": volume(),
		},
		{
			"id": "sound.volume.hostile",
			"name": "Hostile Volume",
			"section": "sound_options",
			"schema": volume(),
		},
		{
			"id": "sound.volume.neutral",
			"name": "Neutral Volume",
			"section": "sound_options",
			"schema": volume(),
		},
		{
			"id": "sound.volume.player",
			"name": "Player Volume",
			"section": "sound_options",
			"schema": volume(),
		},
		{
			"id": "sound.volume.ambient",
			"name": "Ambient Volume",
			"section": "sound_options",
			"schema": volume(),
		},
		{
			"id": "sound.volume.voice",
			"name": "Voice Volume",
			"section": "sound_options",
			"schema": volume(),
		},
	]))
	.unwrap()
}

fn server_options() -> Vec<Control> {
	serde_json::from_value(json!([
		{ "id": "allow_flight", "name": "Allow Flight", "section": "server_options", "schema": { "type": "boolean" } },
		{ "id": "broadcast_console_to_ops", "name": "Broadcast Console to Ops", "section": "server_options", "schema": { "type": "boolean" } },
		{ "id": "broadcast_rcon_to_ops", "name": "Broadcast rcon_options to Ops", "section": "server_options", "schema": { "type": "boolean" } },
		{ "id": "allow_command_blocks", "name": "Allow Command Blocks", "section": "server_options", "schema": { "type": "boolean" } },
		{ "id": "jmx_monitoring", "name": "JMX Monitoring", "section": "server_options", "schema": { "type": "boolean" } },
		{ "id": "enable_status", "name": "Enable Status", "section": "server_options", "schema": { "type": "boolean" } },
		{ "id": "enforce_secure_profile", "name": "Enforce Secure Profile", "section": "server_options", "schema": { "type": "boolean" } },
		{ "id": "hardcore", "name": "Hardcore Mode", "section": "server_options", "schema": { "type": "boolean" } },
		{ "id": "hide_online_players", "name": "Hide Online Players", "section": "server_options", "schema": { "type": "boolean" } },
		{ "id": "offline_mode", "name": "Offline Mode", "section": "server_options", "schema": { "type": "boolean" } },
		{ "id": "prevent_proxy_connections", "name": "Prevent Proxy Connections", "section": "server_options", "schema": { "type": "boolean" } },
		{ "id": "enable_chat_preview", "name": "Enable Chat Preview", "section": "server_options", "schema": { "type": "boolean" } },
		{ "id": "enable_pvp", "name": "Enable PvP", "section": "server_options", "schema": { "type": "boolean" } },
		{ "id": "enable_snooper", "name": "Enable Snooper", "section": "server_options", "schema": { "type": "boolean" } },
		{ "id": "spawn_animals", "name": "Spawn Animals", "section": "server_options", "schema": { "type": "boolean" } },
		{ "id": "spawn_monsters", "name": "Spawn Monsters", "section": "server_options", "schema": { "type": "boolean" } },
		{ "id": "spawn_npcs", "name": "Spawn NPCs", "section": "server_options", "schema": { "type": "boolean" } },
		{ "id": "sync_chunk_writes", "name": "Sync Chunk Writes", "section": "server_options", "schema": { "type": "boolean" } },
		{ "id": "use_native_transport", "name": "Use Native Transport", "section": "server_options", "schema": { "type": "boolean" } },
		{ "id": "enable_code_of_conduct", "name": "Enable Code of Conduct", "section": "server_options", "schema": { "type": "boolean" } },
		{ "id": "entity_broadcast_range", "name": "Entity Broadcast Range", "section": "server_options", "schema": { "type": "number", "min": 0, "max": 1024, "step": 1 } },
		{ "id": "max_chained_neighbor_updates", "name": "Max Chained Neighbor Updates", "section": "server_options", "schema": { "type": "number", "min": -1, "max": 1000000, "step": 1 } },
		{ "id": "max_players", "name": "Max Players", "section": "server_options", "schema": { "type": "number", "min": 1, "max": 1000, "step": 1 } },
		{ "id": "max_tick_time", "name": "Max Tick Time (ms)", "section": "server_options", "schema": { "type": "number", "min": -1, "max": 600000, "step": 1 } },
		{ "id": "op_permission_level", "name": "OP Permission Level", "section": "server_options", "schema": { "type": "number", "min": 1, "max": 4, "step": 1 } },
		{ "id": "player_idle_timeout", "name": "Player Idle Timeout", "section": "server_options", "schema": { "type": "number", "min": 0, "max": 100000, "step": 1 } },
		{ "id": "rate_limit", "name": "Rate Limit", "section": "server_options", "schema": { "type": "number", "min": 0, "max": 10000, "step": 1 } },
		{ "id": "port", "name": "Server Port", "section": "server_options", "schema": { "type": "number", "min": 1, "max": 65535, "step": 1 } },
		{ "id": "simulation_distance", "name": "Simulation Distance", "section": "server_options", "schema": { "type": "number", "min": 2, "max": 32, "step": 1 } },
		{ "id": "spawn_protection", "name": "Spawn Protection Radius", "section": "server_options", "schema": { "type": "number", "min": 0, "max": 100, "step": 1 } },
		{ "id": "view_distance", "name": "View Distance", "section": "server_options", "schema": { "type": "number", "min": 2, "max": 32, "step": 1 } },
		{ "id": "motd", "name": "Message of the Day", "section": "server_options", "schema": { "type": "string" } },
		{ "id": "ip", "name": "Server IP", "section": "server_options", "schema": { "type": "string" } },
		{ "id": "gamemode.default", "name": "Default Gamemode", "section": "server_options", "schema": gamemode() },
		{ "id": "gamemode.force", "name": "Force Gamemode", "section": "server_options", "schema": { "type": "boolean" } },
		{ "id": "network_compression_threshold", "name": "Network Compression", "section": "server_options", "schema": network_compression() },
		{
			"id": "custom",
			"name": "Custom Options",
			"section": "server_options",
			"schema": {
				"type": "json"
			}
		}
	]))
	.unwrap()
}

fn rcon_options() -> Vec<Control> {
	serde_json::from_value(json!([
		{ "id": "rcon.enable", "name": "Enable", "section": "rcon_options", "schema": { "type": "boolean" } },
		{ "id": "rcon.port", "name": "Port", "section": "rcon_options", "schema": { "type": "number", "min": 1, "max": 65535, "step": 1 } },
		{ "id": "rcon.password", "name": "Password", "section": "rcon_options", "schema": { "type": "string" } }
	])).unwrap()
}

fn query_options() -> Vec<Control> {
	serde_json::from_value(json!([
		{ "id": "query.enable", "name": "Enable", "section": "server_query_options", "schema": { "type": "boolean" } },
		{ "id": "query.port", "name": "Port", "section": "server_query_options", "schema": { "type": "number", "min": 1, "max": 65535, "step": 1 } }
	])).unwrap()
}

fn whitelist_options() -> Vec<Control> {
	serde_json::from_value(json!([
		{ "id": "whitelist.enable", "name": "Enable", "section": "whitelist_options", "schema": { "type": "boolean" } },
		{ "id": "whitelist.enforce", "name": "Enforce", "section": "whitelist_options", "schema": { "type": "boolean" } }
	])).unwrap()
}

fn datapacks_options() -> Vec<Control> {
	serde_json::from_value(json!([
		{ "id": "datapacks.function_permission_level", "name": "Function Permission Level", "section": "server_datapack_options", "schema": { "type": "number", "min": 1, "max": 4, "step": 1 } },
		{ "id": "datapacks.initial_enabled", "name": "Initially Enabled", "section": "server_datapack_options", "schema": { "type": "string_list" } },
		{ "id": "datapacks.initial_disabled", "name": "Initially Disabled", "section": "server_datapack_options", "schema": { "type": "string_list" } }
	])).unwrap()
}

fn world_options() -> Vec<Control> {
	serde_json::from_value(json!([
		{ "id": "world.name", "name": "World Name", "section": "server_world_options", "schema": { "type": "string" } },
		{ "id": "world.seed", "name": "World Seed", "section": "server_world_options", "schema": { "type": "string" } },
		{ "id": "world.type", "name": "World Type", "section": "server_world_options", "schema": world_type() },
		{ "id": "world.structures", "name": "Generate Structures", "section": "server_world_options", "schema": { "type": "boolean" } },
		{ "id": "world.generator_settings", "name": "Generator Settings", "section": "server_world_options", "schema": { "type": "json" } },
		{ "id": "world.max_size", "name": "World Border Size", "section": "server_world_options", "schema": { "type": "number", "min": 1, "max": 60000000, "step": 1 } },
		{ "id": "world.max_build_height", "name": "Max Build Height", "section": "server_world_options", "schema": { "type": "number", "min": 64, "max": 2048, "step": 1 } },
		{ "id": "world.allow_nether", "name": "Allow Nether", "section": "server_world_options", "schema": { "type": "boolean" } }
	])).unwrap()
}

fn resource_pack_options() -> Vec<Control> {
	serde_json::from_value(json!([
		{ "id": "resource_pack.uri", "name": "URL", "section": "server_resource_pack_options", "schema": { "type": "string" } },
		{ "id": "resource_pack.prompt", "name": "Prompt", "section": "server_resource_pack_options", "schema": { "type": "string" } },
		{ "id": "resource_pack.sha1", "name": "SHA1 Hash", "section": "server_resource_pack_options", "schema": { "type": "string" } },
		{ "id": "resource_pack.required", "name": "Required", "section": "server_resource_pack_options", "schema": { "type": "boolean" } }
	])).unwrap()
}

fn management_options() -> Vec<Control> {
	serde_json::from_value(json!([
		{ "id": "management.enable", "name": "Enable Management", "section": "server_management_options", "schema": { "type": "boolean" } },
		{ "id": "management.host", "name": "Host", "section": "server_management_options", "schema": { "type": "string" } },
		{ "id": "management.port", "name": "Port", "section": "server_management_options", "schema": { "type": "number", "min": 1, "max": 65535, "step": 1 } },
		{ "id": "management.secret", "name": "Secret", "section": "server_management_options", "schema": { "type": "string" } },
		{ "id": "management.tls_enabled", "name": "TLS Enabled", "section": "server_management_options", "schema": { "type": "boolean" } },
		{ "id": "management.tls_keystore", "name": "TLS Keystore", "section": "server_management_options", "schema": { "type": "string" } },
		{ "id": "management.tls_keystore_password", "name": "TLS Keystore Password", "section": "server_management_options", "schema": { "type": "string" } }
	])).unwrap()
}

fn volume() -> ControlSchema {
	ControlSchema::Number {
		min: Some(0.0),
		max: Some(1.0),
		step: 0.01,
		slider: true,
	}
}

fn log_level() -> ControlSchema {
	serde_json::from_value(json!({
		"type": "choice",
		"variants": [
			{
				"id": null,
				"name": "Unset",
			},
			{
				"id": "none",
				"name": "None",
			},
			{
				"id": "high",
				"name": "High",
			},
			{
				"id": "medium",
				"name": "Medium",
			},
			{
				"id": "low",
				"name": "Low",
			},
			{
				"id": "notification",
				"name": "Notification",
			}
		],
	}))
	.unwrap()
}

fn tutorial_step() -> ControlSchema {
	serde_json::from_value(json!({
		"type": "choice",
		"dropdown": true,
		"variants": [
			{
				"id": null,
				"name": "Unset",
			},
			{
				"id": "none",
				"name": "None",
			},
			{
				"id": "movement",
				"name": "Movement",
			},
			{
				"id": "find_tree",
				"name": "Find Tree",
			},
			{
				"id": "punch_tree",
				"name": "Punch Tree",
			},
			{
				"id": "open_inventory",
				"name": "Open Inventory",
			},
			{
				"id": "craft_planks",
				"name": "Craft Planks",
			},
		],
	}))
	.unwrap()
}

fn narrator_mode() -> ControlSchema {
	serde_json::from_value(json!({
		"type": "choice",
		"dropdown": true,
		"variants": [
			{
				"id": null,
				"name": "Unset",
			},
			{
				"id": "off",
				"name": "Off",
			},
			{
				"id": "all",
				"name": "All",
			},
			{
				"id": "chat",
				"name": "Chat",
			},
			{
				"id": "system",
				"name": "System",
			},
		],
	}))
	.unwrap()
}

fn attack_indicator_mode() -> ControlSchema {
	serde_json::from_value(json!({
		"type": "choice",
		"variants": [
			{
				"id": null,
				"name": "Unset",
			},
			{
				"id": "off",
				"name": "Off",
			},
			{
				"id": "crosshair",
				"name": "Crosshair",
			},
			{
				"id": "hotbar",
				"name": "Hotbar",
			},
		],
	}))
	.unwrap()
}

fn main_hand() -> ControlSchema {
	serde_json::from_value(json!({
		"type": "choice",
		"variants": [
			{
				"id": null,
				"name": "Unset",
			},
			{
				"id": "left",
				"name": "Left",
			},
			{
				"id": "right",
				"name": "Right",
			},
		],
	}))
	.unwrap()
}

fn chat_visibility() -> ControlSchema {
	serde_json::from_value(json!({
		"type": "choice",
		"variants": [
			{
				"id": null,
				"name": "Unset",
			},
			{
				"id": "shown",
				"name": "Shown",
			},
			{
				"id": "commands_only",
				"name": "Commands Only",
			},
			{
				"id": "hidden",
				"name": "Hidden",
			},
		],
	}))
	.unwrap()
}

fn cloud_render_mode() -> ControlSchema {
	serde_json::from_value(json!({
		"type": "choice",
		"variants": [
			{
				"id": null,
				"name": "Unset",
			},
			{
				"id": "off",
				"name": "Off",
			},
			{
				"id": "fancy",
				"name": "Fancy",
			},
			{
				"id": "fast",
				"name": "Fast",
			},
		],
	}))
	.unwrap()
}

fn chunk_updates_mode() -> ControlSchema {
	serde_json::from_value(json!({
		"type": "choice",
		"variants": [
			{
				"id": null,
				"name": "Unset",
			},
			{
				"id": "threaded",
				"name": "Threaded",
			},
			{
				"id": "semi_blocking",
				"name": "Semi-Blocking",
			},
			{
				"id": "fully_blocking",
				"name": "Fully Blocking",
			},
		],
	}))
	.unwrap()
}

fn difficulty() -> ControlSchema {
	serde_json::from_value(json!({
		"type": "choice",
		"variants": [
			{
				"id": null,
				"name": "Unset",
			},
			{
				"id": "peaceful",
				"name": "Peaceful",
			},
			{
				"id": "easy",
				"name": "Easy",
			},
			{
				"id": "normal",
				"name": "Normal",
			},
			{
				"id": "hard",
				"name": "Hard",
			},
		],
	}))
	.unwrap()
}

fn particles_mode() -> ControlSchema {
	serde_json::from_value(json!({
		"type": "choice",
		"variants": [
			{
				"id": null,
				"name": "Unset",
			},
			{
				"id": "all",
				"name": "All",
			},
			{
				"id": "decreased",
				"name": "Decreased",
			},
			{
				"id": "minimal",
				"name": "Minimal",
			},
		],
	}))
	.unwrap()
}

fn graphics_mode() -> ControlSchema {
	serde_json::from_value(json!({
		"type": "choice",
		"variants": [
			{
				"id": null,
				"name": "Unset",
			},
			{
				"id": "fast",
				"name": "Fast",
			},
			{
				"id": "fancy",
				"name": "Fancy",
			},
			{
				"id": "fabulous",
				"name": "Fabulous",
			},
		],
	}))
	.unwrap()
}

fn network_compression() -> ControlSchema {
	serde_json::from_value(json!({
		"type": "choice",
		"variants": [
			{ "id": null, "name": "Unset" },
			{ "id": "disabled", "name": "Disabled" },
			{ "id": "all", "name": "All Packets" }
		]
	}))
	.unwrap()
}

fn world_type() -> ControlSchema {
	serde_json::from_value(json!({
		"type": "choice",
		"variants": [
			{ "id": null, "name": "Unset" },
			{ "id": "minecraft:normal", "name": "Normal" },
			{ "id": "minecraft:flat", "name": "Flat" },
			{ "id": "minecraft:large_biomes", "name": "Large Biomes" },
			{ "id": "minecraft:amplified", "name": "Amplified" },
			{ "id": "minecraft:single_biome_surface", "name": "Single Biome" },
			{ "id": "buffet", "name": "Buffet" },
		]
	}))
	.unwrap()
}

fn gamemode() -> ControlSchema {
	serde_json::from_value(json!({
		"type": "choice",
		"variants": [
			{ "id": null, "name": "Unset" },
			{ "id": "survival", "name": "Survival" },
			{ "id": "creative", "name": "Creative" },
			{ "id": "adventure", "name": "Adventure" },
			{ "id": "spectator", "name": "Spectator" }
		]
	}))
	.unwrap()
}
