# Instance Templates

Instance templates are a powerful system that save you time by sharing common configuration between instances.

## Creating and Managing Templates

Template configuration is almost identical to instances. They have all of the exact same fields as instances, except all of them are optional.

+++ App
Navigate to the `Templates` tab of the home page. Here, you can create, edit, and remove templates just like you would instances. Click `Edit` at the bottom to edit one.
+++ CLI
Use the `nitro template add|edit|delete|list|info` commands to manage your templates.
+++

## How Inheritance Works

The way configuration is derived is pretty much intuitive. Any **unset** value on an instance will be set to the template's value, if there is one. If the instance has a value, it will override the template.

For example, my instance derives from a template which specifies that the Minecraft version is 1.19.2. If the instance does not set a Minecraft version, it will inherit the `1.19.2`.

## Using a Template

+++ App
In an instance's configuration page, select templates you want to inherit from in the `Parent Templates` field.
+++ CLI
Set the `from` field to the ID of the template you want to use. For example, `"from": "my-template"`.
+++

## The Base Template

All instances and templates inherit from a special template, the base template, which can be used for global settings you want to apply everywhere.

To edit the base template:

+++ App
In the `Templates` tab of the home page, the base template will be the first template in the list. Double click or click `Edit` at the bottom to edit it.
+++ CLI
Use the `nitro template edit-base` command.
+++

## Chaining Templates

Templates don't just have to be used by instances, they can also be inherited by other templates. This can be useful for when you have some broad configuration that you use, and a more specified configuration that derives from it.

## Multiple Templates

An instance or template can also inherit from multiple other templates. They will be applied one after the other, with the templates closest to the end having priority.

+++ App
Simply select multiple templates in the `Parent Templates` dropdown.
+++ CLI
The `from` field also accepts a list as well. For example: `"from": ["template1", "template2"`.
+++