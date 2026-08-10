# Guardian
ID: `guardian`

The Guardian plugin is an automatic malware scanner for mods and server plugins. It can detect many common patterns of token stealers, IP grabbers, and viruses.

## Usage
By default, Guardian will automatically scan whenever you update an instance, reporting an error if any files do not pass the scan.

To do a manual scan, you can run `nitro guardian scan <file>` to get a full report on the given file.

## Configuring
In the GUI, this can be configured by going to an instance's configuration under the `Guardian` section.

In your instance / template config:

```
{
    "guardian": {
        "scan": bool
    }
}
```

- `scan`: Whether to enable scanning on this instance. Defaults to `true`.
