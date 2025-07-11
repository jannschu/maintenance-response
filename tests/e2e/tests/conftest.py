import time
import random
import shutil
from pathlib import Path
from typing import Any

import pytest
import requests
import redis

TRAEFIK_URL = "http://localhost"
CONTENT_DIR_REPO = Path(".github/traefik/maintenance")
ECHO_ROUTER = "http/routers/echo"
REDIS_ROOT_KEY = "traefik"

CONTENT_DIRECTORY = Path(__file__).parent
while CONTENT_DIRECTORY.parent != CONTENT_DIRECTORY and not (CONTENT_DIRECTORY / CONTENT_DIR_REPO).exists():
    CONTENT_DIRECTORY = CONTENT_DIRECTORY.parent
if CONTENT_DIRECTORY.parent == CONTENT_DIRECTORY:
    raise RuntimeError("Could not find dynamic.yml in the repository.")
CONTENT_DIRECTORY = CONTENT_DIRECTORY / CONTENT_DIR_REPO


redis = redis.Redis("localhost")
redis.flushdb()
redis.mset(
    {
        f"{REDIS_ROOT_KEY}/{ECHO_ROUTER}/entrypoints": "http",
        f"{REDIS_ROOT_KEY}/{ECHO_ROUTER}/service": "echo",
        f"{REDIS_ROOT_KEY}/{ECHO_ROUTER}/rule": "PathPrefix(`/ci`)",
        f"{REDIS_ROOT_KEY}/http/services/echo/loadBalancer/servers/port": "80",
        f"{REDIS_ROOT_KEY}/http/services/echo/loadBalancer/servers/url": "http://echo/",
    }
)


def pytest_configure(config):
    for _ in range(10):
        try:
            res = requests.get(TRAEFIK_URL + "/static/ok")
            if res.status_code == 200:
                break
        except requests.ConnectionError:
            pass
        time.sleep(0.25)
    else:
        raise RuntimeError("Traefik is not running, please start it before running the tests.")


@pytest.fixture()
def plugin():
    plugin_name = "maintenance-response"
    cleanup = []

    def _configure(enabled=True, content: None | dict[str, str | bytes | None] = None, only_if: str | None = None):
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

        while True:
            h = random.randbytes(10).hex()
            middleware_name = f"maintenance{h}"
            pattern = f"{REDIS_ROOT_KEY}/http/middlewares/{middleware_name}/*"
            _, keys = redis.scan(match=pattern, count=1)
            if keys:
                continue
            else:
                break

        def flatten(config) -> list[tuple[str, str]] | str:
            result = []
            stack = []
            if isinstance(config, dict):
                for key, value in config.items():
                    stack.append((key, value))
            elif isinstance(config, list):
                for i, item in enumerate(config):
                    stack.append((i, item))
            elif isinstance(config, str | float | int):
                return str(config)
            elif isinstance(config, bool):
                return str(config).lower()
            else:
                raise ValueError(f"Unsupported type {type(config)} in config: {config}")
            for key, value in stack:
                value = flatten(value)
                if isinstance(value, str):
                    result.append((key, value))
                else:
                    for sub_key, sub_value in value:
                        result.append((f"{key}/{sub_key}", sub_value))
            return result

        if new:
            tuples = flatten(new)
            assert isinstance(tuples, list), "Flattening did not produce a list of tuples."
            flattened = dict(tuples)
            assert len(flattened) == len(tuples), "Flattening produced duplicate keys."
        else:
            flattened = {"": ""}

        route = f"{REDIS_ROOT_KEY}/{ECHO_ROUTER}/middlewares"

        pipe = redis.pipeline()
        redis_keys = {
            f"{REDIS_ROOT_KEY}/http/middlewares/{middleware_name}/plugin/{plugin_name}/{k}".removesuffix("/"): v
            for k, v in flattened.items()
        }
        redis_keys[f"{route}/0"] = middleware_name + "@redis"
        pipe.mset(redis_keys)
        pipe.execute()

        for _ in range(10):
            middleware = get_middleware_config(f"{middleware_name}@redis")
            if middleware is not None:
                config = middleware["plugin"][plugin_name]
                if "content" in config:
                    config["content"] = config["content"].split(",")
                if config == new:
                    break
            time.sleep(0.1)
        else:
            raise RuntimeError(f"Middleware {middleware_name} found or not updated after 5 attempts.")

    try:
        yield _configure
    finally:
        for dir in cleanup:
            if dir.exists():
                shutil.rmtree(dir)


def get_middleware_config(name: str) -> dict[str, Any] | None:
    response = requests.get(f"{TRAEFIK_URL}:8080/api/http/middlewares/{name}")
    return response.json() if response.status_code == 200 else None


class Query:
    def __init__(self, path="/ci/", method="GET", **kwargs):
        if not path.startswith("/"):
            path = "/" + path
        self.response = requests.request(method, TRAEFIK_URL + path, **kwargs)

    def maintenance(self) -> bool:
        return self.response.status_code == 503

    @property
    def text(self) -> str:
        return self.response.text

    @property
    def headers(self):
        return self.response.headers
