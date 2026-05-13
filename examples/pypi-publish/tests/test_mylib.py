from mylib import greet, add

def test_greet():
    assert greet("world") == "Hello, world!"

def test_add():
    assert add(2, 3) == 5
