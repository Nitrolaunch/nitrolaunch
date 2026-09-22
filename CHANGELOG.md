# Nitrolaunch 0.32.1

## New Features
- CLI flag to automatically answer yes to all prompts
- CLI flag to skip the first-run onboarding
- The `plugin install` command will no longer re-install an existing plugin without a `--force` flag
- Minecraft theme!
- Information about packages and dependencies will now be saved to logs when updating packages on an instance

## Improvements
- Better styling for output indicator

## Bug Fixes
- Plugin versions were not updated in the verified list
- The most recent version of a plugin would not always be installed
- Modrinth slugs were not sanitized properly
- Addon downloads were prone to network errors

# Nitrolaunch 0.32.0

## Meta

- Totally revamped documentation, adding more guides for features and updating everything for both GUI and CLI

## New Features
- CurseForge support for browsing, packages, and modpacks!
- Purpur plugin, adding Purpur loader support and automatic installation
- NeoForge server launching!
- A new tab for the instance page which shows worlds, servers, and screenshots, letting you launch into them instantly
- A new manual files system, especially for CurseForge, which handles package files which must be downloaded manually
- GUI zoom configuration, letting you set the scale of the UI
- `template edit-base` command, making it easy to edit the base template

## Improvements
- Allow `neoforge` as an alias for `neoforged`
- Improved UI for sliders
- TUI package browse install locations are now sorted
- The Paper plugin now links the Mojang server JAR to prevent a redownload on first launch
- Minor UI improvements
- Sorted doc pages in the `docs` plugin

## Bug Fixes
- GUI plugins are not reloaded when they change, meaning a restart is needed more than it should be
- The wrong plugin would be installed if it was a substring of another plugin ID, such as `curseforge_api` installing when you requested `forge`
- Adoptium and GraalVM Java do not install properly on MacOS
- The final CLI error message would not print consistently
- The TUI package browser was broken in Kitty terminals
- The Paper plugin did not work due to a deprecated API
- Adding a package to an instance with browsing or the `package add` command would duplicate existing packages
- TUI package descriptions did not wrap
- TUI package filters would be empty if no repository was selected
- TUI package filters would have invalid values when switching repositories
- Modrinth packages could have invalid characters in slugs which would break parsing
- Mojang transfer did not migrate all instances when none were selected
- Mojang transfer had incorrect paths on all operating systems
- Packages with the same ID from different repositories would be combined into one when derived from a template
- Packages did not have versions if a specific version was installed
- NeoForge checked version compatability using a simple start match, meaning Minecraft 1.21.1 would install 1.21.11 incorrectly
- NeoForge used wrong server path
- NeoForge did not create a necessary leading directory leading to an error
- Force updates would not reinstall the NeoForge installer JAR
- The "Browse Packages" button did not work for templates
- `package add` command did not add the given repository or slug
- `latest` version pattern did not work correctly
- Sliders did not scale their values properly
- Installing a plugin in the GUI could be cancelled if you closed the settings popup
- Separators were missing between parent templates in the `instance info` command
- `plugin install` command would fail if only files were passed in
- Input actions for plugins did not work consistently
- New GUI did not set up executable registry so shortcuts plugin would not work

# Nitrolaunch 0.31.0

# Meta
- The GUI has been completely rewritten in the Freya framework, fixing lots of bugs on Wayland and Linux, reducing app sizes, and making the app run a lot faster

# New Plugins
- Guardian - A basic version of automatic malware scanning for your Minecraft mods. Detects common threat patterns automatically and reports them to you.
- Doctor - Can detect some common crashes and issues, giving solutions to help you fix them

# New Features
- Automatic partial package updates. Now, when you launch an instance, any simple changes such as a package being added or removed will be automatically resolved and applied, while leaving all your other packages untouched
- Multi-repository searching, letting you browse or query packages from multiple repositories at once.
- Output from instances in the CLI now has formatting for colors to help you read the logs better (can be disabled)
- `instance status` command to see what instances are running
- `instance kill` command
- Lots of new/different themes in the themes plugin. Try out sunset, catpuccin, and more!
- Icons are now added to installed modpacks
- "Extra" CLI commands, giving more shell integration for things like cd'ing into an instance
- Cleanup plugin now removes the Fabric cache as well

