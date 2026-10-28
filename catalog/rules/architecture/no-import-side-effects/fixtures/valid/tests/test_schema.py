import json
from pathlib import Path

SCHEMA = json.loads(Path(__file__).with_name("schema.json").read_text())


def test_the_schema_has_a_title():
    assert SCHEMA["title"]
