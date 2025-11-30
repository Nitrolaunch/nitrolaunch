import sys
from smithed.weld import run_weld
import json
from pathlib import Path
import os

def output(method: str, data: object | None | str = None):
	if data is None:
		print(f"%_\"{method}\"")
	else:
		out = {}
		if data == "__null__":
			out[method] = None
		else:
			out[method] = data
		out2 = json.dumps(out)
		print(f"%_{out2}")

# Welds a single datapack / resourcepack directory. Mode should be either "data" or "resource".
def weld_dir(dir: Path, ignore: list, mode: str):
	# Dir to store unwelded files so that they persist across updates
	unwelded_path = dir.joinpath("unwelded")
	if not unwelded_path.exists():
		os.mkdir(unwelded_path)

	# Move all non-ignored packs to the unwelded dir
	for entry in os.listdir(dir):
		path = dir.joinpath(entry)

		if "Welded Packs" in entry:
			continue
		for ignored in ignore:
			if ignored in entry:
				continue

		if path.is_file():
			target = unwelded_path.joinpath(entry)
			# It won't move if the target already exists
			if target.exists():
				os.remove(target)
			os.rename(path, target)

	# Move any ignored packs out of the unwelded dir
	for entry in os.listdir(unwelded_path):
		for ignored in ignore:
			if ignored in entry:
				source = unwelded_path.joinpath(entry)
				target = dir.joinpath(entry)
				if target.exists():
					os.remove(target)
				os.rename(source, target)

	# Now Weld
	beet_config = {
		"output": str(dir)
	}

	packs = [str(unwelded_path.joinpath(x)) for x in os.listdir(unwelded_path)]
	
	target_pack_path = dir.joinpath("Welded Packs.zip")
	with run_weld(packs=packs,config=beet_config,directory=dir) as ctx:
		if mode == "data":
			ctx.data.save(path=target_pack_path, overwrite=True)
		elif mode == "resource":
			ctx.assets.save(path=target_pack_path, overwrite=True)

def set_result(hook: str):
	if hook == "on_instance_setup":
		output("set_result", {
			"main_class_override": None,
			"jar_path_override": None,
			"classpath_extension": []
		})
	else:
		output("set_result", "__null__")

def run():
	hook = sys.argv[1]
	if hook != "after_packages_installed" and hook != "on_instance_setup" and hook != "update_world_files":
		print("$_Incorrect hook")
		set_result(hook)
	
	arg_raw = sys.argv[2]

	arg = json.loads(arg_raw)

	# If this is a full instance update we want to weld after packages are installed
	if hook == "on_instance_setup" and "update_depth" in arg and arg["update_depth"] == "full":
		set_result(hook)
		return
	
	if "disable_weld" in arg["config"] and arg["config"]["disable_weld"]:
		set_result(hook)
		return
	
	output("start_process")
	output("message", {
		"contents": {
			"StartProcess": "Welding packs"
		},
		"level": "important"
	})

	# Figure out the paths to load packs into
	if arg["game_dir"] is None:
		set_result(hook)
		return
	
	game_dir = Path(arg["game_dir"])
	datapack_dirs = []
	datapack_folder = arg["config"]["datapack_folder"] if "datapack_folder" in arg["config"] else None
	if datapack_folder is not None:
		datapack_dirs = [game_dir.joinpath(datapack_folder)]
	else:
		if arg["side"] == "client":
			saves_dir = game_dir.joinpath("saves")
			# Trick to only get the immediate subdirectories
			for entry in next(os.walk(saves_dir))[1]:
				path = saves_dir.joinpath(entry).joinpath("datapacks")
				if path.exists():
					datapack_dirs.append(path)

		else:
			datapack_dirs = [game_dir.joinpath("world/datapacks")]

	resourcepack_dirs = [game_dir.joinpath("resourcepacks")]

	weld_ignore = []
	if "weld_ignore" in arg["config"]:
		weld_ignore = arg["config"]["weld_ignore"]

	# Run Weld on each directory
	for dir in datapack_dirs:
		weld_dir(dir, weld_ignore, "data")

	for dir in resourcepack_dirs:
		weld_dir(dir, weld_ignore, "resource")

	output("message", {
		"contents": {
			"Success": "Packs welded",
		},
		"level": "important"
	})
	output("end_process")

	set_result(hook)


def main():
	try:
		run()
	except Exception as e:
		output("message", {
			"contents": {
				"Error": "Failed to weld packs:\n" + str(e),
			},
			"level": "important"
		})

if __name__ == "__main__":
	main()

