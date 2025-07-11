from tests.conftest import Query


def test_fallback_if_no_content(plugin):
    plugin(enabled=True)
    response = Query()
    assert response.maintenance()
    assert response.text == "Service unavailable due to maintenance"
    assert response.headers["Content-Type"] == "text/plain"


def test_html(plugin):
    plugin(enabled=True, content={"foo.html": "<h1>Maintenance</h1>", "ok.txt": "Maintenance"})
    response = Query(headers={"Accept": "text/html"})
    assert response.maintenance()
    assert response.text == "<h1>Maintenance</h1>"
    assert response.headers["Content-Type"] == "text/html"


def test_prefer(plugin):
    plugin(enabled=True, content={"foo.html": "<h1>Maintenance</h1>", "ok.txt": "Maintenance"})
    response = Query(headers={"Accept": "text/plain;q=0.1, text/html"})
    assert response.maintenance()
    assert response.text == "<h1>Maintenance</h1>"
    assert response.headers["Content-Type"] == "text/html"

    response = Query(headers={"Accept": "text/plain;q=0.1, text/html;q=0.05"})
    assert response.maintenance()
    assert response.text == "Maintenance"
    assert response.headers["Content-Type"] == "text/plain"
