defmodule MyElixirLibTest do
  use ExUnit.Case

  test "greet" do
    assert MyElixirLib.greet("world") == "Hello, world!"
  end

  test "add" do
    assert MyElixirLib.add(2, 3) == 5
  end
end
