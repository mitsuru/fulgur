# frozen_string_literal: true

require_relative "lib/fulgur/version"

Gem::Specification.new do |spec|
  spec.name = "fulgur"
  spec.version = Fulgur::VERSION
  spec.authors = ["Mitsuru Hayasaka"]
  spec.email = ["hayasaka.mitsuru@gmail.com"]

  spec.summary = "Offline HTML/CSS → PDF conversion"
  spec.description = "Ruby bindings for fulgur, a deterministic HTML/CSS to PDF rendering engine."
  spec.homepage = "https://github.com/mitsuru/fulgur"
  spec.license = "Apache-2.0"
  spec.required_ruby_version = ">= 3.3.0"

  spec.metadata["allowed_push_host"] = "https://rubygems.org"
  spec.metadata["source_code_uri"] = "https://github.com/mitsuru/fulgur"

  spec.files = Dir["lib/**/*.rb", "ext/**/*.{rs,toml,rb}", "Cargo.toml", "README.md"]
  spec.require_paths = ["lib"]
  spec.extensions = ["ext/fulgur/extconf.rb"]

  spec.add_dependency "rb_sys", "~> 0.9"
end
