require "bundler/gem_tasks"
require "rake/extensiontask"

GEMSPEC = Gem::Specification.load("pdfcrate.gemspec")

task default: :compile

Rake::ExtensionTask.new("pdfcrate", GEMSPEC) do |ext|
  ext.lib_dir = "lib/pdfcrate"
  ext.ext_dir = "ext/pdfcrate"
  ext.cross_compile = true
  ext.cross_platform = %w[
    x86_64-linux x86_64-linux-musl
    aarch64-linux aarch64-linux-musl
    x86_64-darwin arm64-darwin
    x64-mingw-ucrt
  ]
  ext.cross_compiling do |spec|
    spec.dependencies.reject! { |d| d.name == "rb_sys" }
    spec.files.reject! { |f| f.end_with?(".rs") || f.match?(/Cargo\.(toml|lock)$/) }
  end
end

ENV['RB_SYS_CARGO_PROFILE'] = 'release'
