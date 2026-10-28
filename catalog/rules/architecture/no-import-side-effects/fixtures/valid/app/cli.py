import subprocess
import sys


def main() -> int:
    return subprocess.run(["git", "status"], check=False).returncode


if __name__ == "__main__":
    with open("cli.log", "a") as log:
        log.write("start\n")
    sys.exit(main())
