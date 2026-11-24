from __future__ import annotations

import os
import pathlib
import platform
import tarfile
import zipfile


ROOT = pathlib.Path(__file__).resolve().parents[2]
DIST = ROOT / "dist"
WINDOWS = platform.system() == "Windows"
BINARY = ROOT / "target" / "release" / ("ali.exe" if WINDOWS else "ali")
ASSET_NAME = "ali.exe" if WINDOWS else "ali"
ARCHIVE_NAME = f"ali-{platform.system().lower()}-{platform.machine().lower()}"
INCLUDED = [
    (BINARY, ASSET_NAME),
    (ROOT / "LICENSE", "LICENSE"),
    (ROOT / "README.md", "README.md"),
    (ROOT / "README.it.md", "README.it.md"),
    (ROOT / "README.fr.md", "README.fr.md"),
    (ROOT / "README.es.md", "README.es.md"),
    (ROOT / "config", "config"),
    (ROOT / "docs", "docs"),
]


def files() -> list[tuple[pathlib.Path, str]]:
    result = []
    for source, destination in INCLUDED:
        if not source.exists():
            raise FileNotFoundError(source)
        if source.is_file():
            result.append((source, destination))
        else:
            result.extend(
                (item, str(pathlib.Path(destination) / item.relative_to(source)))
                for item in sorted(source.rglob("*"))
                if item.is_file()
            )
    return result


def main() -> None:
    DIST.mkdir(exist_ok=True)
    entries = files()
    if WINDOWS:
        archive = DIST / f"{ARCHIVE_NAME}.zip"
        with zipfile.ZipFile(archive, "w", zipfile.ZIP_DEFLATED) as output:
            for source, destination in entries:
                output.write(source, pathlib.Path(ARCHIVE_NAME) / destination)
    else:
        archive = DIST / f"{ARCHIVE_NAME}.tar.gz"
        with tarfile.open(archive, "w:gz") as output:
            for source, destination in entries:
                info = output.gettarinfo(str(source), str(pathlib.Path(ARCHIVE_NAME) / destination))
                info.uid = info.gid = 0
                info.uname = info.gname = ""
                info.mtime = int(os.environ.get("SOURCE_DATE_EPOCH", "0"))
                with source.open("rb") as content:
                    output.addfile(info, content)


if __name__ == "__main__":
    main()
