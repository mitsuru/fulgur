Gem::Specification.new do |s|
  s.name        = "fulgur"
  s.version     = "0.0.1"
  s.summary     = "Ruby bindings for fulgur — offline HTML/CSS to PDF conversion"
  s.description = <<~DESC
    Ruby bindings for fulgur, an offline, deterministic HTML/CSS to PDF
    conversion library written in Rust. This is a name reservation;
    the implementation is under active development.
  DESC
  s.authors     = ["Mitsuru Hayasaka"]
  s.email       = "hayasaka.mitsuru@gmail.com"
  s.homepage    = "https://github.com/mitsuru/fulgur"
  s.licenses    = ["MIT", "Apache-2.0"]
  s.metadata    = {
    "source_code_uri"   => "https://github.com/mitsuru/fulgur",
    "changelog_uri"     => "https://github.com/mitsuru/fulgur/blob/main/CHANGELOG.md",
    "bug_tracker_uri"   => "https://github.com/mitsuru/fulgur/issues",
  }

  s.required_ruby_version = ">= 3.1"
  s.files = ["lib/fulgur.rb", "README.md", "LICENSE-MIT"]
end
