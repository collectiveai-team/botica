from subprocess import check_output

COMMIT = check_output(["git", "rev-parse", "HEAD"], text=True).strip()
