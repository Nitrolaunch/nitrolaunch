# Packages

An Nitrolaunch package is simply a file that is evaluated to install files and dependencies. They can be either declarative JSON files or custom scripts. Scripts usually follow the format of `package-id.pkg.txt`. Declarative packages should be named `package-id.json`. Package IDs may contain only letters, numbers, and hyphens (`-`). They cannot be longer than 32 characters.

# Repository

A package repository is any server that provides an `index.json` of packages for the user to source. All that is required to run a repository yourself is to make this `index.json` under `https://example.com/api/nitrolaunch/index.json`. An index follows this format:

```
{
	"metadata": {
		"name": string,
		"description": string,
		"nitro_version": string,
		"color": string
	}
	"packages": {
		"package-id": {
			"url": string,
			"path": string,
			"content_type": "script" | "declarative"
		}
	}
}
```

- `metadata.name` (Optional): The display name of the repository.
- `metadata.description` (Optional): A short description of the repository.
- `metadata.nitro_version` (Optional): The oldest Nitrolaunch version that packages included in the repository are compatible with. Used to give warnings to the user.
- `metadata.color` (Optional): A CSS color that represents the repository.
- `package-id`: The ID of the package.
- `url`: The URL to the package file. Unnecessary if `path` is specified.
- `path`: The path to the package file. Unnecessary if `url` is specified. On local repositories, can be either an absolute filesystem path or a path relative to where the index is. On remote repositories, can only be a relative url from where the index is.
- `content_type`: What type of package this is. Defaults to `"script"`.

## Version Patterns

Version patterns are strings that can be used to match against one or more version of something, often Minecraft or packages. There are a couple variants:

- `single` (Example "1.19.2"): Match a single version.
- `before` (Example "1.19.2-"): Matches a version and all versions before it (inclusive).
- `after` (Example "1.19.2+"): Matches a version and all versions after it (inclusive).
- `range` (Example "1.19.1..1.20.1"): Matches versions in a range (inclusive).
- `prefer` (Example "~1.19.1"): Specifically for package versions, will prefer to install this version, but won't error if something else is matched instead.
- `latest` ("latest"): Matches only the latest version.
- `any` ("\*"): Matches any version.

Each variant can be escaped using backslashes, but keep in mind that all backslashes will be stripped from the final output

# The purpose of host ID instructions

These should be set even if the addons for the package are not downloaded from that website. These will allow Nitrolaunch to make smart decisions in the future and automatically replace files downloaded from these sites with the correct packages and prevent file duplication.
