# Format

## IDs

Your plugin needs a unique ID to distinguish it from others. Check the official plugin list or community lists to make sure the ID you want isn't already taken by another project. IDs must be lowercase with no special characters other than underscores (`_`). It is preferable to also keep them as short as possible.

## Files

The plugin file format is pretty simple. Inside the plugins directory (`NITRO_DATA/plugins`), all you need is a **manifest** file, located either at `plugins/plugin_id.json` or `plugins/plugin_id/plugin.json`. The nested location allows you to bundle other assets along with your plugin easily, but both locations work exactly the same.

If your plugin needs to run an executable, you can bundle your executable in your plugin directory, or install it on the system.

## Manifest

The manifest file describes information about your plugin in a JSON format. All fields are optional. It is structured like this:

```
{
	"name": string,
	"description": string,
	"nitro_version": string,
	"hooks": { ... },
	"subcommands": {
		"subcommand": "hook",
		...
	},
	"dependencies": [string],
	"install_message": string,
	"protocol_version": number,
	"raw_transfer": bool
}
```

- `name`: The display name of your plugin,
- `description`: A short description of your plugin
- `nitro_version`: The minimum version of Nitrolaunch that this plugin supports
- `hooks`: A map of hook IDs to hook handlers. Will be described more in the hooks section.
- `subcommands`: A map of custom subcommands to a short description of what they do
- `dependencies`: A list of plugin IDs that this plugin depends on to work
- `install_message`: A warning message to display when this plugin is installed. Often this is for mentioning dependencies like Python or NodeJS that the user needs to install.
- `protocol_version`: The version of the hook protocol that this plugin uses
- `raw_transfer`: Whether to call the hooks without any base64 encoding. This makes creating plugin programs easier, but can open up your plugin to vulnerabilities or bugs if unescaped data is sent to the hook.

## Hooks

Hooks are the meat and potatoes of plugins. They allow you to inject into specific points of Nitrolaunch's functionality, adding new features. They can act like event handlers, or like data-driven extensions to Nitrolaunch's data.

### Handling

You must define handlers for each hook you want to use in your plugin manifest. There are multiple types of handlers:

Constant handler that returns a fixed value every time it is called:

```
"hook_id": {
	"constant": any,
	"priority": "first" | "any" | "last"
}
```

- `constant`: The constant result of the handler. Will be a different type depending on the hook.

Handler that calls an executable using the hook protocol:

```
"hook_id": {
	"executable": string,
	"args": [string],
	"priority": "first" | "any" | "last"
}
```

- `executable`: The path to the executable to run. The token `"${PLUGIN_DIR}"` will be replaced with the path to the directory for the plugin if present, allowing you to package executables with your plugin easily.
- `args` (Optional): Additional command-line arguments to pass when running the hook. The `"${PLUGIN_DIR}"` token will be replaced for these as well.

Handler that gets the result from a file, containing the result in JSON format. If the result is a string, wrapping quotes are not needed.

```
"hook_id": {
	"file": string,
	"priority": "first" | "any" | "last"
}
```

- `file`: The path to the file to get the result from, relative to the plugin directory. Will fail if no plugin directory is present.

Handler that matches the hook argument to choose another hook handler to handle it with

```
"hook_id": {
	"property": string,
	"cases": {
		"case": HookHandler,
		"case2": HookHandler,
		...
	}
	"priority": "first" | "any" | "last"
}
```

- `property` (Optional): The property to match on the hook argument, if it is an object. If this is not present, the whole argument will be matched against.
- `cases`: The cases to match the hook argument or property against. If the hook argument or property equals the key of the case, then the hook handler inside will be run. Remember to use minified JSON for the case you are matching against.

- `priority` (Optional): The priority (order) for this hook to run relative to other plugins. The order for plugins with the same priority is not defined, but this priority allows you to somewhat control how your plugin interacts with others. Defaults to `"any"`, which runs in the middle of `"first"` and `"last"` hooks.

## State

Plugins can have state managed by Nitrolaunch for the duration of the Nitrolaunch program. This allows a plugin to communicate between hooks easily. Check documentation for how to use this state.

## Profiling and debugging

Plugins can be profiled by setting the environment variable `NITRO_PLUGIN_PROFILE=1` before running to see how long each plugin takes to run its hooks. The variable `NITRO_PLUGIN_DEBUG=1` can also be set to see which hooks are running.
