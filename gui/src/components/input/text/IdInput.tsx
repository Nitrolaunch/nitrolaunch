import { sanitizeInstanceId } from "../../../pages/instance/InstanceConfig";

export default function IdInput(props: IdInputProps) {
	return <input
		type="text"
		id="id"
		name="id"
		onChange={(e) => {
			e.target.value = sanitizeInstanceId(e.target.value);
			props.onChange(e.target.value);
		}}
		onKeyUp={(e: any) => {
			if (
				e.target.value == undefined ||
				e.target.value.length == 0
			) {
				props.onChange("");
			} else {
				props.onChange(e.target.value);
			}
			e.target.value = sanitizeInstanceId(e.target.value);
		}}
	></input>;
}

export interface IdInputProps {
	value: string;
	onChange: (value: string) => void;
}