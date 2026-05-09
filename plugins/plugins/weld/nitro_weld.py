import sys
import json
from pathlib import Path
import os
import traceback
import importlib

def output(method: str, data: object | str | None = None):
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

# Gets the lockfile path
def get_lockfile_path(inst_dir: Path) -> Path:
	return inst_dir.joinpath("nitro_lock.json")

# Opens the lockfile for the instance
def open_lockfile(inst_dir: Path) -> object | None:
	lock_path = get_lockfile_path(inst_dir)

	if not lock_path.exists():
		return None
	
	with open(lock_path, "r") as lockfile:
		return json.load(lockfile)

# Writes the lockfile for the instance
def save_lockfile(lockfile: object, inst_dir: Path | None):
	lock_path = get_lockfile_path(inst_dir)
	
	if not lock_path.exists():
		return
	
	with open(lock_path, "w") as file:
		json.dump(lockfile, file)

# Updates the lockfile for the instance, moving an old addon path to a new one
def update_lockfile(lockfile: object, old_path: Path, new_path: Path):
	if not "addons" in lockfile:
		return

	for entry in lockfile["addons"]:
		if not "files" in entry:
			continue
		for i in range(len(entry["files"])):
			if entry["files"][i] == str(old_path):
				entry["files"][i] = str(new_path)

# Welds a single datapack / resourcepack directory. Mode should be either "data" or "resource".
def weld_dir(dir: Path, ignore: list, mode: str, lockfile: object | None, mc_version: str):
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

			if lockfile is not None:
				update_lockfile(lockfile, path, target)

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
		"output": str(dir),
		"minecraft": mc_version
	}

	packs = [str(unwelded_path.joinpath(x)) for x in os.listdir(unwelded_path)]
	target_pack_path = dir.joinpath("Welded Packs.zip")

	if len(packs) == 0 and not target_pack_path.exists():
		return
	
	# Lazy import to reduce visible startup time
	weld = importlib.import_module("weld", "smithed")
	
	with weld.run_weld(packs=packs,config=beet_config,directory=dir) as ctx:
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
	if hook != "after_packages_installed" and hook != "update_world_files":
		print("$_Incorrect hook")
		set_result(hook)
	
	arg_raw = sys.argv[2]

	arg = json.loads(arg_raw)

	if hook == "after_packages_installed" and arg["update_depth"] == "shallow":
		set_result(hook)
		return
	
	if "disable_weld" in arg["config"] and arg["config"]["disable_weld"]:
		set_result(hook)
		return
	
	is_debug = hook == "update_world_files"
	output_level = "debug" if is_debug else "important"

	if not is_debug:
		output("start_process")
	output("message", {
		"contents": {
			"StartProcess": "Welding packs"
		},
		"level": output_level
	})

	# Figure out the paths to load packs into
	if arg["inst_dir"] is None:
		set_result(hook)
		return
	
	inst_dir = Path(arg["inst_dir"])
	datapack_dirs = []
	datapack_folder = arg["config"]["datapack_folder"] if "datapack_folder" in arg["config"] else None
	if datapack_folder is not None:
		datapack_dirs = [inst_dir.joinpath(datapack_folder)]
	else:
		if arg["side"] == "client":
			saves_dir = inst_dir.joinpath("saves")
			# Trick to only get the immediate subdirectories
			for entry in next(os.walk(saves_dir))[1]:
				path = saves_dir.joinpath(entry).joinpath("datapacks")
				if path.exists():
					datapack_dirs.append(path)

		else:
			datapack_dirs = [inst_dir.joinpath("world/datapacks")]

	resourcepack_dirs = [inst_dir.joinpath("resourcepacks")]

	weld_ignore = []
	if "weld_ignore" in arg["config"]:
		weld_ignore = arg["config"]["weld_ignore"]

	lockfile = open_lockfile(inst_dir)
	mc_version = arg["version_info"]["version"]

	# Run Weld on each directory
	for dir in datapack_dirs:
		weld_dir(dir, weld_ignore, "data", lockfile, mc_version)

	for dir in resourcepack_dirs:
		weld_dir(dir, weld_ignore, "resource", lockfile, mc_version)

	if lockfile is not None:
		save_lockfile(lockfile, inst_dir)

	output("message", {
		"contents": {
			"Success": "Packs welded",
		},
		"level": output_level
	})
	if not is_debug:
		output("end_process")

	set_result(hook)


def main():
	try:
		run()
	except Exception as e:
		output("set_error", "Failed to weld packs:\n" + ''.join(traceback.format_exception(e)))

if __name__ == "__main__":
	main()