# Improvements
- Plugins now clean up their cache folders when uninstalled
- Forge versions are now cached
- Output a little message to let you know when an instance is done running
- Misode webtool
- Smithed plugin now adds the actual addon filenames

# Bug Fixes
- Old package addons would not be removed when the version changed
- CLI output had lots of synchronization issues leading to missing or overlapping messages
- Lots of issues with the Weld plugin
- The Mojang Transfer plugin was missing from the install list
- Version commands had wrong documentation, and --version did not work properly
- Quilt versions were not being parsed correctly
- Terrible performance over time as the Modrinth / Smithed search cache got bigger

# Hotfix as of 8/19/2026
- You can now easily go to the info page of packages in your config
- Added a drop shadow to dropdowns to make them easier to see
- Added skins cache cleanup to the cleanup plugfin
- Ability to bring a package in the browse page to fullscreen
- Added Flatpak package
- Repositories that fail to search are now skipped in multi-search rather than exiting the whole search
- Improved the UI for managing plugins
- Improved some UI icons
- Certain things in the GUI would not update after updating an instance
- Open directory buttons for data directories and instances didn't work properly
- An OS variant was not handled in client meta
- Fix instance import / export and modpack install being cancelled when the dialog is closed
- Fix addons using the wrong hash for storage
- Fix missing ID validation for new modpack instance
- Fix relative paths not working for custom commands
- Fix the wrong URL being used for verified plugins

# Nitrolaunch 0.30.0

## New Plugins
- Octane - Presets for the game to improve performance or memory usage
- Share - Combination of the addon and template share plugins
- Shortcut - Create desktop shortcuts to specific instances on Windows or Linux

## Features
- First-class modpack support - Configure and update a modpack on an instance and easily create a modpack instance from a file or package
- Skin and cape management, with integration with different account types and browsing platforms like Namemc
- Package browsing TUI for the CLI, letting you search for and install packages easily with image support
- See what packages changed when updating an instance before continuing
- Plugin controls API, letting you edit plugin configuration for instances and the app in the GUI
- Temporary instance creation, letting you download and launch a Minecraft version or modpack and choose whether to keep the instance afterward
- View application logs in the GUI
- Themes are now split into base and overlay, letting you combine full reskins and small changes
- Minecraft theme
- Pass quick play when launching an instance
- Utilities to manage your instances and templates, like extracting a template, duplicating an instance, or solidifying templates together
- `account switch` command
- `instance logs` command
- `plugin update` command
- `version list` command
- Unified instance launch API, letting external services launch Nitro instances whether the CLI or GUI is installed
- Plugin installation from a file
- `include` package relationships, letting a package provide other packages
- Restored periodic backups
- Button to view plugin documentation in GUI
- Ability to create an instance with a plugin in mind
- Vertical instance list theme

## Improvements
- Asynchronous internal package registry, improving GUI package config page load time
- Improved internal update handling, leading to less duplicated work and messages
- Better looking package resolution errors
- Improvements to WASM handling, letting hooks from different plugins run at the same time
- Track instance console input and output better, removing files when the instance stops
- Hashed addon storage improving disk usage
- Better CLI prompting for instance creation, making commands easier to use
- Renamed `user auth` to `user login`
- WASM plugins now compile when they are installed
- Internal package resolution improvements
- Internal instance handling improvements
- Better data typing for IO config
- Internal instance data is now stored in the instance folder if possible, improving portability
- Improved GUI styling
- Added some more confirmation messages
- Migrated the stats plugin to WASM, preventing some data corruption issues
- Many improvements to the Weld plugin, providing better stability and fixing some issues with addon integration

