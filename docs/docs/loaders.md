# Loaders

MCVM and its packages support multiple different loaders for the game. The package format attempts to support as many modifications as possible, but that does not mean that every launcher is able to install all of them automatically.

The different types of fields are listed here. Variants may be listed with `supported` or `unsupported` depending on whether MCVM supports installing them.

- `vanilla`: The standard game. (supported)
- `fabric`: The Fabric modloader. (supported)
- `quilt`: The Quilt modloader. (supported)
- `forge`: The MinecraftForge modloader. (unsupported)
- `neoforged`: The NeoForged modloader. (unsupported)
- `liteloader`: The LiteLoader modloader. (unsupported)
- `risugamis`: Risugami's modloader. (unsupported)
- `rift`: The Rift modloader. (unsupported)
- `paper` PaperMC server (supported)
- `sponge` SpongeVanilla server (supported)
- `spongeforge` SpongeForge server (unsupported)
- `craftbukkit` CraftBukkit server (unsupported)
- `spigot` Spigot server (unsupported)
- `glowstone` Glowstone server (unsupported)
- `pufferfish` Pufferfish server (unsupported)
- `purpur` Purpur server (unsupported)
- `folia` Folia server (supported)
- `fabric` The Fabric modloader. (supported)
- `quilt` The Quilt modloader. (supported)
- `forge` The Forge modloader. (unsupported)
- `neoforged` The NeoForged modloader. (unsupported)
- `risugamis` Risugami's modloader. (unsupported)
- `rift` The Rift modloader. (unsupported)

## Loader matches (`loader_match`)

Loader matches are used in packages to match different loaders that support the same format

- `vanilla`
- `fabric`
- `quilt`
- `forge`
- `neoforged`
- `liteloader`
- `risugamis`
- `rift`
- `fabriclike`: Matches any loader that supports loading Fabric mods (Fabric and Quilt).
- `forgelike`: Matches any loader that supports loading Forge mods (MinecraftForge and SpongeForge).
- `bukkit`: Matches any server that can load Bukkit plugins (CraftBukkit, Paper, Spigot, Glowstone, Pufferfish, and Purpur).
