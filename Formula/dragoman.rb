class Dragoman < Formula
  desc "PID redirection and content negotiation server"
  homepage "https://github.com/front-matter/dragoman"
  url "https://github.com/front-matter/dragoman/archive/refs/tags/v0.1.0.tar.gz"
  sha256 "REPLACE_WITH_SHA256_OF_RELEASE_TARBALL"
  license "MIT"
  head "https://github.com/front-matter/dragoman.git", branch: "main"

  depends_on "rust" => :build

  def install
    system "cargo", "install", *std_cargo_args
  end

  def post_install
    (var/"dragoman").mkpath
    (var/"log").mkpath
  end

  service do
    run [opt_bin/"dragoman", "start"]
    keep_alive true
    log_path var/"log/dragoman.log"
    error_log_path var/"log/dragoman.log"
    environment_variables(
      PORT: "3000",
      DRAGOMAN_DB: "#{var}/dragoman/commonmeta.sqlite3",
      RUST_LOG: "dragoman=info",
    )
    working_dir var/"dragoman"
  end

  test do
    port = free_port
    pid = fork do
      ENV["PORT"] = port.to_s
      exec bin/"dragoman", "start"
    end
    sleep 1
    assert_match "dragoman", shell_output("curl -s http://127.0.0.1:#{port}/")
  ensure
    Process.kill("TERM", pid)
    Process.wait(pid)
  end
end
