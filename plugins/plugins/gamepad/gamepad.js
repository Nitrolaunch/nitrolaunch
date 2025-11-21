let GP_DPAD_UP = 12;
let GP_DPAD_DOWN = 13;
let GP_DPAD_LEFT = 14;
let GP_DPAD_RIGHT = 15;

// Check connects and disconnects

function gamepadConnected(e) {
	showMessageToast("Gamepad connected: " + e.gamepad.id);
	if (e.gamepad.mapping != "standard") {
		showWarningToast("Gamepad is not using standard mapping and will not work");
	}
}

window.removeEventListener("gamepadconnected", gamepadConnected);
window.addEventListener("gamepadconnected", gamepadConnected);

function gamepadDisconnected(e) {
	showMessageToast("Gamepad disconnected: " + e.gamepad.id);
}

window.removeEventListener("gamepaddisconnected", gamepadDisconnected);
window.addEventListener("gamepaddisconnected", gamepadDisconnected);

// Gets the element focused by the gamepad
function getGamepadFocus() {
	let elems = document.getElementsByClassName("gamepad-focus");
	if (elems.length == 0) {
		return null;
	} else {
		return elems[0];
	}
}

function addFocus(elem) {
	elem.classList.add("gamepad-focus");
}

function removeFocus(elem) {
	elem.classList.remove("gamepad-focus");
}

// Key navigation

function gamepadKeyEvent(e) {
	if (e.key.includes("Arrow") || e.key == "Enter") {
		let focus = getGamepadFocus();
		if (focus == null) {
			return;
		}

		if (e.key == "ArrowUp") {
			updateFocus(focus, "up");
		} else if (e.key == "ArrowRight") {
			updateFocus(focus, "right");
		} else if (e.key == "ArrowDown") {
			updateFocus(focus, "down");
		} else if (e.key == "ArrowLeft") {
			updateFocus(focus, "left");
		} else if (e.key == "Enter") {
			gamepadAction(focus);
		}

		e.preventDefault();
	}
}

document.removeEventListener("keydown", gamepadKeyEvent);
document.addEventListener("keydown", gamepadKeyEvent);

// Used for rising-edge button checks to make sure we only act on the first frame of press
let upPressedLast = false;
let downPressedLast = false;
let leftPressedLast = false;
let rightPressedLast = false;

// Gamepad checks
globalInterval("gamepad_update", () => {
	let gamepads = navigator.getGamepads();
	for (let gamepad of gamepads) {
		if (gamepad.mapping == "standard") {
			let up = gamepad.buttons[GP_DPAD_UP];
			let down = gamepad.buttons[GP_DPAD_DOWN];
			let left = gamepad.buttons[GP_DPAD_LEFT];
			let right = gamepad.buttons[GP_DPAD_RIGHT];

			let focus = getGamepadFocus();
			if (focus != null) {
				if (!upPressedLast && up.pressed) {
					updateFocus(focus, "up");
				} else if (!downPressedLast && down.pressed) {
					updateFocus(focus, "down");
				} else if (!leftPressedLast && left.pressed) {
					updateFocus(focus, "left");
				} else if (!rightPressedLast && right.pressed) {
					updateFocus(focus, "right");
				}
			}

			upPressedLast = up.pressed;
			downPressedLast = down.pressed;
			leftPressedLast = left.pressed;
			rightPressedLast = right.pressed;
		}
	}
}, 50);

// Sets the default focus on a page
globalInterval("gamepad_set_empty_focus", () => {
	let focus = getGamepadFocus();
	if (focus == null) {
		let instance = document.querySelector(".instance-list-item[data-index][data-section=\"all\"]");
		if (instance != null) {
			addFocus(instance);
			return;
		}
	}
}, 150);

// Updates focus based on a direction (left, right, up, down)
function updateFocus(focusElem, dir) {
	if (focusElem.classList.contains("instance-list-item")) {
		let index = focusElem.dataset.index * 1;
		let section = focusElem.dataset.section;

		let newIndex = index;
		if (dir == "left") {
			newIndex = index - 1;
		} else if (dir == "right") {
			newIndex = index + 1;
		} else if (dir == "up") {
			newIndex = index - 3;
		} else if (dir == "down") {
			newIndex = index + 3;
		}

		let newFocus = document.querySelector(`.instance-list-item[data-index="${newIndex}"][data-section="${section}"]`);
		if (newFocus != null) {
			addFocus(newFocus);
			removeFocus(focusElem);
		}
	}
}

// Runs an action on the currently selected item
function gamepadAction(focusElem) {
	if (focusElem.classList.contains("instance-list-item")) {
		focusElem.click();
		addFocus(focusElem);
	}
}

addStyle(`
	.gamepad-focus {
		outline: 0.2rem solid var(--instance);
	}	
`);
