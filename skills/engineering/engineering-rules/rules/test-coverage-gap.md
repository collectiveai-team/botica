---
title: Treat untested code as a gap — add tests when behavior changes
section: test
scope: general
applies-to: all
status: current
tags: test, coverage, gap, ci
---

## Treat untested code as a gap — add tests when behavior changes

Untested code is a liability. The default posture is: any behavior change (refactor, new
endpoint, bug fix) must ship with a test that encodes the expected behavior. Do not wait
for a separate "testing pass" — tests are part of the change.

**The constraint that matters most:** default tests must not require live cloud services,
model downloads, credentials, or any network access. Tests that need these must be gated
behind a pytest marker and skipped in CI unless the relevant service is explicitly
provisioned.

**Required when you touch behavior:**

```python
# Add or update a test whenever you:
# - refactor a module's internal seam (callers should stay green)
# - add a new public function or API endpoint
# - fix a bug (the test encodes the regression)

@pytest.mark.parametrize("name,expected", [("NDA", 201), ("", 422)])
def test_create_contract_validation(client: TestClient, name: str, expected: int):
    resp = client.post("/contracts/", json={"name": name, "content": "x"})
    assert resp.status_code == expected
```

**Avoid:**

```python
# Tests that unconditionally download a model or call a cloud provider
def test_transcribe():
    model = whisper.load_model("base.en")   # downloads ~150 MB at test time
    result = model.transcribe("audio.wav")
    assert result["text"]
```

Gate resource-heavy tests with a marker:

```python
@pytest.mark.slow
def test_transcribe_with_model():
    ...
```

And skip by default in `pyproject.toml`:

```toml
[tool.pytest.ini_options]
addopts = "-m 'not slow'"
```

Reference: `test-in-memory-adapters` for the fixture pattern that keeps tests fast and
dependency-free.
