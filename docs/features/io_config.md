# IO Config

Some of the lower-level features of Nitrolaunch can be configured through environment variables or a specific file

## Usage

Each property can be configured either through the environment variable `NITRO_{PROPERTY_NAME}` or in a lowercase property in the file `{HOME_DIR}/.nitro.json`.
For example:

`NITRO_TRANSFER_LIMIT=1`
or

```json
{
	"transfer_limit": 1
}
```

When both are set, the environment variable takes precedence.

### Types
- `boolean`: `true / false` for JSON or `1 / 0` for environment variables
- `number`: number
- `string`: string

## Properties

### `transfer_limit` - `number`
The number of concurrent tasks that should be used to download lots of files, for example when downloading addons, game assets, or libraries. Can fix some issues or improve your download speeds on certain systems and connections. Defaults to a good value for your system.

### `link_method` - `string`, `"hard" | "soft" | "copy"`
The IO method used to link shared files on the filesystem. `hard` is used by default as it is the most performant and compatible, but `soft` can be used if you are linking between different filesystems. `copy` is almost never the answer, and can also cause bugs if the files are meant to be modified.

### `data_path` - `string`
Path to the data folder, containing instances, plugins, and internal nitro data. Can be used to save filesystem space by changing where nitro stores files.

### `config_path` - `string`
Path to the config folder, containing nitro configuration.

### `cli_icons` - `boolean`
Enables or disables icons for the CLI. Defaults to false.

### `cli_wrap` - `boolean`
Enables or disables text wrapping for the CLI. Defaults to true.
