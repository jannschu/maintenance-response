from tests.conftest import Query


def test_enable(plugin):
    plugin(enabled=True)
    assert Query().maintenance()


def test_disable(plugin):
    plugin(enabled=False)
    assert not Query().maintenance()
