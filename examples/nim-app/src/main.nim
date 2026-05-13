import std/strformat

proc greet(name: string): string =
  &"Hello, {name}!"

when isMainModule:
  let name = paramStr(0).extractFilename
  echo greet("world")