## Fixes
- Very slow Linux rendering issues with blurred shadows
- Instance config modal does not update properly
- Broken `instance info` formatting
- Throw an error if a version is not supported by Smithed
- MultiMC scan improvements
- Cannot migrate MultiMC instances on Windows
- Spurious "Main class overwritten twice" error
- Don't crash for some launch plugin errors
- Incorrect completions path
- Some broken styles
- Missing package script `slug` instruction
- Some plugin files are not included in release
- Search in package config is slow and case-sensitive
- Better theme loading error handling
- Package quick add creates invalid packages
- Offline authentication doesn't work
- Missing parity between `launch` and `instance launch` commands
- Default config is outdated
- Package override merging is broken
- Removed addon overwrite checking
- Servers won't launch sometimes
- Error with missing directory not letting you update an instance

# Nitrolaunch 0.29.0

## Meta
- Renamed users to accounts - There's only one real user, and that's you ❤
- Revamped how packages work, with the ID and display slug being separate parts of the request now. This means packages stay consistent, there are no duplicates, and fetching everything is way faster

## New Plugins
- Forge - Install Forge on your instances (currently only NeoForged and client instances are supported, but more is coming soon)
- Themes - A fresh set of extra themes for your launcher, currently with Sleek and Ghost themes
- Completions - Basic shell completions, currently with only simple ZSH completion

## Features
- You can now view the logs of an instance in the console tab
- Plugin API for custom logs
- Plugins can now override how their instances and templates are edited, letting you edit instances from the `config_split` plugin in the GUI, for example
- `instance edit` and `template edit` commands
- `log browse` command to view CLI launcher logs
- `plugin edit` command to edit config for a plugin
- Added a welcome message to the CLI similar to the GUI that prompts to install default plugins
- Launched instances now store the account that launched them, letting you select a specific one to kill
- Added a launcher version indicator to the GUI
- Plugins in the GUI now show indicators when an update is available
- WASM plugins can now use the NitroOutput API to send dynamic messages and prompts

## Improvements
- Modrinth package fetching is now much faster
- Both Modrinth and Smithed will now auto-update the cache every hour instead of only after a sync
- CLI output, specifically with long-running tasks and progress bars, is much better and animated
- Package update errors in the UI now show the packages much better
- The WASM plugin compile cache is now shared across the GUI, improving performance and preventing deadlocks
- You can now specify versions for each plugin in the `plugin install` command
- Improved the UI for the launcher settings, making it a modal window instead of a page
- Added the `temp` alias for template commands
- Internal improvements to package evaluation and resolution

## Fixes
- Template consolidation could still cause spurious errors
- Launcher asks you to log in every time
- When clicking to browse packages with a lastest or latest snapshot instance, things would break
- Windows launch crash due to named pipe issues
- MultiMC transfer does not follow link setting
- Modrinth and Smithed crash when searching because directory was not created
- External links on package pages open inside the same webview softlocking you
- Instance page does not react when the instance changes
- GUI skin head provider no longer works
- Multiple merge bugs with instance config
- Broken cached WASM is now recompiled automatically
- GUI config does not show custom Java types

# Version 0.28.1

## Features
- Instances and templates provided by plugins can now be edited and deleted
- `instance edit` and `template edit` commands to edit instances and templates easily, and allow editing plugin instances and templates
- `template delete` command
- `log browse` command to view application logs
- Update indicators on plugins
- Added a version indicator for the GUI

## Improvements
- Tons of visual improvements for the CLI, including loading spinners and better progress bars
- Improved settings page UI
- WASM loader cache is now shared between GUI commands, improving performance and fixing deadlocks
- Added temp alias for template commands
- Better log handling
- Java config in GUI now properly shows plugin Java types

## Fixes
- Cannot launch on Windows due to invalid named pipe creation
- Cannot change plugin version in GUI
- Replaced skin head preview provider
- Spurious cyclic template crashes
- Cannot switch instance page with running indicators
- Instance plugin config does not work
- Invalid cached WASM is now recompiled