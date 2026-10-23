import asyncio
import os
import subprocess


def test_the_file_downloads():
    subprocess.run(["/usr/bin/curl", "-o", "out.json", "https://example.com"], check=True)


def test_the_tag_is_read():
    assert subprocess.check_output(("git", "describe")).strip()


def test_the_report_is_printed():
    os.system("lpr report.txt")


async def test_the_server_answers():
    process = await asyncio.create_subprocess_exec("/usr/bin/curl", "https://example.com")
    assert await process.wait() == 0
