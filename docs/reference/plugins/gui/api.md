# GUI JavaScript API

Whenever your plugin is running JavaScript code in the GUI (from hooks like `inject_page_script` and `add_dropdown_buttons`), there is an API of functions and objects available to you.

### `tauriInvoke(command: string, args: any)`

The Tauri `invoke` function

### `TauriWindow`

The Tauri `Window` class

### `copyToClipboard(text: string)`

Copies text to the user's clipboard

### `customAction(plugin: string, action: string, payload: any)`

Runs the `custom_action` hook with the given action and payload on the plugin you specify (usually your plugin)

### `setModal(contents: string | undefined)`

Opens a modal / dialog window with the given HTML contents. Can be set to undefined to close the modal.

### `globalInterval(id: string, function: () => void, interval: number)`

Like `setInterval()`, but will replace an existing interval with the same ID if it is called multiple times. Should be preferred to `setInterval()` in global code like `inject_page_script`. Useful for waiting for something to be loaded before attaching event listeners or new HTML elements.

### `showMessageToast(message: string | Element)`

Displays a message with a toast in the corner of the screen

### `showSuccessToast(message: string | Element)`

Displays a success message with a toast in the corner of the screen

### `showWarningToast(message: string | Element)`

Displays a warning message with a toast in the corner of the screen

### `showErrorToast(message: string | Element, isPersistent?: boolean)`

Displays an error message with a toast in the corner of the screen. `isPersistent` can be set to false to make the error not persist in the toast list after disappearing.

### `startTask(task: string)`

Displays to the user a long-running task in the footer. `task` can be whatever string you want. Used along with `endTask`.

### `endTask(task: string)`

Ends a long-running task. `task` should be the exact same string used with `startTask`.

### `inputError(id: string)`

Shows a red error shake animation on the element with the given ID.

### `clearInputError(id: string)`

Removes the red error shake animation on the element with the given ID. Should be done whenever there is not an error if you want the error animation to be removed.

### `sanitizeInstanceID(id: string)`

Sanitizes an input so that it is a valid instance / template / package ID.

### `updateInstanceList()`

Re-fetches the list of instances and templates on the homepage. Should be done whenever you add, remove, or change any instances with your plugin.
