# Packages Guide
Packages are a new concept introduced by Nitrolaunch to allow easy installation of mods, resource packs, and more. By using them, you don't have to worry about dependencies, downloading, what folders to use, or mod conflicts. Everything mostly just works.

## 1. Finding the packages you want
Packages are referred to using their ID, which is always lowercase. To find the packages you want, use the `nitro package browse` command to search through and get information about the packages you want to install.

## 2. Adding packages to an instance
From the package browser, installing a package is very easy. Simply press `[i]` on the package and pick where to install. You can also edit your configuration and add the package want to the `packages` field of that instance or template.

Example:
```
{
	"instances": {
		"example": {
			"version": "1.20.1",
			"side": "client",
			"loader": "fabric",
			"packages": [
				"modrinth:sodium",
				"modrinth:create"
			]
		}
	}
}
```

Note that you must include a `repository:` tag in front of each package to specify what repository the package is from.

## 3. Updating packages
Now that you have added a package to an instance, make sure to run `nitro instance update <instance>` in order to actually install the package. You should also do this whenever you remove packages, or want to update them to new versions.
