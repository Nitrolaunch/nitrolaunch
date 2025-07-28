# Hooks

Hooks are what give your plugin functionality. They are essentially custom event handlers that can be written in any language. They can run code whenever something happens, or inject new items into some of the data-driven parts of Nitrolaunch. Handlers for hooks are defined in the plugin manifest.

## Parts of a Hook

- ID: Every hook has a unique ID used to identify it
- Argument and Result: These are the inputs and outputs of the hook. They can be any JSON type, such as a string or object, and depend on which hook you are handling.

## How Hooks are Run

Most of the time when Nitrolaunch calls a hook, it will check every plugin that supports that hook, and call the hook on each one to create a final list of results. Handlers are not exclusive; multiple plugins can subscribe to the same hook. However, some hooks are only called on specific plugins. For example, the `on_load` hook is only called on a specific plugin once it is loaded.

## List of Hooks

### `on_load`

Called when this plugin is loaded. Can be used to set up state and such.

- Argument: None
- Result: None

### `subcommand`

Called whenever one of the subcommands that this hook registers are run. The argument is the list of arguments that were provided to the subcommand, _including_ the subcommand itself. Note that this hook also takes over output, meaning anything coming from stdout will be output to the console instead.

- Argument: `[string]`
- Result: None

### `modify_instance_config`

Called on every instance to possibly modify its config. The output config will be merged with the instance's current config in the same way as profiles are. Note that the input is not sequential: All plugins will be given the same config before modification, instead of applying one after the other, and the results will all be merged together.

- Argument:

```
{
	"config": InstanceConfig
}
```

- Result:

```
{
	"config": InstanceConfig
}
```

### `add_versions`

This hook allows you to add extra Minecraft versions to the version manifest, allowing them to be specified in instance configuration and automatically downloaded.

- Argument: None
- Result:

```
[
	{
		"id": string,
		"type": "release" | "snapshot" | "old_alpha" | "old_beta",
		"url": string,
		"is_zipped": bool
	},
	...
]
```

### `on_instance_setup`

Called when an instance is being set up, for update or launch. Can return modifications to make to the launch parameters
resulting from installing a certain modification

- Argument:

```
{
	"id": string,
	"side": "client" | "server",
	"game_dir": string,
	"version_info": {
		"version": string,
		"versions": [string]
	},
	"loader": Loader,
	"current_loader_version": string | null,
	"desired_loader_version": string | null,
	"config": InstanceConfig,
	"internal_dir": string,
	"update_depth": "shallow" | "full" | "force"
}
```

- Result:

```
{
	"main_class_override": string | null,
	"jar_path_override": string | null,
	"classpath_extension": [string]
}
```

### `remove_loader`

Called when the loader of an instance changes, to allow cleaning up old or invalid files. Will be given the loader that needs to be removed.

- Argument:

```
{
	"id": string,
	"side": "client" | "server",
	"game_dir": string,
	"version_info": {
		"version": string,
		"versions": [string]
	},
	"loader": Loader,
	"custom_config": {...},
	"internal_dir": string,
	"update_depth": "shallow" | "full" | "force"
}
```

- Result: None

### `on_instance_launch`

Called whenever an instance is launched

- Argument: InstanceLaunchArg
- Result: None

### `while_instance_launch`

Also called when an instance is launched, but is non-blocking, and runs alongside the instance. Can be used for periodic tasks and such.

- Argument: InstanceLaunchArg
- Result: None

### `on_instance_stop`

Called when an instance is stopped. This happens when Minecraft is closed or crashes. This hook will _not_ be called if Nitrolaunch crashes while the instance is running.

- Argument: InstanceLaunchArg
- Result: None

### `custom_package_instruction`

Handles custom instructions in script packages.

- Argument:

```
{
	"pkg_id": string,
	"command": string,
	"args": [string]
}
```

- Result:

```
{
	"handled": bool,
	"addon_reqs": [
		{
			"id": string,
			"file_name": string | null,
			"kind": "resource_pack" | "mod" | "plugin" | "shader" | "datapack",
			"url": string | null,
			"path": string | null,
			"version": string | null,
			"hashes": {
				"sha256": string | null,
				"sha512": string | null
			}
		}
	],
	"deps": [
		{
			"value": string,
			"explicit": bool
		}
	],
	"conflicts": [string],
	"recommendations": [
		{
			"value": string,
			"invert": bool
		}
	],
	"bundled": [string],
	"compats": [[string, string]],
	"extensions": [string],
	"notices": [string]
}
```

- `handled`: Whether this instruction was handled or not. Should be false if this instruction is not for your plugin.

## `handle_auth`

Handles authentication with custom user types

- Argument:

```
{
	"user_id": string,
	"user_type": string
}
```

- Result:

```
{
	"handled": bool,
	"profile": {
		"name": string,
		"id": string,
		"skins": [
			{
				"id": string,
				"url" string,
				"state": "active" | "inactive",
				"variant": "classic" | "slim"
			}
		],
		"capes": [
			{
				"id": string,
				"url" string,
				"state": "active" | "inactive",
				"alias": string
			}
		]
	} | null
}
```

- `profile.id`: The UUID of the user

### `add_translations`

Adds extra translations to Nitrolaunch

- Argument: None
- Result:

```
{
	"language": {
		"key": "translation",
		...
	},
	...
}
```

### `add_instance_transfer_formats`

