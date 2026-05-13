defmodule ElixirAppTest do
  use ExUnit.Case

  test "greets by name" do
    assert ElixirApp.greet("world") == "Hello, world!"
  end

  test "adds two numbers" do
    assert ElixirApp.add(2, 3) == 5
  end
end
