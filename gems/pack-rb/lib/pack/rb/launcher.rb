require "fileutils"
require "net/http"
require "rbconfig"
require "tempfile"
require "tmpdir"
require "uri"

module Pack
  module RB
    class Launcher
      class Error < StandardError; end

      class << self
        def run(argv = ARGV)
          new(argv).run
        end
      end

      def initialize(argv)
        @argv = argv
      end

      def run
        binary = ensure_binary!
        exec(binary, *@argv)
      rescue SystemCallError => e
        raise Error, "failed to launch pack binary: #{e.message}"
      end

      private

      def ensure_binary!
        path = installed_binary_path
        return path if File.exist?(path)

        FileUtils.mkdir_p(File.dirname(path))
        download_to(path)
        FileUtils.chmod("+x", path) unless windows?
        path
      end

      def download_to(path)
        url = download_url
        uri = URI.parse(url)
        Net::HTTP.start(uri.host, uri.port, use_ssl: uri.scheme == "https") do |http|
          request = Net::HTTP::Get.new(uri.request_uri)
          http.request(request) do |response|
            unless response.is_a?(Net::HTTPSuccess)
              raise Error, "download failed with HTTP #{response.code} from #{url}"
            end

            Tempfile.create(["pack-rb", windows? ? ".exe" : ""], File.dirname(path)) do |tmp|
              response.read_body { |chunk| tmp.write(chunk) }
              tmp.flush
              FileUtils.mv(tmp.path, path)
            end
          end
        end
      rescue SocketError, IOError, SystemCallError => e
        raise Error, "failed to download pack binary: #{e.message}"
      end

      def download_url
        "#{download_base_url}/#{asset_name}"
      end

      def download_base_url
        ENV["PACK_RB_DOWNLOAD_BASE_URL"] || "https://github.com/#{github_repository}/releases/download/v#{pack_version}"
      end

      def github_repository
        ENV["PACK_RB_GITHUB_REPOSITORY"] || "Blu3Ph4ntom/pack"
      end

      def installed_binary_path
        File.join(install_dir, pack_version, asset_name)
      end

      def install_dir
        ENV["PACK_RB_INSTALL_DIR"] || File.join(Dir.home, ".pack-rb", "bin")
      end

      def pack_version
        ENV["PACK_RB_VERSION"] || VERSION
      end

      def asset_name
        "pack-#{target_triple}#{windows? ? '.exe' : ''}"
      end

      def target_triple
        @target_triple ||= begin
          cpu = normalize_cpu
          os = RbConfig::CONFIG["host_os"]

          case os
          when /linux/
            raise Error, "unsupported CPU for Linux: #{cpu}" unless cpu == "x86_64"
            "x86_64-unknown-linux-gnu"
          when /darwin/
            "#{cpu}-apple-darwin"
          when /mswin|mingw|cygwin/
            raise Error, "unsupported CPU for Windows: #{cpu}" unless cpu == "x86_64"
            "x86_64-pc-windows-msvc"
          else
            raise Error, "unsupported platform: #{RbConfig::CONFIG["host_cpu"]} #{os}"
          end
        end
      end

      def normalize_cpu
        case RbConfig::CONFIG["host_cpu"]
        when "x86_64", "amd64"
          "x86_64"
        when "arm64", "aarch64"
          "aarch64"
        else
          raise Error, "unsupported CPU: #{RbConfig::CONFIG["host_cpu"]}"
        end
      end

      def windows?
        (/mswin|mingw|cygwin/ =~ RbConfig::CONFIG["host_os"]) != nil
      end
    end
  end
end
