require_relative "my_gem/version"

module MyGem
  class Error < StandardError; end

  def self.greet(name)
    "Hello, #{name}!"
  end

  def self.add(a, b)
    a + b
  end
end
