import sys
from pathlib import Path

# We import is a bit messier and could simply
# be:
# from ..model import add, mul, predict
# if we only wanted to use pytest.
# The below import enables us to use mut.py
# as well as pytest:
src = Path(__file__).parents[0] / ".." / "."
print(src)
sys.path.append(str(src))

from model import add, mul, predict


def test_add():
    result = add(2, 2)


def test_mul():
    result = mul(2, 2)
    assert result == 4


if __name__ == "__main__":
    main()
