import pytest

import pyfulgur
from pyfulgur import AssetBundle


def test_render_error_is_exception_class():
    assert isinstance(pyfulgur.RenderError, type)
    assert issubclass(pyfulgur.RenderError, Exception)


def test_missing_css_file_raises_file_not_found(tmp_path):
    bundle = AssetBundle()
    with pytest.raises(FileNotFoundError):
        bundle.add_css_file(str(tmp_path / "nope.css"))


def test_font_file_missing_raises_file_not_found(tmp_path):
    bundle = AssetBundle()
    with pytest.raises(FileNotFoundError):
        bundle.add_font_file(str(tmp_path / "nope.ttf"))
