# Share
ID: `share`

The Share plugin lets you easily share your configuration or addons with friends to synchronize your experience

## Usage

### Sharing Templates
- `nitro template share <template>`: Exports a template online and gives you a code to copy to share it
- `nitro template use <code> <id>`: Imports a template from the given code, giving it <id> as it's new ID in your config

### Sharing Addon Zips
`nitro instance share-addons <instance> <addon1> <addon2> ...`

- `<instance>` The instance to zip the addons of
- `<addon>` Addon types to include in the zip. Can be one of `mods`, `resource_packs`, `plugins`, or `shaders`.
- By default the output is saved to `./addons.zip`. You can use the `--output` flag to specify another filename if you want.
