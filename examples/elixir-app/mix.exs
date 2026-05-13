defmodule ElixirApp.MixProject do
  use Mix.Project

  def project do
    [
      app: :elixir_app,
      version: "1.0.0",
      elixir: "~> 1.14",
      deps: deps()
    ]
  end

  def application do
    [
      extra_applications: [:logger]
    ]
  end

  defp deps do
    [
      {:ex_doc, "~> 0.29", only: :dev, runtime: false}
    ]
  end
end
