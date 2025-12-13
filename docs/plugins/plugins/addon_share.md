# Addon Share
ID: `addon_share`

The Addon Share plugin lets you easily zip the addons in an instance into a .zip file. 

## Usage
`nitro instance share-addons <instance> <addon1> <addon2> ...`

- `<instance>` The instance to zip the addons of
- `<addon>` Addon types to include in the zip. Can be one of `mods`, `resource_packs`, `plugins`, or `shaders`.
- By default the output is saved to `./addons.zip`. You can use the `--output` flag to specify another filename if you want.
