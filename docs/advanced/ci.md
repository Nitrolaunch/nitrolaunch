# CI and Automation

Here are some tips and information for running Nitrolaunch in CI pipelines, Docker containers, or any other automated system.

## Installing

To install the CLI, which is the most useful for automation, the fastest way is to download it from the GitHub release.

For example:
```
curl -o nitro https://github.com/Nitrolaunch/nitrolaunch/releases/download/0.32.0/Nitrolaunch_cli_0.32.0_linux
```

Then the CLI can be run with `./nitro`.
## Skipping Onboarding

The first time the CLI is run after being installed, it will prompt you to install default plugins. This is probably not want you want in an automated pipeline.

To disable, simply add `--skip-onboarding` before your first nitro command, for example `nitro --skip-onboarding instance list`. You will not need to add the flag anymore after the first command.

## Running as Root

When running Nitrolaunch as root on Linux, the data directory will be `/root/.local/share/nitro`, **not** `/usr/share/nitro`.