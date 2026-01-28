class Ticktick < Formula
  desc "CLI for TickTick task management"
  homepage "https://github.com/karbassi/ticktick-cli"
  version "0.1.0"
  license "MIT"

  on_macos do
    on_arm do
      url "https://github.com/karbassi/ticktick-cli/releases/download/v#{version}/ticktick-macos-arm64.tar.gz"
      sha256 "REPLACE_WITH_SHA256"
    end
    on_intel do
      url "https://github.com/karbassi/ticktick-cli/releases/download/v#{version}/ticktick-macos-x86_64.tar.gz"
      sha256 "REPLACE_WITH_SHA256"
    end
  end

  on_linux do
    on_arm do
      url "https://github.com/karbassi/ticktick-cli/releases/download/v#{version}/ticktick-linux-arm64.tar.gz"
      sha256 "REPLACE_WITH_SHA256"
    end
    on_intel do
      url "https://github.com/karbassi/ticktick-cli/releases/download/v#{version}/ticktick-linux-x86_64.tar.gz"
      sha256 "REPLACE_WITH_SHA256"
    end
  end

  def install
    bin.install "ticktick"
    generate_completions_from_executable(bin/"ticktick", shell_parameter_format: :clap)
  end

  test do
    assert_match "ticktick", shell_output("#{bin}/ticktick --version")
  end
end
