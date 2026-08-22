"""`toboggan_py.pyi` still describes the module that is actually built.

The stubs are written by hand, and nothing about a PyO3 build checks them
against the extension they document — a property renamed in Rust leaves the
stub confidently wrong, and every editor and type-checker downstream repeats
the lie. This is that check.

It compares in both directions on purpose. A member missing from the stub is an
undocumented API; a member only in the stub is a promise the module does not
keep, which is the worse of the two because it type-checks.

`mypy.stubtest` (in `mise check:py`) goes further — annotations, defaults,
property-versus-method — and where the two overlap it is the better check. Two
things keep this file worth having. It runs in the ordinary test suite, with no
type-checker involved; and the constructor check below is the *only* coverage of
the constructor's shape, because pyo3 spells it `__new__` while the stub spells
it `__init__` and stubtest has to be told to ignore that (see
`stubtest-allowlist.txt`).
"""

import ast
import os
import pathlib

import pytest

import toboggan_py

STUB = os.path.join(
    os.path.dirname(os.path.dirname(os.path.abspath(__file__))), "toboggan_py.pyi"
)

# Inherited from `object` or supplied by PyO3 for every class; documenting them
# would be noise, and their absence from the stub is not drift.
INHERITED = {
    "__class__",
    "__delattr__",
    "__dict__",
    "__dir__",
    "__doc__",
    "__eq__",
    "__format__",
    "__ge__",
    "__getattribute__",
    "__getstate__",
    "__gt__",
    "__hash__",
    "__init__",
    "__init_subclass__",
    "__le__",
    "__lt__",
    "__module__",
    "__ne__",
    "__new__",
    "__qualname__",
    "__reduce__",
    "__reduce_ex__",
    "__setattr__",
    "__sizeof__",
    "__subclasshook__",
    "__weakref__",
}


STUB_TREE = ast.parse(pathlib.Path(STUB).read_text(encoding="utf-8"))


def _declared():
    """Every class in the stub, with the members it declares."""
    return {
        node.name: {
            item.name
            for item in node.body
            if isinstance(item, (ast.FunctionDef, ast.AsyncFunctionDef))
        }
        for node in STUB_TREE.body
        if isinstance(node, ast.ClassDef)
    }


def _built():
    """Every exported class in the built module, with the members it has."""
    built = {}
    for name in toboggan_py.__all__:
        cls = getattr(toboggan_py, name)
        members = {attr for attr in vars(cls) if attr not in INHERITED}
        # A PyO3 class gets a `__text_signature__` only when it has `#[new]`.
        # Probing by calling `cls()` instead would construct a real client and
        # connect it to whatever server happens to be running — a check must not
        # have side effects, least of all that one.
        if getattr(cls, "__text_signature__", None) is not None:
            members.add("__init__")
        built[name] = members
    return built


def test_the_stub_and_the_module_declare_the_same_classes():
    assert set(_declared()) == set(_built())


@pytest.mark.parametrize("name", sorted(toboggan_py.__all__))
def test_the_stub_and_the_module_declare_the_same_members(name):
    declared, built = _declared()[name], _built()[name]

    assert not (built - declared), (
        f"{name}: in the module but not in the stub: {sorted(built - declared)}"
    )
    assert not (declared - built), (
        f"{name}: in the stub but not in the module: {sorted(declared - built)}"
    )


def test_the_constructor_takes_the_parameters_the_stub_promises():
    signature = toboggan_py.Toboggan.__text_signature__ or ""
    built = [
        argument.split("=")[0].strip()
        for argument in signature.strip("()").split(",")
        if argument.strip()
    ]

    declared = next(
        [argument.arg for argument in item.args.args if argument.arg != "self"]
        for node in STUB_TREE.body
        if isinstance(node, ast.ClassDef) and node.name == "Toboggan"
        for item in node.body
        if isinstance(item, ast.FunctionDef) and item.name == "__init__"
    )

    assert declared == built


def test_every_exported_class_is_in_dunder_all():
    """`__all__` is what maturin's generated `__init__.py` re-exports, so a class
    missing from it is invisible to `from toboggan_py import *` no matter what
    the stub says."""
    exported = {
        name
        for name, value in vars(toboggan_py).items()
        if isinstance(value, type) and not name.startswith("_")
    }
    assert exported == set(toboggan_py.__all__)
