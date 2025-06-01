from importlib.metadata import version, PackageNotFoundError
import os
from pathlib import Path
from platform import python_version
import shutil
import subprocess
import subprocess
import sys
env = os.environ.copy()
PLATFORMS = {
    "linux": "manylinux2014_x86_64",
    "windows": "win_amd64",
}
PLATFORM = "windows" if os.name == "nt" else "linux"
VERSION = python_version()
UPDATE_VERSIONS = os.environ.get("UPDATE_VERSIONS", None) is not None
REQ_PATH = Path('pyfunctions/requirements_pinned_%s.txt' % PLATFORM)
REQ_TORCHLESS_PATH = Path('pyfunctions/.requirements_pinned_%s.txt' % PLATFORM)
def create_fixed_requirements():
    with open("pyfunctions/requirements.txt", "r") as fp:
        packages = [x.strip() for x in fp.readlines()]
    versions = []

    for x in packages:
        try:
            versions.append(version(x.split("==")[0]))
        except PackageNotFoundError:
            versions.append(None)

    with open(REQ_PATH, "w") as fp:
        fp.writelines([f"{x.split('==')[0]}=={y}\n" if y is not None else f"{x.split('==')[0]}\n" for (x, y) in zip(packages, versions)])

    with open(REQ_TORCHLESS_PATH, "w") as fp:
        fp.writelines([f"{x.split('==')[0]}=={y}\n" if y is not None else f"{x.split('==')[0]}\n" for (x, y) in zip(packages, versions) if not "torch" in x])

def run_pip_download(platform_tag, python_version, requirements, output_dir):
    print(f"Downloading wheels for platform: {platform_tag} and Python {python_version}...")

    command = [
        sys.executable, "-m", "pip", "download",
        "--platform", PLATFORMS[platform_tag],
        "--implementation", "cp",
        "--python-version", python_version,
        "--only-binary=:all:",
        "--dest", output_dir,
        "--requirement", requirements,
        "--no-deps"
    ]

    subprocess.run(command, check=True, env=env)

def install_required_packages():
    paths = list(Path("deps/pypackages").glob("*.whl"))

    for p in paths:
        print(p)
        subprocess.run(["pip", "install", str(p)])


if __name__ == "__main__":
    out = Path("deps/pypackages")
    if not out.exists():
        os.makedirs(out)
    else:
        shutil.rmtree(out)

    if UPDATE_VERSIONS or not REQ_TORCHLESS_PATH.exists():
        create_fixed_requirements()
    run_pip_download(PLATFORM, VERSION, REQ_TORCHLESS_PATH, str(out))
    install_required_packages()