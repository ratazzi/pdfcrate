require "bundler/gem_tasks"
require "rake/extensiontask"

task default: :compile

Rake::ExtensionTask.new("pdfcrate") do |ext|
  ext.lib_dir = "lib/pdfcrate"
  ext.cross_platform = [
    'x86_64-linux', 'x86_64-linux-musl',
    'aarch64-linux', 'aarch64-linux-musl',
    'x86_64-darwin', 'arm64-darwin',
    'x64-mingw-ucrt'
  ]
  ext.cross_compile = true
end

ENV['RB_SYS_CARGO_PROFILE'] = 'release'

task :native => :compile
task :gem => :build
