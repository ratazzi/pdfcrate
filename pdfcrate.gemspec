require_relative 'lib/pdfcrate/version'

Gem::Specification.new do |spec|
  spec.name          = "pdfcrate"
  spec.version       = Pdfcrate::VERSION
  spec.authors       = ["ratazzi"]
  spec.email         = ["ratazzi@gmail.com"]

  spec.summary       = "Ruby bindings for pdfcrate PDF generation library"
  spec.description   = "A Ruby gem providing Prawn-compatible API bindings to the pdfcrate Rust PDF generation library using Magnus."
  spec.homepage      = "https://github.com/ratazzi/pdfcrate"
  spec.license       = "MIT"
  spec.required_ruby_version = ">= 3.3.0"

  spec.files = Dir[
    'lib/**/*.rb',
    'ext/**/*.{rb,rs,toml}',
    'ext/**/src/**/*.rs',
    'README.md',
    'LICENSE',
    'Gemfile',
    '*.gemspec'
  ]

  spec.files += Dir['lib/**/*.{bundle,so,dll}'] if spec.respond_to?(:platform) && spec.platform != "ruby"

  spec.require_paths = ["lib"]

  spec.extensions = ["ext/pdfcrate/extconf.rb"] if !spec.respond_to?(:platform) || spec.platform == "ruby"

  if !spec.respond_to?(:platform) || spec.platform == "ruby"
    spec.add_development_dependency "rake", "~> 13.0"
    spec.add_development_dependency "rake-compiler", "~> 1.2"
  end

  spec.add_dependency "rb_sys", "~> 0.9.87"

  spec.metadata = {
    "source_code_uri" => "https://github.com/ratazzi/pdfcrate",
    "rubygems_mfa_required" => "true"
  }
end
