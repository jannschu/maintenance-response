import time
import random
from pathlib import Path
from typing import Any

import pytest
import requests
import yaml

TRAEFIK_URL = "http://localhost"

DYNAMIC_FILE_REPO = Path(".github/traefik/dynamic/dynamic.yml")

DYNAMIC = Path(__file__).parent
while DYNAMIC.parent and not (DYNAMIC / DYNAMIC_FILE_REPO).exists():
    DYNAMIC = DYNAMIC.parent
if not DYNAMIC.parent:
    raise RuntimeError("Could not find dynamic.yml in the repository.")
DYNAMIC = DYNAMIC / DYNAMIC_FILE_REPO

CONTENT_DIRECTORY = DYNAMIC.parent.parent / "maintenance"


def pytest_configure(config):
    for _ in range(10):
        try:
            res = requests.get(TRAEFIK_URL)
            if res.status_code == 200:
                break
        except requests.ConnectionError:
            pass
        time.sleep(0.25)
    else:
        raise RuntimeError("Traefik is not running, please start it before running the tests.")


@pytest.fixture()
def plugin():
    middleware_name = "maintenance"
    plugin_name = "maintenance-response"
    cleanup = []

    def _configure(enabled=True, content: None | dict[str, str | bytes | None] = None, only_if: str | None = None):
        with DYNAMIC.open("rb") as f:
            dynamic = yaml.safe_load(f)

        el = dynamic
        for key in ("http", "middlewares", middleware_name, "plugin"):
            if key not in el:
                el[key] = {}
            el = el[key]

        new: dict[str, Any] = {
            # Traefik does not properly handle booleans in YAML <-> JSON?
            "enabled": "true" if enabled else "false",
        }
        if content is not None:
            new["content"] = content
        if only_if is not None:
            new["onlyIf"] = only_if

        # prepare content
        if content is not None:
            new["content"] = []
            while True:
                dir = CONTENT_DIRECTORY / random.randbytes(20).hex()
                if dir.exists():
                    continue
                dir.mkdir()
                cleanup.append(dir)
                break
            for name, content_value in (content or {}).items():
                new["content"].append(f"/maintenance/{dir.name}/{name}")
                if content_value is None:
                    continue
                out = dir / name
                if isinstance(content_value, str):
                    out.write_text(content_value)
                elif isinstance(content_value, bytes):
                    out.write_bytes(content_value)
                else:
                    raise ValueError(f"Content for {name} must be bytes or str, got {type(content_value)}")

        el[plugin_name] = new

        with DYNAMIC.open("w") as f:
            yaml.safe_dump(dynamic, f, default_flow_style=False, sort_keys=False)

        for _ in range(10):
            middleware = get_middleware_config(f"{middleware_name}@file")
            if middleware is not None:
                config = middleware["plugin"][plugin_name]
                if config == new:
                    break
                print(config)
            time.sleep(0.1)
        else:
            raise RuntimeError(f"Middleware {middleware_name}@file not found or not updated after 5 attempts.")

    with DYNAMIC.open("rb") as f:
        original_dynamic = f.read()
    try:
        yield _configure
    finally:
        with DYNAMIC.open("wb") as f:
            f.write(original_dynamic)


def get_middleware_config(name: str) -> dict[str, Any] | None:
    response = requests.get(f"{TRAEFIK_URL}:8080/api/http/middlewares/{name}")
    return response.json() if response.status_code == 200 else None


class Query:
    def __init__(self, path="/", method="GET"):
        if not path.startswith("/"):
            path = "/" + path
        self.response = requests.request(method, TRAEFIK_URL + path)

    def maintenance(self) -> bool:
        return self.response.status_code == 503

    @property
    def text(self) -> str:
        return self.response.text
