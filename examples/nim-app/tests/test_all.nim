import std/unittest
import ../src/main

suite "main":
  test "greet returns expected message":
    check greet("world") == "Hello, world!"

  test "greet handles empty name":
    check greet("") == "Hello, !"
