# Sandboxing

Sandboxing an instance allows you to protect yourself against a large amount of malware by isolating the instance process and what files it can access. This prevents malicious mods and server plugins from stealing login credentials from other apps, reading personal documents, executing malicious code, or doing anything else on your computer that isn't necessary for the instance.

By sandboxing an instance, the game process will be prevented from accessing any file or port on your system unless that file, folder above it, or port is specifically allowed.

!!!warning Warning
By nature, sandboxing cannot protect against *every* possible attack. Combine sandboxing with the [Guardian](plugins/plugins/guardian.md) malware scanner and good internet practices to ensure you are safe.
!!!

!!!info
For right now, sandboxing is only available on Linux.
!!!

## Turning it on

Sandboxing will be disabled by default.

+++ App
Open the configuration for an instance and switch to the `Sandbox` tab. Switch `Enable Sandboxing` to true.
+++ CLI
Use `nitro instance edit <instance>` to open the configuration for the instance. Add a `sandbox` field and set `enable` to `true` like this:
```json
{
    "sandbox": {
        "enable": true
    }
}
```
+++

You can also enable it on your base template to apply sandboxing globally to all instances.

After being enabled, sandboxing should work pretty much out of the box. You will probably see lots of errors in your console, but these are mostly false flags that don't affect the instance.

## Groups

Groups are collections of files that make it easy to allow or disallow system features in the sandboxed instance.

By default, the `Base`, `Instance`, `Game Files`, `Devices`, and `Network` groups will be enabled for an instance. These are essential for an instance to run. Client instances will also enable the `Graphics`, `Input`, and `Audio` groups.

- Base (`base`) - Allows executing Java, reading from stdin and stdout, using system libraries, and some other essential things
- Instance (`instance`) - Allows usage of the instance's folder
- Game Files (`game_files`) - Allows reading shared Minecraft assets and libraries
- Devices (`devices`) - Allows reading and writing to any device on the computer. Can be disabled and narrowed down to the exact devices you need, but this can take a lot of work.
- Network (`network`) - Allows ports for Minecraft servers, HTTP, and HTTPS. Can be disabled if you are only running an instance locally.
- Graphics (`graphics`) - Allows reading and writing graphics-essential files
- Input (`input`) - Does nothing for now, use `devices`
- Audio (`audio`) - Allows reading audio configuration and other things essential for audio

### Disabling default groups

+++ App
In the `Sandbox` tab, add groups to the disabled list.
+++ CLI
```json
{
	"sandbox": {
		"policy": {
			"disallowed": ["network"]
		}
	}
}
```
+++

### Allowing extra groups

+++ App
In the `Sandbox` tab, add groups to the allowed list.
+++ CLI
```json
{
	"sandbox": {
		"policy": {
			"allowed": ["graphics"]
		}
	}
}
```
+++

## Allowing extra files

You can also specifically allow extra files and folders to be accessed, with specific permissions. Each path can be set as either read-only, read-write, or executable.

+++ App
In the `Sandbox` tab, add paths with permissions to the allowed list.
+++ CLI
```json
{
	"sandbox": {
		"policy": {
			"allowed_paths": {
				"/etc/mypath": "read",
				"/home/my_executable": "execute",
				"/foo/bar": "read_write"
			}
		}
	}
}
```
+++