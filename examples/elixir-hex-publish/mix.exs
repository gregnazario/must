defmodule MyElixirLib.MixProject do
  use Mix.Project

  @version "0.1.0"

  def project do
    [
      app: :my_elixir_lib,
      version: @version,
      elixir: "~> 1.16",
      start_permanent: Mix.env() == :prod,
      deps: deps(),
      package: package(),
      description: "A sample Elixir library"
    ]
  end

  def application do
    []
  end

  defp deps do
    [{:ex_doc, "~> 0.34", only: :dev, runtime: false}]
  end

  defp package do
    [
      licenses: ["MIT"],
      links: %{"GitHub" => "https://github.com/myorg/my_elixir_lib"}
    ]
  end
end
