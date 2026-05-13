require "spec_helper"

RSpec.describe MyGem do
  it "greets" do
    expect(MyGem.greet("world")).to eq("Hello, world!")
  end

  it "adds" do
    expect(MyGem.add(2, 3)).to eq(5)
  end
end
