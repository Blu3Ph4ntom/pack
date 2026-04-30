require "fileutils"
require "digest"
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
        checksum = expected_checksum_for(asset_name)
        with_download_response(download_url) do |response|
          Tempfile.create(["pack-rb", windows? ? ".exe" : ""], File.dirname(path)) do |tmp|
            tmp.binmode
            response.read_body { |chunk| tmp.write(chunk) }
            tmp.flush
            staged_path = tmp.path
            tmp.close
            verify_checksum!(staged_path, checksum)
            FileUtils.mv(staged_path, path)
          end
        end
      rescue SocketError, IOError, SystemCallError, Timeout::Error => e
        raise Error, "failed to download pack binary: #{e.message}"
      end

      def with_download_response(url, limit = 5, &block)
        raise Error, "too many redirects while downloading #{url}" if limit <= 0

        uri = URI.parse(url)
        Net::HTTP.start(uri.host, uri.port, use_ssl: uri.scheme == "https") do |http|
          http.open_timeout = 15
          http.read_timeout = 120

          request = Net::HTTP::Get.new(uri.request_uri)
          request["User-Agent"] = "pack-rb/#{VERSION}"
          request["Accept"] = "application/octet-stream"

          http.request(request) do |response|
            case response
            when Net::HTTPSuccess
              yield response
            when Net::HTTPRedirection
              location = response["location"]
              raise Error, "redirect missing location for #{url}" unless location

              redirected = URI.join(url, location).to_s
              with_download_response(redirected, limit - 1, &block)
            else
              raise Error, "download failed with HTTP #{response.code} from #{url}"
            end
          end
        end
      end

      def expected_checksum_for(target_asset)
        return nil if ENV["PACK_RB_SKIP_CHECKSUM"] == "1"

        body = +""
        with_download_response("#{download_base_url}/SHA256SUMS") do |response|
          response.read_body { |chunk| body << chunk }
        end

        match = body.each_line.map(&:strip).find { |line| line.end_with?("  #{target_asset}") }
        raise Error, "checksum entry missing for #{target_asset}" unless match

        match.split(/\s+/, 2).first
      end

      def verify_checksum!(path, expected)
        return unless expected

        actual = Digest::SHA256.file(path).hexdigest
        return if actual == expected

        raise Error, "checksum mismatch for downloaded pack binary"
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
        when "x86_64", "amd64", "x64"
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
