from tests.conftest import Query


def test_filter_field_host(plugin):
    plugin(enabled=True, only_if='http.host in { "example.com" "foo.example.com" }')
    assert Query(headers={"Host": "example.com"}).maintenance()

    assert Query(headers={"Host": "foo.example.com"}).maintenance()

    assert not Query(headers={"Host": "example.org"}).maintenance()


def test_filter_field_path(plugin):
    plugin(enabled=True, only_if='not (http.path ~ "^/ci/admin/")')
    assert not Query(path="/ci/admin/").maintenance()
    assert not Query(path="/ci/admin/a/b").maintenance()
    assert Query(path="/ci/home").maintenance()


def test_filter_field_method(plugin):
    plugin(enabled=True, only_if='http.method == "POST" or http.method == "PUT"')
    assert not Query(method="GET").maintenance()
    assert Query(method="POST").maintenance()
    assert Query(method="PUT").maintenance()


def test_filter_field_user_agent(plugin):
    plugin(enabled=True, only_if='http.ua contains "Traefik"')
    assert Query(headers={"User-Agent": "Traefik 3"}).maintenance()
    assert not Query(headers={"User-Agent": "curl/7.64.1"}).maintenance()
