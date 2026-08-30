# Version Patterns

Version patterns are strings that can be used to match against one or more version of something, often Minecraft or packages. There are a couple variants:

- `single` (Example "1.19.2"): Match a single version.
- `before` (Example "1.19.2-"): Matches a version and all versions before it (inclusive).
- `after` (Example "1.19.2+"): Matches a version and all versions after it (inclusive).
- `range` (Example "1.19.1..1.20.1"): Matches versions in a range (inclusive).
- `prefer` (Example "~1.19.1"): Specifically for package versions. Lets multiple packages depend on different explicit versions, picking the newest one out of all of them.
- `latest` ("latest"): Matches only the latest version.
- `any` ("\*"): Matches any version.

Each variant can be escaped using backslashes, but keep in mind that all backslashes will be stripped from the final output