Adds information about new transfer formats that this plugin adds support for. Returns a list of formats, including information about features that they support and don't support.

- Argument: None
- Result:

```
[
	{
		"id": string,
		"import": {
			"modloader": "supported" | "format_unsupported" | "plugin_unsupported",
			"mods": "supported" | "format_unsupported" | "plugin_unsupported",
			"launch_settings": "supported" | "format_unsupported" | "plugin_unsupported"
		} | null,
		"export": {
			"modloader": "supported" | "format_unsupported" | "plugin_unsupported",
			"mods": "supported" | "format_unsupported" | "plugin_unsupported",
			"launch_settings": "supported" | "format_unsupported" | "plugin_unsupported"
		} | null
	},
	...
]
```

### `export_instance`

Hook called on a specific plugin to export an instance using one of the formats it supports

- Argument:

```
{
	"format": string,
	"id": string,
	"config": InstanceConfig,
	"minecraft_version": string,
	"loader_version": string,
	"game_dir": string,
	"result_path": string
}
```

- `id`: The instance ID
- `result_path`: The desired path to the output file
- Result: None

### `import_instance`

Hook called on a specific plugin to import an instance using one of the formats it supports

- Argument:

```
{
	"format": string,
	"id": string,
	"source_path": string,
	"result_path": string
}
```

- `id`: The desired ID of the resulting instance
- `source_path`: The path to the instance to import
- `result_path`: Where to place the files for the imported instance
- Result:

```
{
	"format": string,
	"config": InstanceConfig
}
```

### `add_supported_loaders`

Adds extra loaders to the list of supported ones for installation. This should be done
if you plan to install these loaders with your plugin.

- Argument: None
- Result: Loader[]

### `add_instances`

Adds new instances to the config

- Argument: None
- Result:

```
{
	"inst1": InstanceConfig,
	"inst2": InstanceConfig,
	...
}
```

### `add_profiles`

Adds new profiles to the config

- Argument: None
- Result:

```
{
	"prof1": ProfileConfig,
	"prof2": ProfileConfig,
	...
}
```

### `inject_page_script`

Called whenever certain pages in the GUI are opened. Runs whatever the result of the hook is as Javascript on the page.

- Argument:

```
{
	"page": "instances" | "instance" | "instance_config" | "profile_config" | "global_profile_config",
	"object": string | null
}
```

- Result: string

- `object`: The identifier for whatever 'thing' this page is representing. Could be an instance, profile, anything else, or nothing.

### `add_sidebar_buttons`

Adds custom buttons to the sidebar

- Argument: None

- Result:

```
{
	"html": string,
	"href": string,
	"selected_url": string | null,
	"selected_url_start": string | null,
	"color": string
}
```

- `html`: The inner HTML of the button
- `href`: Where the button leads to, likely a custom page
- `selected_url`: What the current URL should equal to select this item
- `selected_url_start`: What the current URL should start with to select this item

### `get_page`

Lets you add custom pages. The page will be available at `/custom/yourcustompagedata`. You can include custom data like a specific ID in the data section of the URL as well.

- Argument: string

This is the custom page data in the URL

- Result: string | null

This is the resulting page as HTML. Only include things that would be in a `<body>` tag.

### `add_custom_package_repositories`

Adds new package repositories that can be queried and searched

- Argument: None

- Result:

```
[
	{
		"id": string,
		"is_preferred": bool,
		"metadata": RepoMetadata
	},
	...
]
```

- `is_preferred`: Whether this repository should be loaded before or after repositories like `std` and `core`
- `metadata`: [RepoMetadata](../../packages/index.md)

### `query_custom_package_repository`

Asks for a package from a custom repository that this plugin registered with `add_custom_package_repositories`.

- Argument:

```
{
	"repository": string,
	"package": string
}
```

- Result:

```
{
	"contents": string,
	"content_type": "script" | "declarative",
	"flags": [
		"out_of_date" | "deprecated" | "insecure" | "malicious", ...
	]
} | null
```

### `search_custom_package_repository`

Searches / browses for packages from a custom repository that this plugin registered with `add_custom_package_repositories`.

- Argument:

```
{
	"repository": string,
	"parameters": {
		"count": integer,
		"skip": integer,
		"search": string | null,
		"types": PackageType[],
		"minecraft_versions": string[],
		"loaders": Loader[],
		"categories": PackageCategory[]
	}
}
```

- Result:

```
{
	"results": string[],
	"total_results": integer,
	"previews": {
		"package": [PackageMetadata, PackageProperties],
		...
	}
}
```

- `previews`: Limited data about packages used to make quick previews. Useful if your API returns them, as it makes browsing much faster.

### `search_custom_package_repository`

Synchronizes the cache for a custom repository that this plugin registered with `add_custom_package_repositories`. Should remove all cached packages associated with the repository so that new versions of packages can be used.

- Argument:

```
{
	"repository": string
}
```

- Result: None

## Common Types

### InstanceLaunchArg

```
{
	"id": string,
	"side": "client" | "server",
	"dir": string,
	"game_dir": string,
	"version_info": {
		"version": string,
		"versions": [string]
	},
	"config": InstanceConfig,
	"pid": integer | null,
	"stdout_path": string | null,
	"stdin_path": string | null
}
```

Note: The `pid`, `stdout_path`, and `stdin_path` fields will all be `null` for the `on_instance_launch` hook, and are only available in the other hooks.
