Gem::Specification.new do |spec|
  spec.name          = "my_gem"
  spec.version       = "0.1.0"
  spec.summary       = "A sample Ruby gem"
  spec.description   = "A sample Ruby gem for demonstration purposes"
  spec.license       = "MIT"
  spec.authors       = ["MyOrg"]
  spec.homepage      = "https://github.com/myorg/my_gem"

  spec.files         = Dir.glob("lib/**/*.rb")
  spec.require_paths = ["lib"]
  spec.required_ruby_version = ">= 3.0"
end
