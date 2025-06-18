// General utilities for data handling

// Canonicalizes a common deserializion format (DeserListOrSingle), which can be a single value, a list of values, or undefined, to just an array
export function canonicalizeListOrSingle<T>(value: T | T[] | undefined) {
	if (value == undefined) {
		return [];
	}
	if (Array.isArray(value)) {
		return value;
	}

	return [value];
}

// Returns an empty string when a value is undefined
export function emptyUndefined(value: string | undefined) {
	if (value == undefined) {
		return "";
	} else {
		return value;
	}
}

// Returns undefined when a value is an empty string
export function undefinedEmpty(value: string | undefined) {
	if (value == "") {
		return undefined;
	} else {
		return value;
	}
}
