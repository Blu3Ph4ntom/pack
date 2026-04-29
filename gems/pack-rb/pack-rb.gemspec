lib_dir = File.expand_path("lib", __dir__)
$LOAD_PATH.unshift(lib_dir) unless $LOAD_PATH.include?(lib_dir)

require "pack/rb/version"

Gem::Specification.new do |spec|
  spec.name = "pack-rb"
  spec.version = Pack::RB::VERSION
  spec.authors = ["Hemanth", "Piper Software"]
  spec.email = ["opensource@piper.dev"]

  spec.summary = "Install and launch the Pack Rust binary from RubyGems."
  spec.description = "pack-rb installs a Ruby wrapper executable named `pack` that downloads the matching Pack release binary for the current platform and then hands control to it."
  spec.homepage = "https://github.com/Blu3Ph4ntom/pack"
  spec.license = "MIT"
  spec.required_ruby_version = ">= 3.1"

  spec.metadata = {
    "homepage_uri" => spec.homepage,
    "source_code_uri" => spec.homepage,
    "changelog_uri" => "#{spec.homepage}/releases",
    "rubygems_mfa_required" => "true"
  }

  spec.files = Dir.chdir(__dir__) do
    Dir["lib/**/*", "exe/*", "README.md", "LICENSE.txt"]
  end
  spec.bindir = "exe"
  spec.executables = ["pack"]
  spec.require_paths = ["lib"]
  spec.post_install_message = <<~MSG
    pack-rb installed the `pack` launcher.
    First run will download the matching Pack binary from GitHub Releases.

    Example:
      pack --help
  MSG
end
