require_relative "../lib/ruby_app"

RSpec.describe RubyApp do
  it "greets by name" do
    expect(RubyApp.greet("world")).to eq("Hello, world!")
  end

  it "adds two numbers" do
    expect(RubyApp.add(2, 3)).to eq(5)
  end
end
